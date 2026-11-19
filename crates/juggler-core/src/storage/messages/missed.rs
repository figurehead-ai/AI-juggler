//! D-119 item 6 (2026-11-16): a pool-wide message is copied into the
//! inboxes of the members open when it's sent; a member resumed later
//! catches up from the pool's own log. The reads and the one write that
//! takes.

use rusqlite::{OptionalExtension, params};

use super::{
    MESSAGE_COLUMNS, Message, MessageEvent, MessagesRepo, RecipientState, collect, message_of,
    page, refs_json,
};
use crate::error::CoreError;
use crate::model::Timestamp;

/// A pool's pool-wide messages from a time on (`?2`) that none of `?3`'s
/// refs sent or holds a copy of (in any state), and that the person
/// didn't refuse as a whole (D-28: held at the pool's level, then Deny).
const MISSED: &str = "FROM agent_messages m
  WHERE m.pool_id = ?1 AND m.to_all = 1 AND m.created_at >= ?2
    AND m.sender_ref NOT IN (SELECT value FROM json_each(?3))
    AND NOT EXISTS (SELECT 1 FROM agent_recipients r
                     WHERE r.message_id = m.id
                       AND r.recipient_ref IN (SELECT value FROM json_each(?3)))
    AND NOT EXISTS (SELECT 1 FROM agent_asks a
                     WHERE a.object_ref = m.id AND a.action = 'post_message'
                       AND a.answer IN ('deny', 'pool_deleted'))";

/// A pool-wide message a member missed, and whether the person's OK for
/// the whole of it is still open (held at the pool's level: its copy
/// waits with the others').
#[derive(Clone, Debug)]
pub struct Missed {
    /// The message.
    pub message: Message,
    /// Held for the person's OK, still open.
    pub awaits_ok: bool,
}

impl MessagesRepo<'_> {
    /// The pool-wide messages of `pool_id` sent from `since` on (epoch ms)
    /// that none of `refs` sent or holds a copy of, and that the person
    /// didn't refuse: newest first, at most `limit`; and how many there
    /// are in all.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn missed_broadcasts(
        &self,
        pool_id: &str,
        since: i64,
        refs: &[String],
        limit: u32,
    ) -> Result<(Vec<Missed>, usize), CoreError> {
        let refs = refs_json(refs);
        let total: i64 = self.conn.query_row(
            &format!("SELECT COUNT(*) {MISSED}"),
            params![pool_id, since, refs],
            |row| row.get(0),
        )?;
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {MESSAGE_COLUMNS},
                    EXISTS (SELECT 1 FROM agent_asks a
                             WHERE a.object_ref = m.id AND a.action = 'post_message'
                               AND a.answered_at IS NULL)
             {MISSED} ORDER BY m.seq DESC LIMIT ?4"
        ))?;
        let rows = collect(
            stmt.query_map(params![pool_id, since, refs, page(limit)], |row| {
                Ok(Missed {
                    message: message_of(row)?,
                    awaits_ok: row.get(16)?,
                })
            })?,
        )?;
        Ok((rows, usize::try_from(total).unwrap_or(0)))
    }

    /// Puts copies into one recipient's inbox (`recipient`: its ref and
    /// name), each in its state; a message it already has a copy of keeps
    /// it as it is. How many went in.
    ///
    /// # Errors
    ///
    /// Storage failures (nothing goes in).
    pub fn add_copies(
        &self,
        recipient: (&str, &str),
        copies: &[(String, RecipientState)],
    ) -> Result<usize, CoreError> {
        let now = Timestamp::now().millis();
        let tx = crate::storage::write_transaction(self.conn)?;
        let mut added = 0;
        for (id, state) in copies {
            added += tx.execute(
                "INSERT OR IGNORE INTO agent_recipients (message_id, recipient_ref, recipient_name, state, state_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, recipient.0, recipient.1, state.as_str(), now],
            )?;
        }
        tx.commit()?;
        Ok(added)
    }

    /// The newest event `actor` logged with `verb` in `pool_id`.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn last_event(
        &self,
        pool_id: &str,
        actor: &str,
        verb: &str,
    ) -> Result<Option<MessageEvent>, CoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, at, pool_id, actor_ref, verb, object_ref, detail FROM agent_events
                  WHERE pool_id = ?1 AND actor_ref = ?2 AND verb = ?3
                  ORDER BY at DESC, id DESC LIMIT 1",
                params![pool_id, actor, verb],
                |row| {
                    Ok(MessageEvent {
                        id: row.get(0)?,
                        at: row.get(1)?,
                        pool_id: row.get(2)?,
                        actor_ref: row.get(3)?,
                        verb: row.get(4)?,
                        object_ref: row.get(5)?,
                        detail: row.get(6)?,
                    })
                },
            )
            .optional()?)
    }
}

#[cfg(test)]
mod tests {
    use crate::storage::{NewMessage, RecipientState, Storage};

    fn broadcast(storage: &Storage, pool: &str, id: &str, from: &str, to: &[&str]) {
        let repo = storage.messages();
        if repo.thread("thr").unwrap().is_none() {
            repo.create_thread("thr", pool, "News", from).unwrap();
        }
        let rows: Vec<(String, String, RecipientState)> = to
            .iter()
            .map(|r| ((*r).to_owned(), (*r).to_owned(), RecipientState::Unread))
            .collect();
        repo.insert_message(&NewMessage {
            id,
            thread_id: "thr",
            pool_id: pool,
            sender_ref: from,
            sender_kind: "agent",
            sender_name: "Planner",
            kind: "note",
            subject: "News",
            body: id,
            body_sha256: id,
            hop: 1,
            reply_to: None,
            to_all: true,
            redacted: 0,
            recipients: &rows,
        })
        .unwrap();
    }

    /// What a member missed: the pool-wide messages it holds no copy of
    /// and didn't send, newest first, counted in all; a copy put in once
    /// is never put in twice.
    #[test]
    fn a_member_misses_only_the_pool_wide_messages_it_has_no_copy_of() {
        let storage = Storage::open_in_memory().unwrap();
        let pool = storage.pools().create("Team", None).unwrap().id;
        broadcast(&storage, &pool, "m1", "session:a", &["session:b"]);
        broadcast(&storage, &pool, "m2", "session:a", &[]);
        broadcast(&storage, &pool, "m3", "session:b", &[]);
        broadcast(&storage, &pool, "m4", "session:a", &[]);
        let repo = storage.messages();
        let refs = vec!["session:b".to_owned(), "work:w".to_owned()];
        let (missed, total) = repo.missed_broadcasts(&pool, 0, &refs, 1).unwrap();
        assert_eq!(total, 2, "m2 and m4: m1 is held, m3 is its own");
        assert_eq!(missed.len(), 1);
        assert_eq!(missed[0].message.id, "m4", "newest first");
        assert!(!missed[0].awaits_ok);
        let copies = vec![
            ("m4".to_owned(), RecipientState::Unread),
            ("m1".to_owned(), RecipientState::Unread),
        ];
        assert_eq!(repo.add_copies(("session:b", "B"), &copies).unwrap(), 1);
        let (missed, total) = repo.missed_broadcasts(&pool, 0, &refs, 50).unwrap();
        assert_eq!(total, 1);
        assert_eq!(missed[0].message.id, "m2");
        let later = repo.message("m2").unwrap().unwrap().created_at;
        assert_eq!(
            repo.missed_broadcasts(&pool, later, &refs, 50).unwrap().1,
            1
        );
        assert_eq!(
            repo.missed_broadcasts(&pool, later + 1, &refs, 50)
                .unwrap()
                .1,
            0
        );
        repo.log(&pool, "session:b", "caught_up", "x", Some("1"))
            .unwrap();
        let event = repo
            .last_event(&pool, "session:b", "caught_up")
            .unwrap()
            .unwrap();
        assert_eq!(event.detail.as_deref(), Some("1"));
        assert!(
            repo.last_event(&pool, "session:a", "caught_up")
                .unwrap()
                .is_none()
        );
    }
}
