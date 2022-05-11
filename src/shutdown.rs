use tokio::sync::broadcast::{Receiver, Sender};

/// Listens for the server shutdown signal.
///
/// Shutdown is signalled using a `Receiver`. Only a single value is
/// ever sent. Once a value has been sent via the broadcast channel, the server
/// should shutdown.
///
/// The `Shutdown` struct listens for the signal and tracks that the signal has
/// been received. Callers may query for whether the shutdown signal has been
/// received or not.
#[derive(Debug)]
pub struct Shutdown {
    /// `true` if the shutdown signal has been received
    shutdown: bool,

    sender: Sender<()>,
    /// The receive half of the channel used to listen for shutdown.
    pub notify: Receiver<()>,
}

impl Shutdown {
    /// Create a new `Shutdown` backed by the given `Receiver`.
    pub(crate) fn new(sender: Sender<()>, notify: Receiver<()>) -> Shutdown {
        Shutdown {
            shutdown: false,
            sender,
            notify,
        }
    }

    /// Returns `true` if the shutdown signal has been received.
    pub(crate) fn is_shutdown(&self) -> bool {
        self.shutdown
    }

    /// Returns new Receiver for shutdown broadcaster.
    pub(crate) fn subscribe(&self) -> Receiver<()> {
        self.sender.subscribe()
    }

    /// Receive the shutdown notice, waiting if necessary.
    pub(crate) async fn recv(&mut self) {
        // If the shutdown signal has already been received, then return
        // immediately.
        if self.shutdown {
            return;
        }

        // Cannot receive a "lag error" as only one value is ever sent.
        let _ = self.notify.recv().await;

        // Remember that the signal has been received.
        self.shutdown = true;
    }
}
