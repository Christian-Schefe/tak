use crate::tak::{
    Player, TakAction, TakCoord, TakGame, TakGameAPI, TakGameState, TakInvalidAction, TakTower,
};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct TimedTakGame {
    game: TakGame,
    time_mode: TimeMode,
    pub time_left: [Duration; 2],
    last_action_time: CrossPlatformInstant,
}

#[derive(Clone, Debug)]
pub struct TimeMode {
    pub time_limit: Duration,
    pub time_increment: Duration,
}

impl TimeMode {
    pub fn new(time_limit: Duration, time_increment: Duration) -> Self {
        Self {
            time_limit,
            time_increment,
        }
    }
}

impl TimedTakGame {
    pub fn get_time_remaining(&self, player: Player) -> Duration {
        let time_left = match player {
            Player::White => self.time_left[0],
            Player::Black => self.time_left[1],
        };
        if player != self.game.current_player {
            return time_left;
        }
        let now = CrossPlatformInstant::now();
        let elapsed = now.elapsed_since(self.last_action_time);
        time_left.saturating_sub(Duration::from_millis(elapsed))
    }
}

impl TakGameAPI for TimedTakGame {
    type Settings = TimeMode;

    fn try_do_action(&mut self, action: TakAction) -> Result<TakGameState, TakInvalidAction> {
        let current_player = self.game.current_player;
        self.game.try_do_action(action)?;
        let now = CrossPlatformInstant::now();
        let elapsed = now.elapsed_since(self.last_action_time);
        self.last_action_time = now;
        let time_left = match current_player {
            Player::White => &mut self.time_left[0],
            Player::Black => &mut self.time_left[1],
        };
        *time_left = time_left.saturating_sub(Duration::from_millis(elapsed));
        if time_left.is_zero() {
            self.game.game_state = TakGameState::Win(current_player.opponent());
        } else {
            *time_left += self.time_mode.time_increment;
        }
        Ok(self.game.game_state)
    }

    fn new_game(size: usize, settings: Self::Settings) -> Self {
        let game = TakGame::new(size);
        let start_time = CrossPlatformInstant::now();
        let time_mode = settings;
        Self {
            game,
            time_left: [time_mode.time_limit, time_mode.time_limit],
            last_action_time: start_time,
            time_mode,
        }
    }

    fn current_player(&self) -> Player {
        self.game.current_player
    }

    fn size(&self) -> usize {
        self.game.size
    }

    fn get_actions(&self) -> &Vec<TakAction> {
        self.game.get_actions()
    }

    fn try_get_tower(&self, pos: TakCoord) -> Option<&TakTower> {
        self.game.try_get_tower(pos)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
