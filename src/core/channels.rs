use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc::Sender, Mutex};
use uuid::Uuid;

use super::task::InputData;

#[derive(Default, Clone)]
pub struct ChannelsManager {
    pub clicked_channels: Arc<Mutex<HashMap<Uuid, Sender<()>>>>,
    pub triggered_channels: Arc<Mutex<HashMap<Uuid, Sender<InputData>>>>,
}

impl ChannelsManager {
    pub async fn add_clicked(&self, uuid: Uuid, s: Sender<()>) {
        self.clicked_channels.lock().await.insert(uuid, s);
    }

    pub async fn add_triggered(&self, uuid: Uuid, s: Sender<InputData>) {
        self.triggered_channels.lock().await.insert(uuid, s);
    }
}
