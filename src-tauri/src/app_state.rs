use crate::{
    worker_client::WorkerConfig,
    redirect_queue::KvRedirectQueue,
    notes::ParsedNotes,
};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::sync::RwLock;

pub struct AppStateInner {
    pub worker: RwLock<Option<WorkerConfig>>,
    pub generation: AtomicU64,
    pub redirect_queue: KvRedirectQueue,
    pub current_notes: RwLock<Option<ParsedNotes>>,
    pub media_scheduled_generation: AtomicU64,
    pub listener_cancel: RwLock<Option<tokio::sync::oneshot::Sender<()>>>,
}

pub type AppState = Arc<AppStateInner>;

impl Default for AppStateInner {
    fn default() -> Self {
        Self {
            worker: RwLock::new(None),
            generation: AtomicU64::new(0),
            redirect_queue: KvRedirectQueue::new(1000),
            current_notes: RwLock::new(None),
            media_scheduled_generation: AtomicU64::new(0),
            listener_cancel: RwLock::new(None),
        }
    }
}

impl AppStateInner {
    pub fn next_generation(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn current_generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }
}