use std::cmp::Eq;
use std::collections::HashMap;
use std::hash::Hash;

// Persistance is a trait for storing info about the current state of tracked data.
pub trait Persistance<K, V> {
    fn write(&mut self, key: K, value: V) -> Result<(), String>;
    fn read(&self, key: K) -> Option<&V>;
}

#[derive(Clone)]
// InMemoryPersistance implements Persistance for in memory hash map.
pub struct InMemoryPersistance<K, V> {
    data: HashMap<K, V>,
}

impl<K, V> InMemoryPersistance<K, V> {
    pub fn new() -> Self {
        InMemoryPersistance {
            data: HashMap::new(),
        }
    }
}

// default implementaion for InMemoryPersistance.
impl<K, V> Default for InMemoryPersistance<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> Persistance<K, V> for InMemoryPersistance<K, V>
where
    K: Eq + Hash,
{
    fn write(&mut self, key: K, value: V) -> Result<(), String> {
        self.data.insert(key, value);
        Ok(())
    }
    fn read(&self, key: K) -> Option<&V> {
        self.data.get(&key)
    }
}
