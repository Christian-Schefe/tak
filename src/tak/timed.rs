use crate::tak::ptn::{Ptn, PtnAttribute};
use crate::tak::{
    TakAction, TakCoord, TakGame, TakGameState, TakInvalidAction, TakPlayer, TakTower,
};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct TimedTakGame {
    game: TakGame,
    time_mode: TimeMode,
    pub time_left: [Duration; 2],
    last_action_time: Option<CrossPlatformInstant>,
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
    pub fn get_time_remaining(&self, player: TakPlayer) -> Duration {
        let time_left = match player {
            TakPlayer::White => self.time_left[0],
            TakPlayer::Black => self.time_left[1],
        };
        if player != self.game.current_player {
            return time_left;
        }
        let now = CrossPlatformInstant::now();
        let elapsed = self
            .last_action_time
            .map(|t| now.elapsed_since(t))
            .unwrap_or(0);
        time_left.saturating_sub(Duration::from_millis(elapsed))
    }

    pub fn try_do_action(&mut self, action: TakAction) -> Result<TakGameState, TakInvalidAction> {
        let current_player = self.game.current_player;
        self.game.try_do_action(action)?;
        let now = CrossPlatformInstant::now();
        let elapsed = self
            .last_action_time
            .map(|t| now.elapsed_since(t))
            .unwrap_or(0);
        self.last_action_time = Some(now);
        let time_left = match current_player {
            TakPlayer::White => &mut self.time_left[0],
            TakPlayer::Black => &mut self.time_left[1],
        };
        *time_left = time_left.saturating_sub(Duration::from_millis(elapsed));
        if time_left.is_zero() {
            self.game.game_state = TakGameState::Win(current_player.opponent());
        } else {
            *time_left += self.time_mode.time_increment;
        }
        Ok(self.game.game_state)
    }

    pub fn new_game(size: usize, settings: TimeMode) -> Self {
        let game = TakGame::new(size);
        let time_mode = settings;
        Self {
            game,
            time_left: [time_mode.time_limit, time_mode.time_limit],
            last_action_time: None,
            time_mode,
        }
    }

    pub fn current_player(&self) -> TakPlayer {
        self.game.current_player
    }

    pub fn size(&self) -> usize {
        self.game.size
    }

    pub fn get_actions(&self) -> &Vec<TakAction> {
        self.game.get_actions()
    }

    pub fn try_get_tower(&self, pos: TakCoord) -> Option<&TakTower> {
        self.game.try_get_tower(pos)
    }

    pub fn update_from_ptn(&mut self, ptn: Ptn) -> Option<()> {
        self.game.update_from_ptn(ptn)
    }

    pub fn to_ptn(&self) -> Ptn {
        let mut ptn = self.game.to_ptn();
        ptn.attributes.push(PtnAttribute::Clock(
            self.time_mode.time_limit,
            self.time_mode.time_increment,
        ));
        ptn
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
