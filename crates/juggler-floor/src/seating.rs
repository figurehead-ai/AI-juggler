//! Who sits where (SPEC §9.7, AJ-E3503 req. 4): a session keeps its desk
//! while its App's area keeps its size. A new session takes the lowest
//! free desk; one that ends frees its desk; when the area changes size,
//! everyone is seated again in order.
//!
//! The render thread keeps one [`Seating`] for as long as the Floor is
//! open; nothing here is saved (a reopened Floor seats in order).

use std::collections::BTreeMap;

/// Each App's desks: its capacity and who sits at which desk.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Seating {
    apps: BTreeMap<String, (usize, BTreeMap<String, usize>)>,
}

impl Seating {
    /// Seats `sessions` (the App's live sessions, by the rail's key, in
    /// start order) at an area of `capacity` desks: each desk's sitter,
    /// `None` for an empty desk. Sessions beyond the capacity aren't
    /// seated (the area grows on the next reflow).
    pub fn seat(&mut self, app: &str, capacity: usize, sessions: &[&str]) -> Vec<Option<String>> {
        let entry = self.apps.entry(app.to_owned()).or_default();
        if entry.0 != capacity {
            *entry = (capacity, BTreeMap::new());
        }
        let desks = &mut entry.1;
        desks.retain(|key, _| sessions.contains(&key.as_str()));
        let mut taken: Vec<bool> = vec![false; capacity];
        for desk in desks.values() {
            if let Some(slot) = taken.get_mut(*desk) {
                *slot = true;
            }
        }
        for key in sessions {
            if desks.contains_key(*key) {
                continue;
            }
            if let Some(free) = taken.iter().position(|t| !t) {
                taken[free] = true;
                desks.insert((*key).to_owned(), free);
            }
        }
        let mut out = vec![None; capacity];
        for (key, desk) in desks.iter() {
            if let Some(slot) = out.get_mut(*desk) {
                *slot = Some(key.clone());
            }
        }
        out
    }

    /// Forgets Apps no longer on the Floor.
    pub fn keep_only(&mut self, apps: &[&str]) {
        self.apps.retain(|id, _| apps.contains(&id.as_str()));
    }
}

#[cfg(test)]
mod tests {
    use super::Seating;

    fn at(desks: &[Option<String>], key: &str) -> Option<usize> {
        desks.iter().position(|d| d.as_deref() == Some(key))
    }

    #[test]
    fn a_session_keeps_its_desk_until_the_area_changes_size() {
        let mut seating = Seating::default();
        let first = seating.seat("app", 6, &["a", "b", "c"]);
        assert_eq!(
            (at(&first, "a"), at(&first, "b"), at(&first, "c")),
            (Some(0), Some(1), Some(2))
        );
        // b ends: a and c stay put; d takes b's desk.
        let next = seating.seat("app", 6, &["a", "c", "d"]);
        assert_eq!(
            (at(&next, "a"), at(&next, "c"), at(&next, "d")),
            (Some(0), Some(2), Some(1))
        );
        // The area grows: everyone is seated again, in order.
        let grown = seating.seat("app", 12, &["a", "c", "d"]);
        assert_eq!(
            (at(&grown, "a"), at(&grown, "c"), at(&grown, "d")),
            (Some(0), Some(1), Some(2))
        );
        assert_eq!(grown.len(), 12);
    }

    #[test]
    fn a_full_area_seats_whom_it_can_and_apps_are_forgotten() {
        let mut seating = Seating::default();
        let desks = seating.seat("app", 2, &["a", "b", "c"]);
        assert_eq!(desks.iter().flatten().count(), 2);
        seating.keep_only(&[]);
        assert_eq!(seating, Seating::default());
    }
}
