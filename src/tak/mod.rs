pub mod action;
pub mod ptn;
pub mod timed;

use crate::tak::action::{TakAction, TakActionResult};
use crate::tak::ptn::Ptn;
use serde::{Deserialize, Serialize};
pub use timed::*;

#[derive(Clone, Debug, PartialEq)]
pub struct TakKomi {
    pub whole: usize,
    pub half: bool,
}

impl TakKomi {
    pub fn new(whole: usize, half: bool) -> Self {
        TakKomi { whole, half }
    }

    pub fn determine_winner(&self, white_score: usize, black_score: usize) -> Option<TakPlayer> {
        if white_score > black_score + self.whole {
            Some(TakPlayer::White)
        } else if white_score < black_score + self.whole {
            Some(TakPlayer::Black)
        } else if self.half {
            Some(TakPlayer::Black)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TakGame {
    pub size: usize,
    pub komi: TakKomi,
    pub board: Vec<TakTile>,
    pub current_player: TakPlayer,
    pub actions: Vec<TakActionResult>,
    pub hands: [TakHand; 2],
    id_counter: usize,
    game_state: TakGameState,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TakWinReason {
    Road,
    FlatCount,
    Timeout,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TakGameState {
    Ongoing,
    Win(TakPlayer, TakWinReason),
    Draw,
}

pub type TakResult<T> = Result<T, TakInvalidAction>;
pub type TakFeedback = Result<(), TakInvalidAction>;

#[derive(Clone, Debug, PartialEq)]
pub enum TakInvalidAction {
    NoRemainingStones,
    NoRemainingCapstones,
    InvalidPosition,
    TileOccupied,
    TileEmpty,
    NotYourPiece,
    InvalidAction,
    GameOver,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TakHand {
    pub stones: usize,
    pub capstones: usize,
}

impl TakHand {
    pub fn new(size: usize) -> Self {
        TakHand {
            stones: match size {
                3 => 10,
                4 => 15,
                5 => 21,
                6 => 30,
                7 => 40,
                8 => 50,
                _ => panic!("Invalid Tak board size"),
            },
            capstones: match size {
                3 => 0,
                4 => 0,
                5 => 1,
                6 => 1,
                7 => 2,
                8 => 2,
                _ => panic!("Invalid Tak board size"),
            },
        }
    }

    pub fn try_take_stone(&mut self) -> TakFeedback {
        if self.stones > 0 {
            self.stones -= 1;
            Ok(())
        } else {
            Err(TakInvalidAction::NoRemainingStones)
        }
    }

    pub fn try_take_capstone(&mut self) -> TakFeedback {
        if self.capstones > 0 {
            self.capstones -= 1;
            Ok(())
        } else {
            Err(TakInvalidAction::NoRemainingCapstones)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TakCoord {
    pub x: usize,
    pub y: usize,
}

impl TakCoord {
    pub fn new(x: usize, y: usize) -> Self {
        TakCoord { x, y }
    }

    pub fn validate(&self, size: usize) -> TakFeedback {
        if self.x < size && self.y < size {
            Ok(())
        } else {
            Err(TakInvalidAction::InvalidPosition)
        }
    }

    pub fn try_get_positions(
        &self,
        direction: &Direction,
        times: usize,
        size: usize,
    ) -> Option<Vec<TakCoord>> {
        match direction {
            Direction::Left => {
                if self.x >= times {
                    Some(
                        (1..=times)
                            .map(|i| TakCoord::new(self.x - i, self.y))
                            .collect(),
                    )
                } else {
                    None
                }
            }
            Direction::Right => {
                if self.x + times < size {
                    Some(
                        (1..=times)
                            .map(|i| TakCoord::new(self.x + i, self.y))
                            .collect(),
                    )
                } else {
                    None
                }
            }
            Direction::Down => {
                if self.y >= times {
                    Some(
                        (1..=times)
                            .map(|i| TakCoord::new(self.x, self.y - i))
                            .collect(),
                    )
                } else {
                    None
                }
            }
            Direction::Up => {
                if self.y + times < size {
                    Some(
                        (1..=times)
                            .map(|i| TakCoord::new(self.x, self.y + i))
                            .collect(),
                    )
                } else {
                    None
                }
            }
        }
    }

    pub fn offset_by(&self, direction: &Direction, times: usize) -> Option<TakCoord> {
        match direction {
            Direction::Left => {
                if self.x >= times {
                    Some(TakCoord::new(self.x.saturating_sub(times), self.y))
                } else {
                    None
                }
            }
            Direction::Right => Some(TakCoord::new(self.x + times, self.y)),
            Direction::Down => {
                if self.y >= times {
                    Some(TakCoord::new(self.x, self.y.saturating_sub(times)))
                } else {
                    None
                }
            }
            Direction::Up => Some(TakCoord::new(self.x, self.y + times)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IDStone {
    pub id: usize,
    pub player: TakPlayer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TakPlayer {
    White,
    Black,
}

impl TakPlayer {
    pub fn opponent(&self) -> TakPlayer {
        match self {
            TakPlayer::White => TakPlayer::Black,
            TakPlayer::Black => TakPlayer::White,
        }
    }
    pub fn all() -> Vec<TakPlayer> {
        vec![TakPlayer::White, TakPlayer::Black]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TakPieceType {
    Flat,
    Wall,
    Capstone,
}

pub type TakTile = Option<TakTower>;

#[derive(Clone, Debug, PartialEq)]
pub struct TakTower {
    pub top_type: TakPieceType,
    pub composition: Vec<IDStone>,
}

impl TakTower {
    pub fn controlling_player(&self) -> TakPlayer {
        self.composition[self.composition.len() - 1].player
    }

    pub fn height(&self) -> usize {
        self.composition.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn try_from_diff(a: &TakCoord, b: &TakCoord) -> Option<Direction> {
        if a.x == b.x {
            if a.y + 1 == b.y {
                Some(Direction::Up)
            } else if b.y + 1 == a.y {
                Some(Direction::Down)
            } else {
                None
            }
        } else if a.y == b.y {
            if a.x + 1 == b.x {
                Some(Direction::Right)
            } else if b.x + 1 == a.x {
                Some(Direction::Left)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn all() -> Vec<Direction> {
        vec![
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ]
    }
}

impl TakGame {
    pub fn new(size: usize) -> Self {
        TakGame {
            size,
            komi: TakKomi::new(2, false),
            board: vec![None; size * size],
            current_player: TakPlayer::White,
            actions: Vec::new(),
            hands: [TakHand::new(size), TakHand::new(size)],
            id_counter: 0,
            game_state: TakGameState::Ongoing,
        }
    }

    pub fn try_do_action(&mut self, action: TakAction) -> TakResult<TakActionResult> {
        if self.game_state != TakGameState::Ongoing {
            return Err(TakInvalidAction::GameOver);
        }
        let action_result = match action {
            TakAction::PlacePiece {
                position,
                piece_type,
            } => self.try_place_piece(position, piece_type),
            TakAction::MovePiece {
                from,
                direction,
                take,
                drops,
            } => self.try_move_piece(from, direction, take, drops),
        }?;
        self.actions.push(action_result.clone());
        self.check_game_over();
        self.current_player = match self.current_player {
            TakPlayer::White => TakPlayer::Black,
            TakPlayer::Black => TakPlayer::White,
        };
        Ok(action_result)
    }

    pub fn try_get_tower(&self, pos: TakCoord) -> Option<&TakTower> {
        self.try_get_tile(&pos).ok()?.as_ref()
    }

    pub fn update_from_ptn(&mut self, ptn: Ptn) -> Option<()> {
        let actions = ptn
            .turns
            .iter()
            .flat_map(|actions| actions.iter().map(|x| TakAction::from_ptn(x)))
            .collect::<Option<Vec<_>>>()?;
        let size = ptn.get_size()?;
        let mut game = Self::new(size);
        for action in actions {
            game.try_do_action(action).ok()?;
        }
        *self = game;
        Some(())
    }

    fn check_game_over(&mut self) {
        let player = self.current_player;
        let check_direction = |is_horizontal: bool| {
            let mut visited = vec![false; self.size * self.size];
            let mut stack = Vec::new();
            for i in 0..self.size {
                let pos = if is_horizontal {
                    TakCoord::new(0, i)
                } else {
                    TakCoord::new(i, 0)
                };
                if let Some(tower) = self.get_tile(&pos) {
                    if tower.controlling_player() == player && tower.top_type != TakPieceType::Wall
                    {
                        stack.push(pos);
                    }
                }
            }
            while let Some(pos) = stack.pop() {
                if visited[pos.y * self.size + pos.x] {
                    continue;
                }
                visited[pos.y * self.size + pos.x] = true;
                if (if is_horizontal { pos.x } else { pos.y }) == self.size - 1 {
                    return true;
                }
                for direction in Direction::all() {
                    if let Some(new_pos) = pos.offset_by(&direction, 1) {
                        if let Ok(tower) = self.try_get_tower_at(&new_pos) {
                            if tower.controlling_player() == player
                                && tower.top_type != TakPieceType::Wall
                            {
                                stack.push(new_pos);
                            }
                        }
                    }
                }
            }
            false
        };
        dioxus::logger::tracing::info!("Checking game over");
        if check_direction(true) || check_direction(false) {
            self.game_state = TakGameState::Win(player, TakWinReason::Road);
            dioxus::logger::tracing::info!("Game over by road: {:?}", self.game_state);
            return;
        }

        let hand = self.get_hand(player);
        let has_remaining_stones = hand.stones > 0 || hand.capstones > 0;

        dioxus::logger::tracing::info!("Counting");
        let mut flat_counts = [0, 0];
        for y in 0..self.size {
            for x in 0..self.size {
                let pos = TakCoord::new(x, y);
                if let Ok(tower) = self.try_get_tower_at(&pos) {
                    if tower.top_type == TakPieceType::Flat {
                        let i = match tower.controlling_player() {
                            TakPlayer::White => 0,
                            TakPlayer::Black => 1,
                        };
                        flat_counts[i] += 1;
                    }
                } else if has_remaining_stones {
                    dioxus::logger::tracing::info!(
                        "Square {:?} is empty and player has stones left",
                        pos
                    );
                    return;
                }
            }
        }
        if let Some(winner) = self.komi.determine_winner(flat_counts[0], flat_counts[1]) {
            self.game_state = TakGameState::Win(winner, TakWinReason::FlatCount);
        } else {
            self.game_state = TakGameState::Draw;
        }
        dioxus::logger::tracing::info!("Game over: {:?}", self.game_state);
    }

    fn get_hand_mut(&mut self, player: TakPlayer) -> &mut TakHand {
        match player {
            TakPlayer::White => &mut self.hands[0],
            TakPlayer::Black => &mut self.hands[1],
        }
    }

    fn get_hand(&self, player: TakPlayer) -> &TakHand {
        match player {
            TakPlayer::White => &self.hands[0],
            TakPlayer::Black => &self.hands[1],
        }
    }

    pub fn to_ptn(&self) -> Ptn {
        let turns = self
            .actions
            .chunks(2)
            .map(|actions| actions.iter().map(|x| x.to_ptn()).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let attributes = vec![ptn::PtnAttribute::Size(self.size)];
        Ptn { attributes, turns }
    }

    fn get_tile(&self, position: &TakCoord) -> &TakTile {
        position.validate(self.size).unwrap();
        &self.board[position.y * self.size + position.x]
    }

    fn get_tile_mut(&mut self, position: &TakCoord) -> &mut TakTile {
        position.validate(self.size).unwrap();
        &mut self.board[position.y * self.size + position.x]
    }

    fn try_get_tile(&self, position: &TakCoord) -> TakResult<&TakTile> {
        position.validate(self.size)?;
        Ok(&self.board[position.y * self.size + position.x])
    }

    fn try_get_tower_at(&self, position: &TakCoord) -> TakResult<&TakTower> {
        position.validate(self.size)?;
        self.board[position.y * self.size + position.x]
            .as_ref()
            .ok_or(TakInvalidAction::TileEmpty)
    }

    pub fn get_valid_place_options(&self, player: TakPlayer) -> Vec<TakPieceType> {
        if self.actions.len() < 2 {
            return vec![TakPieceType::Flat];
        }
        let hand = self.get_hand(player);
        let mut options = Vec::new();
        if hand.capstones > 0 {
            options.push(TakPieceType::Capstone);
        }
        if hand.stones > 0 {
            options.push(TakPieceType::Flat);
            options.push(TakPieceType::Wall);
        }
        options
    }

    fn try_place_piece(
        &mut self,
        position: TakCoord,
        piece_type: TakPieceType,
    ) -> TakResult<TakActionResult> {
        let player = if self.actions.len() >= 2 {
            self.current_player
        } else {
            if piece_type != TakPieceType::Flat {
                return Err(TakInvalidAction::InvalidAction);
            }
            self.current_player.opponent()
        };
        let tile = self.try_get_tile(&position)?;
        if let None = tile {
            if piece_type == TakPieceType::Capstone {
                self.get_hand_mut(player).try_take_capstone()?;
            } else {
                self.get_hand_mut(player).try_take_stone()?;
            }
            *self.get_tile_mut(&position) = Some(TakTower {
                top_type: piece_type,
                composition: vec![IDStone {
                    player,
                    id: self.id_counter,
                }],
            });
            self.id_counter += 1;
        } else {
            return Err(TakInvalidAction::TileOccupied);
        }
        Ok(TakActionResult::PlacePiece {
            position,
            piece_type,
        })
    }

    fn try_move_piece(
        &mut self,
        from: TakCoord,
        direction: Direction,
        take: usize,
        drops: Vec<usize>,
    ) -> TakResult<TakActionResult> {
        if self.actions.len() < 2 {
            return Err(TakInvalidAction::InvalidAction);
        }

        let from_tower = self.try_get_tower_at(&from)?;
        let from_top_type = from_tower.top_type;
        let from_composition_len = from_tower.composition.len();
        if from_tower.controlling_player() != self.current_player {
            return Err(TakInvalidAction::NotYourPiece);
        }

        let drop_len = drops.len();
        let drop_sum: usize = drops.iter().sum();
        if take > self.size
            || from_composition_len < take
            || take == 0
            || drop_sum != take
            || drops.iter().any(|&i| i < 1)
        {
            return Err(TakInvalidAction::InvalidAction);
        }
        let positions = from
            .try_get_positions(&direction, drop_len, self.size)
            .ok_or(TakInvalidAction::InvalidAction)?;
        let mut has_flattened = false;
        for i in 0..drop_len {
            if let Some(tower) = self.get_tile(&positions[i]) {
                if tower.top_type != TakPieceType::Flat {
                    let can_flatten = tower.top_type == TakPieceType::Wall
                        && from_top_type == TakPieceType::Capstone
                        && i == drop_len - 1
                        && drops[i] == 1;
                    if !can_flatten {
                        return Err(TakInvalidAction::InvalidAction);
                    }
                    has_flattened = true;
                }
            }
        }

        let taken = {
            let from_tower = self.get_tile_mut(&from);
            if from_composition_len == take {
                from_tower.take().unwrap().composition
            } else {
                let composition_offset = from_composition_len - take;
                let mut_tower = from_tower.as_mut().unwrap();
                mut_tower.top_type = TakPieceType::Flat;
                mut_tower.composition.split_off(composition_offset)
            }
        };

        let mut drop_index = 0;

        for i in 0..drop_len {
            let tile = self.get_tile_mut(&positions[i]);
            let added_slice = &taken[drop_index..drop_index + drops[i]];
            let new_top_type = if i == drop_len - 1 {
                from_top_type
            } else {
                TakPieceType::Flat
            };
            if let Some(tower) = tile {
                tower.composition.extend_from_slice(added_slice);
                tower.top_type = new_top_type;
            } else {
                *tile = Some(TakTower {
                    top_type: new_top_type,
                    composition: added_slice.to_vec(),
                });
            }
            drop_index += drops[i];
        }

        Ok(TakActionResult::MovePiece {
            from,
            direction,
            take,
            drops,
            flattened: has_flattened,
        })
    }
}
