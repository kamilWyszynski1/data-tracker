use std::collections::HashMap;
use uuid::Uuid;

// Persistance is a trait for storing info about the current state of tracked data.
pub trait Persistance {
    fn write(&mut self, key: Uuid, value: u32) -> Result<(), String>;
    fn read(&self, key: &Uuid) -> Option<&u32>;
}

#[derive(Clone)]
// InMemoryPersistance implements Persistance for in memory hash map.
pub struct InMemoryPersistance {
    data: HashMap<Uuid, u32>,
}

impl InMemoryPersistance {
    pub fn new() -> Self {
        InMemoryPersistance {
            data: HashMap::new(),
        }
    }
}

// default implementation for InMemoryPersistance.
impl Default for InMemoryPersistance {
    fn default() -> Self {
        Self::new()
    }
}

impl Persistance for InMemoryPersistance {
    fn write(&mut self, key: Uuid, value: u32) -> Result<(), String> {
        println!("writing: {}{}", key, value);
        self.data.insert(key, value);
        Ok(())
    }
    fn read(&self, key: &Uuid) -> Option<&u32> {
        for (key, value) in (&self.data).into_iter() {
            println!("{} / {}", key, value);
        }
        self.data.get(&key)
    }
}

struct A {
    hm: HashMap<i32, i32>,
}

impl A {
    fn default() -> Self {
        A { hm: HashMap::new() }
    }
    fn write(&mut self, i: i32) {
        self.hm.insert(i, i);
    }
}
