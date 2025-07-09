use crate::{
    TakBoard, TakClock, TakCoord, TakDir, TakGameState, TakInvalidActionError, TakInvalidMoveError,
    TakInvalidPlaceError, TakInvalidUndoActionError, TakInvalidUndoMoveError,
    TakInvalidUndoPlaceError, TakPieceVariant, TakPlayer, TakTimeMode, TakTimestamp, TakWinReason,
};

#[derive(Debug, Clone, PartialEq)]
pub struct TakKomi {
    pub amount: usize,
    pub tiebreak: bool,
}

impl TakKomi {
    pub fn new(amount: usize, tiebreak: bool) -> Self {
        TakKomi { amount, tiebreak }
    }
    pub fn determine_winner(&self, counts: [usize; 2]) -> Option<TakPlayer> {
        let white_score = counts[0];
        let black_score = counts[1] + self.amount;
        if white_score > black_score {
            Some(TakPlayer::White)
        } else if black_score > white_score {
            Some(TakPlayer::Black)
        } else if self.tiebreak {
            Some(TakPlayer::Black)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TakStones {
    pub stones: usize,
    pub capstones: usize,
}

impl TakStones {
    pub fn new(stones: usize, capstones: usize) -> Self {
        TakStones { stones, capstones }
    }

    pub fn from_size(size: usize) -> Self {
        let (stones, capstones) = match size {
            3 => (10, 0),
            4 => (15, 0),
            5 => (21, 1),
            6 => (30, 1),
            7 => (40, 2),
            8 => (50, 2),
            _ => panic!("Invalid Tak board size"),
        };
        TakStones::new(stones, capstones)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TakGameSettings {
    pub size: usize,
    pub stones: TakStones,
    pub komi: TakKomi,
    pub time_mode: Option<TakTimeMode>,
}

impl TakGameSettings {
    pub fn new(
        size: usize,
        stones: TakStones,
        komi: TakKomi,
        time_mode: Option<TakTimeMode>,
    ) -> Self {
        TakGameSettings {
            size,
            stones,
            komi,
            time_mode,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TakHand {
    pub stones: usize,
    pub capstones: usize,
}

impl TakHand {
    pub fn new(stones: &TakStones) -> Self {
        TakHand {
            stones: stones.stones,
            capstones: stones.capstones,
        }
    }

    pub fn try_take(&mut self, variant: TakPieceVariant) -> bool {
        match variant {
            TakPieceVariant::Flat | TakPieceVariant::Wall => {
                if self.stones > 0 {
                    self.stones -= 1;
                    true
                } else {
                    false
                }
            }
            TakPieceVariant::Capstone => {
                if self.capstones > 0 {
                    self.capstones -= 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn can_take(&self, variant: TakPieceVariant) -> bool {
        match variant {
            TakPieceVariant::Flat | TakPieceVariant::Wall => self.stones > 0,
            TakPieceVariant::Capstone => self.capstones > 0,
        }
    }

    pub fn undo_take(&mut self, variant: TakPieceVariant) {
        match variant {
            TakPieceVariant::Flat | TakPieceVariant::Wall => self.stones += 1,
            TakPieceVariant::Capstone => self.capstones += 1,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.stones == 0 && self.capstones == 0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TakActionRecord {
    PlacePiece {
        pos: TakCoord,
        variant: TakPieceVariant,
        player: TakPlayer,
    },
    MovePiece {
        pos: TakCoord,
        dir: TakDir,
        take: usize,
        drops: Vec<usize>,
        flattened: bool,
    },
}
#[derive(Debug, Clone, PartialEq)]
pub enum TakAction {
    PlacePiece {
        pos: TakCoord,
        variant: TakPieceVariant,
    },
    MovePiece {
        pos: TakCoord,
        dir: TakDir,
        take: usize,
        drops: Vec<usize>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TakGame {
    pub komi: TakKomi,
    pub board: TakBoard,
    pub current_player: TakPlayer,
    pub action_history: Vec<TakActionRecord>,
    pub hands: [TakHand; 2],
    pub game_state: TakGameState,
    pub clock: Option<TakClock>,
}

impl TakGame {
    pub fn new(settings: TakGameSettings) -> Self {
        let size = settings.size;
        let board = TakBoard::new(size);
        let hands = [
            TakHand::new(&settings.stones),
            TakHand::new(&settings.stones),
        ];
        let clock = settings.time_mode.as_ref().map(|mode| TakClock::new(&mode));
        TakGame {
            komi: settings.komi,
            board,
            current_player: TakPlayer::White,
            action_history: Vec::new(),
            hands,
            game_state: TakGameState::Ongoing,
            clock,
        }
    }

    pub fn check_timeout(&mut self) -> bool {
        if let Some(clock) = &mut self.clock {
            if clock.get_time_remaining(self.current_player, true) == 0 {
                self.game_state =
                    TakGameState::Win(self.current_player.other(), TakWinReason::Timeout);
                return true;
            }
        }
        false
    }

    fn on_end_move(&mut self, record: TakActionRecord) {
        let affected_positions = match &record {
            TakActionRecord::PlacePiece { pos, .. } => vec![*pos],
            TakActionRecord::MovePiece {
                pos, dir, drops, ..
            } => {
                let mut positions = vec![*pos];
                let mut current_pos = *pos;
                for _ in 0..drops.len() {
                    current_pos = current_pos.offset_dir(*dir);
                    positions.push(current_pos);
                }
                positions
            }
        };

        if let Some(road) = self
            .board
            .check_for_road(&affected_positions, self.current_player)
        {
            self.game_state =
                TakGameState::Win(self.current_player, TakWinReason::Road(road.0, road.1));
        } else if let Some(road) = self
            .board
            .check_for_road(&affected_positions, self.current_player.other())
        {
            self.game_state = TakGameState::Win(
                self.current_player.other(),
                TakWinReason::Road(road.0, road.1),
            );
        } else if !self.board.has_empty_space() || self.hands.iter().any(TakHand::is_empty) {
            let counts = self.board.count_flats();
            if let Some(winner) = self.komi.determine_winner(counts) {
                self.game_state = TakGameState::Win(winner, TakWinReason::Flat);
            } else {
                self.game_state = TakGameState::Draw;
            }
        }

        if let Some(clock) = &mut self.clock {
            clock.update(TakTimestamp::now(), self.current_player);
            if clock.get_time_remaining(self.current_player, false) == 0 {
                self.game_state =
                    TakGameState::Win(self.current_player.other(), TakWinReason::Timeout);
            }
        }

        self.action_history.push(record);
        self.current_player = self.current_player.other();
    }

    pub fn try_do_action(&mut self, action: TakAction) -> Result<(), TakInvalidActionError> {
        match action {
            TakAction::PlacePiece { pos, variant } => self
                .try_place(pos, variant)
                .map_err(TakInvalidActionError::InvalidPlace),
            TakAction::MovePiece {
                pos,
                dir,
                take,
                drops,
            } => self
                .try_move(pos, dir, take, &drops)
                .map_err(TakInvalidActionError::InvalidMove),
        }
    }

    pub fn undo_action(&mut self) -> Result<(), TakInvalidUndoActionError> {
        let last_action = self
            .action_history
            .pop()
            .ok_or(TakInvalidUndoActionError::NoLastAction)?;
        self.current_player = self.current_player.other();
        self.game_state = TakGameState::Ongoing;

        match last_action.clone() {
            TakActionRecord::PlacePiece {
                pos,
                variant,
                player,
            } => self
                .try_undo_place(pos, variant, player)
                .map_err(TakInvalidUndoActionError::InvalidPlace),
            TakActionRecord::MovePiece {
                pos,
                dir,
                take,
                drops,
                flattened,
            } => self
                .try_undo_move(pos, dir, take, &drops, flattened)
                .map_err(TakInvalidUndoActionError::InvalidMove),
        }
        .expect(
            format!(
                "Undo action should not fail: {:?}, {:?}, {}",
                last_action,
                self.action_history,
                self.board.to_partial_tps()
            )
            .as_str(),
        );
        Ok(())
    }

    pub fn try_place(
        &mut self,
        pos: TakCoord,
        variant: TakPieceVariant,
    ) -> Result<(), TakInvalidPlaceError> {
        if self.game_state != TakGameState::Ongoing {
            return Err(TakInvalidPlaceError::NotAllowed);
        }

        self.board.can_place(pos)?;
        let player = if self.action_history.len() < 2 {
            if variant != TakPieceVariant::Flat {
                return Err(TakInvalidPlaceError::InvalidVariant);
            }
            self.current_player.other()
        } else {
            self.current_player
        };
        let hand = &mut self.hands[player.index()];
        if !hand.try_take(variant) {
            return Err(TakInvalidPlaceError::NotEnoughStones);
        }
        self.board.do_place_unchecked(pos, variant, player);
        let record = TakActionRecord::PlacePiece {
            pos,
            variant,
            player,
        };
        self.on_end_move(record);
        Ok(())
    }

    fn try_undo_place(
        &mut self,
        pos: TakCoord,
        variant: TakPieceVariant,
        player: TakPlayer,
    ) -> Result<(), TakInvalidUndoPlaceError> {
        self.board.try_undo_place(pos, variant, player)?;
        let hand = &mut self.hands[player.index()];
        hand.undo_take(variant);
        Ok(())
    }

    pub fn try_move(
        &mut self,
        pos: TakCoord,
        dir: TakDir,
        take: usize,
        drops: &[usize],
    ) -> Result<(), TakInvalidMoveError> {
        if self.game_state != TakGameState::Ongoing {
            return Err(TakInvalidMoveError::NotAllowed);
        }

        if self.action_history.len() < 2 {
            return Err(TakInvalidMoveError::NotAllowed);
        }
        let flattened = self.board.try_move(pos, dir, take, drops)?;
        let record = TakActionRecord::MovePiece {
            pos,
            dir,
            take,
            drops: drops.to_vec(),
            flattened,
        };
        self.on_end_move(record);
        Ok(())
    }

    fn try_undo_move(
        &mut self,
        pos: TakCoord,
        dir: TakDir,
        take: usize,
        drops: &[usize],
        flattened: bool,
    ) -> Result<(), TakInvalidUndoMoveError> {
        self.board.try_undo_move(pos, dir, take, drops, flattened)
    }

    pub fn gen_moves(&self) -> Vec<TakAction> {
        if self.game_state != TakGameState::Ongoing {
            return Vec::new();
        }

        let mut moves = Vec::new();
        let player = self.current_player;
        let hand = &self.hands[player.index()];

        for pos in TakCoord::iter_board(self.board.size) {
            if self.board.can_place(pos).is_ok() {
                if hand.stones > 0 {
                    moves.push(TakAction::PlacePiece {
                        pos,
                        variant: TakPieceVariant::Flat,
                    });
                    if self.action_history.len() >= 2 {
                        moves.push(TakAction::PlacePiece {
                            pos,
                            variant: TakPieceVariant::Wall,
                        });
                    }
                }
                if hand.capstones > 0 && self.action_history.len() >= 2 {
                    moves.push(TakAction::PlacePiece {
                        pos,
                        variant: TakPieceVariant::Capstone,
                    });
                }
            }
        }

        if self.action_history.len() < 2 {
            return moves;
        }

        for (pos, tower) in self.board.iter_pieces(player) {
            for take in 1..=tower.height().min(self.board.size) {
                for &dir in &[TakDir::Up, TakDir::Down, TakDir::Left, TakDir::Right] {
                    for drop_len in 1..=take {
                        let offset_pos = pos.offset_dir_many(dir, drop_len as i32);
                        if !offset_pos.is_valid(self.board.size) {
                            break;
                        }
                        let drops_vec = partition_number(take, drop_len);
                        for drops in drops_vec {
                            if self.board.try_get_tower(offset_pos).is_some_and(|t| {
                                t.variant == TakPieceVariant::Capstone
                                    || (t.variant == TakPieceVariant::Wall
                                        && !(*drops.last().expect("Drops should not be empty")
                                            == 1
                                            && tower.variant == TakPieceVariant::Capstone))
                            }) {
                                break;
                            }
                            moves.push(TakAction::MovePiece {
                                pos,
                                dir,
                                take,
                                drops,
                            });
                        }
                        if self
                            .board
                            .try_get_tower(offset_pos)
                            .is_some_and(|t| t.variant != TakPieceVariant::Flat)
                        {
                            break;
                        }
                    }
                }
            }
        }

        moves
    }

    pub fn to_tps(&self) -> String {
        format!(
            "{} {}",
            self.board.to_partial_tps(),
            match self.current_player {
                TakPlayer::White => " 1",
                TakPlayer::Black => " 2",
            }
        )
    }

    pub fn try_from_tps(tps: &str, komi: TakKomi) -> Option<Self> {
        let mut parts = tps.split_whitespace();
        let board_str = parts.next()?;
        let player_str = parts.next()?;
        let player = match player_str {
            "1" => TakPlayer::White,
            "2" => TakPlayer::Black,
            _ => return None,
        };
        let turn_index = parts.next()?.parse::<usize>().ok()? - 1;
        let board = TakBoard::try_from_partial_tps(board_str)?;
        let white_stones = board.count_stones(TakPlayer::White);
        let black_stones = board.count_stones(TakPlayer::Black);
        let stones = TakStones::from_size(board.size);
        Some(TakGame {
            komi,
            board,
            current_player: player,
            action_history: vec![
                TakActionRecord::PlacePiece {
                    pos: TakCoord::new(0, 0),       // Placeholder, will not be used
                    variant: TakPieceVariant::Flat, // Placeholder, will not be used
                    player: TakPlayer::White,       // Placeholder, will not be used
                };
                turn_index
            ],
            hands: [
                TakHand::new(&TakStones::new(
                    stones.stones - white_stones.0,
                    stones.capstones - white_stones.1,
                )),
                TakHand::new(&TakStones::new(
                    stones.stones - black_stones.0,
                    stones.capstones - black_stones.1,
                )),
            ],
            game_state: TakGameState::Ongoing,
            clock: None,
        })
    }

    pub fn validate(&self, stones: &TakStones) {
        let stone_count = self.board.count_stones(TakPlayer::White);
        if stone_count.0 + self.hands[0].stones != stones.stones
            || stone_count.1 + self.hands[0].capstones != stones.capstones
        {
            println!("{:?}", self.action_history);
            println!("{}", self.to_tps());
            panic!(
                "Invalid stone count for White: {} in board, {} in hand, should be {}",
                stone_count.0, self.hands[0].stones, stones.stones
            );
        }
        let stone_count = self.board.count_stones(TakPlayer::Black);
        if stone_count.0 + self.hands[1].stones != stones.stones
            || stone_count.1 + self.hands[1].capstones != stones.capstones
        {
            println!("{:?}", self);
            panic!(
                "Invalid stone count for Black: {} in board, {} in hand, should be {}",
                stone_count.0, self.hands[1].stones, stones.stones
            );
        }
    }
}

fn partition_number(num: usize, n: usize) -> Vec<Vec<usize>> {
    if n == 0 {
        if num == 0 {
            Vec::new()
        } else {
            Vec::new()
        }
    } else if n == 1 {
        if num == 0 {
            Vec::new()
        } else {
            vec![vec![num]]
        }
    } else if num < n {
        Vec::new()
    } else {
        let mut result = Vec::new();
        for first in 1..=(num - n + 1) {
            for mut rest in partition_number(num - first, n - 1) {
                let mut partition = vec![first];
                partition.append(&mut rest);
                result.push(partition);
            }
        }
        result
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_number_basic() {
        // partition 5 into 2 parts: should be [[1,4], [2,3], [3,2], [4,1]]
        let mut result = partition_number(5, 2);
        result.sort();
        let mut expected = vec![vec![1, 4], vec![2, 3], vec![3, 2], vec![4, 1]];
        expected.sort();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_partition_number_three_parts() {
        // partition 6 into 3 parts: should be [[1,1,4], [1,2,3], [1,3,2], [1,4,1], [2,1,3], [2,2,2], [2,3,1], [3,1,2], [3,2,1], [4,1,1]]
        let mut result = partition_number(6, 3);
        result.sort();
        let mut expected = vec![
            vec![1, 1, 4],
            vec![1, 2, 3],
            vec![1, 3, 2],
            vec![1, 4, 1],
            vec![2, 1, 3],
            vec![2, 2, 2],
            vec![2, 3, 1],
            vec![3, 1, 2],
            vec![3, 2, 1],
            vec![4, 1, 1],
        ];
        expected.sort();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_partition_number_single_part() {
        // partition 7 into 1 part: should be [[7]]
        let result = partition_number(7, 1);
        assert_eq!(result, vec![vec![7]]);
    }

    #[test]
    fn test_partition_number_no_parts() {
        // partition 0 into 0 parts: should be []
        let result = partition_number(0, 0);
        assert_eq!(result, Vec::<Vec<usize>>::new());
    }

    #[test]
    fn test_partition_number_invalid() {
        // partition 3 into 5 parts: not possible, should be []
        let result = partition_number(3, 5);
        assert_eq!(result, Vec::<Vec<usize>>::new());
    }

    #[test]
    fn precompute_values() {
        for num in 0..=10 {
            for n in 1..=num {
                let partitions = partition_number(num, n);
                println!("partition_number({}, {}) = {:?}", num, n, partitions.len());
            }
        }
    }
}
