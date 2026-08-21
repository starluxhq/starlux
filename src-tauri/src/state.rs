use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

#[derive(Default)]
pub struct AppState {
    blur_hide_suppressed: AtomicBool,
    /// The conversation the Quick Bar is on, so expanding lands on it.
    active_conversation: Mutex<Option<String>>,
}

impl AppState {
    pub fn set_blur_hide_suppressed(&self, suppressed: bool) {
        self.blur_hide_suppressed
            .store(suppressed, Ordering::Relaxed);
    }

    pub fn blur_hide_suppressed(&self) -> bool {
        self.blur_hide_suppressed.load(Ordering::Relaxed)
    }

    pub fn set_active_conversation(&self, id: String) {
        *self.active_conversation.lock().unwrap() = Some(id);
    }

    pub fn clear_active_conversation(&self) {
        *self.active_conversation.lock().unwrap() = None;
    }

    pub fn active_conversation(&self) -> Option<String> {
        self.active_conversation.lock().unwrap().clone()
    }
}
