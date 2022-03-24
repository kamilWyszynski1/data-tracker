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

    pub async fn save(&self, key: Uuid, value: u32) -> Result<(), &'static str> {
        self.shared.lock().await.write(key, value)
    }
}

// Persistance is a trait for storing info about the current state of tracked data.
pub trait Persistance {
    fn write(&mut self, key: Uuid, value: u32) -> Result<(), &'static str>;
    fn read(&self, key: &Uuid) -> Option<u32>;
}
