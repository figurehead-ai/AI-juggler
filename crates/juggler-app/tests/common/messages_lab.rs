//! E18: a pool of fake agents for the messaging tests. Sessions are rows
//! (as if their tool ran in a terminal) or the fake tool launched in its
//! driven runtime; each gets an agent-tools key the way the app mints one,
//! and calls go through the bridge's own entry point (`calls::handle`).
//! Nothing real is launched or read.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{FAKE_HARNESS_ID, FakeHarnessAdapter};
use juggler_agentbridge::wire::{self, Args, Call, Request, Response, SendArgs};
use juggler_app::commands;
use juggler_app::features::{agent_bridge, pools, runtime};
use juggler_app::state::AppState;
use juggler_core::model::{
    InstanceKind, InstanceRecord, MachineId, ProfileId, SessionId, SessionStatus, Timestamp,
};

/// A pool "Team" with fake agents in it.
pub struct Lab {
    _dir: tempfile::TempDir,
    pub state: Arc<AppState>,
    pub account: String,
    pub profile: String,
    pub cwd: PathBuf,
    pub pool: String,
}

impl Lab {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        // Short: a driven session's socket path must fit `sun_path`.
        let root = dir.path().to_path_buf();
        let mut registry = AdapterRegistry::new();
        registry.register(Arc::new(FakeHarnessAdapter::new(
            crate::common::fake_harness::bin(),
        )));
        let state = Arc::new(AppState::init_with_registry(root.join("a"), registry).unwrap());
        // Any program of root's stands in for juggler-agent: the calls go
        // through `calls::handle`, as the program's would.
        state
            .agent_bridge
            .set_program(Some(PathBuf::from("/bin/echo")));
        agent_bridge::settings::answer_impl(&state, true).unwrap();
        let cwd = root.join("work");
        std::fs::create_dir_all(&cwd).unwrap();
        let account = commands::create_account(&state, FAKE_HARNESS_ID, "Lab", None)
            .unwrap()
            .id;
        let profile = commands::create_profile(
            &state,
            &account,
            "lab",
            None,
            Some(cwd.to_string_lossy().into_owned()),
        )
        .unwrap()
        .id;
        let info = pools::pool_create_impl(&state, "Team", None).unwrap();
        let pool = info
            .pools
            .iter()
            .find(|pool| pool.name == "Team")
            .unwrap()
            .id
            .clone();
        Self {
            _dir: dir,
            state,
            account,
            profile,
            cwd,
            pool,
        }
    }

    /// Names a session and puts it in the pool; its agent-tools key.
    fn join(&self, session: &SessionId, name: &str) -> String {
        self.state
            .storage
            .lock()
            .unwrap()
            .sessions()
            .set_label(session, Some(name))
            .unwrap();
        pools::pool_move_impl(&self.state, session.as_str(), &self.pool).unwrap();
        self.key(session)
    }

    /// A fresh agent-tools key for `session`, minted as a launch mints it.
    pub fn key(&self, session: &SessionId) -> String {
        let adapter = FakeHarnessAdapter::new(crate::common::fake_harness::bin());
        let mut spec = juggler_adapter::launch::interactive_spec(
            PathBuf::from("/bin/true"),
            vec!["run".into()],
            std::collections::BTreeMap::new(),
            PathBuf::from("/"),
        );
        let prepared = agent_bridge::prepare(
            &self.state,
            &adapter,
            &mut spec,
            session,
            None,
            &self.account,
        );
        assert!(
            !matches!(prepared, agent_bridge::Prepared::Not(_)),
            "{prepared:?}"
        );
        spec.env.get(wire::TOKEN_ENV).cloned().unwrap()
    }

    /// An agent whose tool runs in a terminal (a row, no process).
    pub fn agent(&self, id: &str, name: &str) -> (SessionId, String) {
        let session = SessionId::new(id);
        self.state
            .storage
            .lock()
            .unwrap()
            .sessions()
            .insert(&InstanceRecord {
                id: session.clone(),
                conversation: None,
                profile: ProfileId::new(self.profile.as_str()),
                machine: MachineId::new("this-mac"),
                kind: InstanceKind::Terminal,
                project: None,
                cwd: self.cwd.clone(),
                status: SessionStatus::Running,
                started_at: Timestamp::now(),
                last_activity: Some(Timestamp::now()),
                usage: Default::default(),
                attention: None,
                running: None,
                transcript_ref: None,
                window: None,
            })
            .unwrap();
        let key = self.join(&session, name);
        (session, key)
    }

    /// An agent run in AI Juggler: the fake tool, driven.
    pub fn driven(&self, name: &str) -> (SessionId, String) {
        runtime::commands::choose_impl(&self.state, &self.profile, Some("driven")).unwrap();
        let launched = commands::launch_profile(&self.state, &self.profile, None).unwrap();
        let session = SessionId::new(launched.session_id.as_str());
        assert!(
            self.state.runtime.is_driven(&session),
            "{:?}",
            self.state.runtime.info(session.as_str())
        );
        wait_for("the driven session to be ready", || {
            juggler_app::features::messages::delivery::between_turns(&self.state, &session)
        });
        let key = self.join(&session, name);
        (session, key)
    }

    /// One call through the bridge's own entry point.
    pub fn call(&self, key: &str, call: Call, args: Args) -> Response {
        let request = Request::new(key.to_owned(), call, args);
        agent_bridge::calls::handle(&self.state, &serde_json::to_string(&request).unwrap())
    }

    /// `send_message`.
    pub fn send(&self, key: &str, send: SendArgs) -> Response {
        self.call(
            key,
            Call::SendMessage,
            Args {
                send: Some(send),
                ..Args::default()
            },
        )
    }

    /// `send_message` to one member, a new thread or in `thread`.
    pub fn say(&self, key: &str, to: &SessionId, body: &str, thread: Option<&str>) -> Response {
        self.send(
            key,
            SendArgs {
                to: to.to_string(),
                body: body.into(),
                thread: thread.map(str::to_owned),
                ..SendArgs::default()
            },
        )
    }

    /// `read_inbox` (unread only).
    pub fn inbox(&self, key: &str) -> Response {
        self.call(
            key,
            Call::ReadInbox,
            Args {
                inbox: Some(wire::InboxArgs::default()),
                ..Args::default()
            },
        )
    }

    /// The driven session's conversation lines (held in memory).
    pub fn lines(&self, session: &SessionId) -> Vec<String> {
        runtime::commands::get_impl(&self.state, session.as_str())
            .unwrap()
            .map(|view| view.items.into_iter().map(|item| item.text).collect())
            .unwrap_or_default()
    }

    /// Stops every session the lab launched.
    pub fn stop_all(&self) {
        // Also after a failed assertion poisoned the lock: reap anyway.
        let live: Vec<SessionId> = self
            .state
            .storage
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .sessions()
            .live()
            .unwrap_or_default()
            .into_iter()
            .map(|record| record.id)
            .collect();
        for session in live {
            if self.state.runtime.is_driven(&session) {
                let _ = self.state.supervisor.stop(&session, Duration::from_secs(2));
            }
        }
    }
}

impl Drop for Lab {
    fn drop(&mut self) {
        self.stop_all();
    }
}

pub fn wait_for(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// A result's field as text.
pub fn field(response: &Response, pointer: &str) -> String {
    response
        .result
        .as_ref()
        .and_then(|result| result.pointer(pointer))
        .map(|value| {
            value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned)
        })
        .unwrap_or_default()
}
