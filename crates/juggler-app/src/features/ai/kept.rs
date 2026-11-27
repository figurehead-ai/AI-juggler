//! What a job's model wrote for a turn, kept in memory so it is written
//! once (the turn review's lines, the feed's summary): a bounded map,
//! cleared whole when it fills (a turn asked again after that is written
//! again, never wrong).

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;

/// A bounded map of results by key.
pub struct Kept<K, V> {
    map: Mutex<HashMap<K, V>>,
    max: usize,
}

impl<K: Eq + Hash, V: Clone> Kept<K, V> {
    /// An empty map that keeps at most `max` results.
    #[must_use]
    pub fn new(max: usize) -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
            max,
        }
    }

    /// The result kept for `key`, if any.
    #[must_use]
    pub fn get(&self, key: &K) -> Option<V> {
        self.map.lock().ok().and_then(|map| map.get(key).cloned())
    }

    /// Keeps `value` for `key`, clearing the map first when it is full.
    pub fn keep(&self, key: K, value: V) {
        if let Ok(mut map) = self.map.lock() {
            if map.len() >= self.max {
                map.clear();
            }
            map.insert(key, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Kept;

    #[test]
    fn a_full_map_is_cleared_before_the_next_result_is_kept() {
        let kept = Kept::new(2);
        kept.keep("a", 1);
        kept.keep("b", 2);
        assert_eq!(kept.get(&"a"), Some(1));
        kept.keep("c", 3);
        assert_eq!(kept.get(&"a"), None);
        assert_eq!(kept.get(&"c"), Some(3));
    }
}
