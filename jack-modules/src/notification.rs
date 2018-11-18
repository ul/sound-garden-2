//! # Notification
//!
//! JACK notification handler which allows to block thread until client is shut down.
use crossbeam_channel::{bounded, Receiver, Sender};
use void::Void;

pub struct Notification {
    /// Dropping this sender would signal that client is shut down
    is_alive: Option<Sender<Void>>,
}

impl Notification {
    /// Create new notification handler and return receiver which blocks until client is shut down
    pub fn new() -> (Self, Receiver<Void>) {
        let (tx, rx) = bounded(0);
        (Notification { is_alive: Some(tx) }, rx)
    }
}

impl jack::NotificationHandler for Notification {
    fn shutdown(&mut self, _status: jack::ClientStatus, _reason: &str) {
        self.is_alive = None;
    }
}
