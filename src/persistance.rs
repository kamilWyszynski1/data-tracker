use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Tracker persistance state shared across all handlers.
pub struct Db<P: Persistance> {
    shared: Arc<Mutex<P>>,
}

impl<P> Db<P>
where
    P: Persistance,
{
    /// Create new `Db` instance with given persistance.
    pub fn new(p: P) -> Self {
        Db {
            shared: Arc::new(Mutex::new(p)),
        }
    }

    pub async fn get(&self, key: &Uuid) -> Option<u32> {
        self.shared.lock().await.read(key)
    }

    pub async fn save(&self, key: Uuid, value: u32) -> Result<(), String> {
        self.shared.lock().await.write(key, value)
    }
}

// Persistance is a trait for storing info about the current state of tracked data.
pub trait Persistance {
    fn write(&mut self, key: Uuid, value: u32) -> Result<(), String>;
    fn read(&self, key: &Uuid) -> Option<u32>;
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
        info!("writing: {}{}", key, value);
        self.data.insert(key, value);
        Ok(())
    }

    fn read(&self, key: &Uuid) -> Option<u32> {
        for (key, value) in (&self.data).iter() {
            info!("{} / {}", key, value);
        }
        self.data.get(key).copied()
    }
}
