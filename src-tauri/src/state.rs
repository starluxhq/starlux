use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
pub struct AppState {
    blur_hide_suppressed: AtomicBool,
}

impl AppState {
    pub fn set_blur_hide_suppressed(&self, suppressed: bool) {
        self.blur_hide_suppressed
            .store(suppressed, Ordering::Relaxed);
    }

    pub fn blur_hide_suppressed(&self) -> bool {
        self.blur_hide_suppressed.load(Ordering::Relaxed)
    }
}
