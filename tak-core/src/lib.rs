mod board;
mod coord;
mod game;
mod time;

pub use board::*;
pub use coord::*;
pub use game::*;
pub use time::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TakPieceVariant {
    Flat,
    Wall,
    Capstone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TakPlayer {
    White,
    Black,
}

impl TakPlayer {
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
pub enum TakInvalidPlaceError {
    NotAllowed,
    InvalidPosition,
    PositionOccupied,
    NotEnoughStones,
    InvalidVariant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakInvalidUndoPlaceError {
    NotAllowed,
    InvalidPosition,
    PositionEmpty,
    ActionMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakInvalidUndoMoveError {
    NotAllowed,
    InvalidPosition,
    InvalidTakeCount,
    InvalidDropCount,
    ActionMismatch,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TakInvalidActionError {
    InvalidPlace(TakInvalidPlaceError),
    InvalidMove(TakInvalidMoveError),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TakInvalidUndoActionError {
    InvalidPlace(TakInvalidUndoPlaceError),
    InvalidMove(TakInvalidUndoMoveError),
    NoLastAction,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TakWinReason {
    Road(TakCoord, TakCoord),
    Flat,
    Timeout,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TakGameState {
    Ongoing,
    Win(TakPlayer, TakWinReason),
    Draw,
}
