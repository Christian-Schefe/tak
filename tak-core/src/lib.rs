mod action;
mod board;
mod coord;
mod game;
mod movegen;
mod ptn;
mod time;
mod tps;
mod ui;

pub use action::*;
pub use board::*;
pub use coord::*;
pub use game::*;
pub use movegen::*;
pub use ptn::*;
pub use time::*;
pub use tps::*;
pub use ui::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, fixed_map::Key)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

pub enum TakPieceVariant {
    Flat,
    Wall,
    Capstone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, fixed_map::Key)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TakPlayer {
    White,
    Black,
}

impl TakPlayer {
    pub const ALL: [TakPlayer; 2] = [TakPlayer::White, TakPlayer::Black];
    pub fn other(&self) -> Self {
        match self {
            TakPlayer::White => TakPlayer::Black,
            TakPlayer::Black => TakPlayer::White,
        }
    }

    pub fn index(&self) -> usize {
        match self {
            TakPlayer::White => 0,
            TakPlayer::Black => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TakInvalidPlaceError {
    NotAllowed,
    InvalidPosition,
    PositionOccupied,
    NotEnoughStones,
    InvalidVariant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TakInvalidMoveError {
    NotAllowed,
    InvalidPosition,
    InvalidDirection,
    PositionEmpty,
    NotEnoughPieces,
    InvalidTakeCount,
    InvalidDropCount,
    Blocked,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TakInvalidActionError {
    InvalidPlace(TakInvalidPlaceError),
    InvalidMove(TakInvalidMoveError),
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TakWinReason {
    Road,
    Flat,
    Timeout,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TakGameState {
    Ongoing,
    Win(TakPlayer, TakWinReason),
    Draw,
    Canceled,
}
