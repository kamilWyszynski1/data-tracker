use super::interface::Persistance;
use std::collections::HashMap;
use uuid::Uuid;

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
    fn write(&mut self, key: Uuid, value: u32) -> Result<(), &'static str> {
        info!("writing: {}{}", key.to_simple(), value);
        self.data.insert(key, value);
        Ok(())
    }

    fn read(&self, key: &Uuid) -> Option<u32> {
        for (key, value) in (&self.data).iter() {
            info!("{} / {}", key.to_simple(), value);
        }
        self.data.get(key).copied()
    }
}
