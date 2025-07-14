use crate::TakPlayer;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TakTimeMode {
    pub time: usize,
    pub increment: usize,
}

impl TakTimeMode {
    /// Creates a new TakTimeMode with the given time in seconds and increment in seconds.
    pub fn new(time: usize, increment: usize) -> Self {
        TakTimeMode { time, increment }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TakTimestamp {
    pub millis: u64,
}

impl TakTimestamp {
    #[cfg(not(feature = "wasm"))]
    pub fn now() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        TakTimestamp { millis: now }
    }

    #[cfg(feature = "wasm")]
    pub fn now() -> Self {
        use web_sys::js_sys::Date;
        TakTimestamp {
            millis: Date::new_0().get_time() as u64,
        }
    }

    pub fn elapsed_since(&self, earlier: TakTimestamp) -> u64 {
        self.millis.saturating_sub(earlier.millis)
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TakClock {
    pub time_remaining_millis: [u64; 2],
    pub increment_millis: u64,
    pub last_update_timestamp: Option<TakTimestamp>,
}

impl TakClock {
    pub fn new(mode: &TakTimeMode) -> Self {
        let time_remaining = mode.time as u64 * 1000;
        TakClock {
            time_remaining_millis: [time_remaining, time_remaining],
            increment_millis: mode.increment as u64 * 1000,
            last_update_timestamp: None,
        }
    }

    pub fn update(&mut self, time: TakTimestamp, player: TakPlayer) {
        let elapsed = self
            .last_update_timestamp
            .map(|t| time.elapsed_since(t))
            .unwrap_or(0);
        self.last_update_timestamp = Some(time);
        let time_left = &mut self.time_remaining_millis[player.index()];
        *time_left = time_left.saturating_sub(elapsed);
        if *time_left > 0 {
            *time_left += self.increment_millis;
        }
    }

    pub fn get_time_remaining_at(&self, player: TakPlayer, now: TakTimestamp) -> u64 {
        let time_left = self.time_remaining_millis[player.index()];
        let elapsed = self
            .last_update_timestamp
            .map(|t| now.elapsed_since(t))
            .unwrap_or(0);
        time_left.saturating_sub(elapsed)
    }

    pub fn get_time_remaining(&self, player: TakPlayer, apply_elapsed: bool) -> u64 {
        let time_left = self.time_remaining_millis[player.index()];
        if !apply_elapsed {
            return time_left;
        }
        let now = TakTimestamp::now();
        let elapsed = self
            .last_update_timestamp
            .map(|t| now.elapsed_since(t))
            .unwrap_or(0);
        time_left.saturating_sub(elapsed)
    }

    pub fn set_time_remaining(&mut self, player: TakPlayer, time_remaining: u64) {
        self.time_remaining_millis[player.index()] = time_remaining;
    }
}
