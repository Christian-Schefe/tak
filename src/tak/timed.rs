use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TakTimeMode {
    pub time_limit: usize,
    pub time_increment: usize,
}

impl TakTimeMode {
    pub fn new(time_limit: usize, time_increment: usize) -> Self {
        Self {
            time_limit,
            time_increment,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CrossPlatformInstant {
    millis: u64,
}

impl CrossPlatformInstant {
    #[cfg(not(target_family = "wasm"))]
    pub fn now() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        CrossPlatformInstant { millis: now }
    }

    #[cfg(target_family = "wasm")]
    pub fn now() -> Self {
        use web_sys::js_sys::Date;
        CrossPlatformInstant {
            millis: Date::new_0().get_time() as u64,
        }
    }

    pub fn elapsed_since(&self, earlier: CrossPlatformInstant) -> u64 {
        self.millis.saturating_sub(earlier.millis)
    }
}
