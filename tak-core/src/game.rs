use crate::{
    TakAction, TakActionRecord, TakBoard, TakClock, TakCoord, TakDir, TakGameState,
    TakInvalidActionError, TakInvalidMoveError, TakInvalidPlaceError, TakPieceVariant, TakPlayer,
    TakPtn, TakPtnAttr, TakTimeMode, TakTimestamp, TakTps, TakWinReason,
};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TakKomi {
    pub amount: usize,
    pub tiebreak: bool,
}

impl TakKomi {
    pub fn new(amount: usize, tiebreak: bool) -> Self {
        TakKomi { amount, tiebreak }
    }
    pub fn none() -> Self {
        TakKomi {
            amount: 0,
            tiebreak: false,
        }
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TakGameSettings {
    pub size: usize,
    pub stones: TakStones,
    pub komi: TakKomi,
    pub time_mode: Option<TakTimeMode>,
    pub start_position: TakTps,
}

impl TakGameSettings {
    pub fn new(
        size: usize,
        stones: Option<TakStones>,
        komi: TakKomi,
        time_mode: Option<TakTimeMode>,
    ) -> Self {
        TakGameSettings {
            size,
            stones: stones.unwrap_or_else(|| TakStones::from_size(size)),
            komi,
            time_mode,
            start_position: TakTps::new_empty(size),
        }
    }

    pub fn new_with_position(
        size: usize,
        start_position: TakTps,
        stones: Option<TakStones>,
        komi: TakKomi,
        time_mode: Option<TakTimeMode>,
    ) -> Self {
        TakGameSettings {
            size,
            stones: stones.unwrap_or_else(|| TakStones::from_size(size)),
            komi,
            time_mode,
            start_position,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TakHand {
    pub stones: usize,
    pub capstones: usize,
}

impl TakHand {
    pub fn new(stones: usize, capstones: usize) -> Self {
        TakHand { stones, capstones }
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

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TakGame {
    pub settings: TakGameSettings,
    pub board: TakBoard,
    pub current_player: TakPlayer,
    pub ply_index: usize,
    pub action_history: Vec<TakActionRecord>,
    pub hands: [TakHand; 2],
    pub game_state: TakGameState,
    pub clock: Option<TakClock>,
}

impl TakGame {
    pub fn new(settings: TakGameSettings) -> Option<Self> {
        let size = settings.size;
        if size < 3 || size > 8 {
            return None;
        }
        let board = TakBoard::try_from_partial_tps(&settings.start_position.position)?;
        if board.size != size {
            return None;
        }
        let white_stones = board.count_stones(TakPlayer::White);
        let black_stones = board.count_stones(TakPlayer::Black);
        if white_stones.0 > settings.stones.stones
            || white_stones.1 > settings.stones.capstones
            || black_stones.0 > settings.stones.stones
            || black_stones.1 > settings.stones.capstones
        {
            return None;
        }
        let hands = [
            TakHand::new(
                settings.stones.stones - white_stones.0,
                settings.stones.capstones - white_stones.1,
            ),
            TakHand::new(
                settings.stones.stones - black_stones.0,
                settings.stones.capstones - black_stones.1,
            ),
        ];
        let clock = settings.time_mode.as_ref().map(|mode| TakClock::new(mode));
        Some(TakGame {
            board,
            current_player: settings.start_position.player,
            ply_index: settings.start_position.get_ply_index(),
            action_history: Vec::new(),
            hands,
            game_state: TakGameState::Ongoing,
            clock,
            settings,
        })
    }

    pub fn abort(&mut self, winner: TakPlayer) {
        self.check_timeout();
        if self.game_state != TakGameState::Ongoing {
            return;
        }
        self.game_state = TakGameState::Win(winner, TakWinReason::Timeout);
        if let Some(clock) = &mut self.clock {
            clock.set_time_remaining(winner.other(), 0);
        }
    }

    pub fn reset(&mut self) {
        *self = TakGame::new(self.settings.clone()).expect("Game should be valid");
    }

    pub fn get_time_remaining(&self, player: TakPlayer, apply_elapsed: bool) -> Option<u64> {
        self.clock
            .as_ref()
            .map(|clock| clock.get_time_remaining(player, apply_elapsed))
    }

    pub fn set_time_remaining(&mut self, player: TakPlayer, time_remaining: u64) {
        if let Some(clock) = &mut self.clock {
            clock.set_time_remaining(player, time_remaining);
            self.check_timeout();
        }
    }

    pub fn check_timeout(&mut self) -> bool {
        if self.game_state != TakGameState::Ongoing {
            return false;
        }
        if let Some(clock) = &mut self.clock {
            if clock.get_time_remaining(self.current_player, true) == 0 {
                self.game_state =
                    TakGameState::Win(self.current_player.other(), TakWinReason::Timeout);
                clock.set_time_remaining(self.current_player, 0);
                return true;
            } else if clock.get_time_remaining(self.current_player.other(), false) == 0 {
                self.game_state = TakGameState::Win(self.current_player, TakWinReason::Timeout);
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

        if let Some(_road) = self
            .board
            .check_for_road(&affected_positions, self.current_player)
        {
            self.game_state = TakGameState::Win(self.current_player, TakWinReason::Road);
        } else if let Some(_road) = self
            .board
            .check_for_road(&affected_positions, self.current_player.other())
        {
            self.game_state = TakGameState::Win(self.current_player.other(), TakWinReason::Road);
        } else if !self.board.has_empty_space() || self.hands.iter().any(TakHand::is_empty) {
            let counts = self.board.count_flats();
            if let Some(winner) = self.settings.komi.determine_winner(counts) {
                self.game_state = TakGameState::Win(winner, TakWinReason::Flat);
            } else {
                self.game_state = TakGameState::Draw;
            }
        }

        self.action_history.push(record);
        self.ply_index += 1;
        self.current_player = self.current_player.other();
    }

    pub fn seek_ply_index(&self, ply_index: usize) -> Option<Self> {
        if ply_index > self.ply_index {
            return None;
        }
        let mut game = self.clone();
        if ply_index < self.ply_index {
            game.game_state = TakGameState::Ongoing;
        }
        game.action_history.truncate(ply_index);
        let ptn = game.to_ptn();
        Some(TakGame::try_from_ptn(ptn).expect("Should be able to seek to ply index"))
    }

    pub fn try_do_action(&mut self, action: TakAction) -> Result<(), TakInvalidActionError> {
        let current_player = self.current_player;
        let now = if let Some(clock) = &mut self.clock {
            let now = TakTimestamp::now();
            if clock.get_time_remaining_at(current_player, now) == 0 {
                self.game_state = TakGameState::Win(current_player.other(), TakWinReason::Timeout);
                clock.set_time_remaining(current_player, 0);
            }
            Some(now)
        } else {
            None
        };
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
        }?;
        if let Some(clock) = &mut self.clock {
            clock.update(now.expect("Should have now timestamp"), current_player);
        }
        Ok(())
    }

    pub fn get_last_action(&self) -> Option<&TakActionRecord> {
        self.action_history.last()
    }

    fn try_place(
        &mut self,
        pos: TakCoord,
        variant: TakPieceVariant,
    ) -> Result<(), TakInvalidPlaceError> {
        if self.game_state != TakGameState::Ongoing {
            return Err(TakInvalidPlaceError::NotAllowed);
        }

        self.board.can_place(pos)?;
        let player = if self.ply_index < 2 {
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

    fn try_move(
        &mut self,
        pos: TakCoord,
        dir: TakDir,
        take: usize,
        drops: &[usize],
    ) -> Result<(), TakInvalidMoveError> {
        if self.game_state != TakGameState::Ongoing {
            return Err(TakInvalidMoveError::NotAllowed);
        }

        if self.ply_index < 2 {
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

    pub fn to_tps(&self) -> TakTps {
        TakTps::new(self.board.to_partial_tps(), self.ply_index)
    }

    pub fn to_ptn(&self) -> TakPtn {
        let turns = self
            .action_history
            .iter()
            .map(|x| x.to_ptn())
            .collect::<Vec<_>>();
        let mut attributes = vec![
            TakPtnAttr::Size(self.board.size),
            TakPtnAttr::Komi(self.settings.komi.amount, self.settings.komi.tiebreak),
            TakPtnAttr::Flats(self.settings.stones.stones),
            TakPtnAttr::Caps(self.settings.stones.capstones),
        ];
        if let Some(time_mode) = &self.settings.time_mode {
            attributes.push(TakPtnAttr::Clock(time_mode.time, time_mode.increment));
        }
        if self.settings.start_position != TakTps::new_empty(self.board.size) {
            attributes.push(TakPtnAttr::TPS(self.settings.start_position.clone()));
        }
        let mut ptn = TakPtn::new(
            turns,
            self.settings.start_position.move_index,
            self.game_state.clone(),
        );
        ptn.attributes = attributes;
        ptn
    }

    pub fn try_from_ptn(ptn: TakPtn) -> Option<Self> {
        let settings = ptn.get_settings()?;
        let mut game = Self::new(settings)?;

        let mut actions = Vec::new();
        for (i, (_, white_turn, black_turn)) in ptn.turns.iter().enumerate() {
            if let Some(white_turn) = white_turn {
                actions.push(TakAction::from_ptn(&white_turn)?);
            } else if i != 0 {
                return None;
            }
            if let Some(black_turn) = black_turn {
                actions.push(TakAction::from_ptn(&black_turn)?);
            } else if i != ptn.turns.len() - 1 {
                return None;
            }
        }

        for action in actions {
            let res = game.try_do_action(action);
            if let Err(e) = res {
                eprintln!(
                    "Error applying action to game: {}, error: {:?}",
                    game.board.to_partial_tps(),
                    e
                );
                return None;
            }
        }

        if let TakGameState::Win(_, TakWinReason::Timeout) = &ptn.game_state {
            game.game_state = ptn.game_state;
        } else if ptn.game_state != game.game_state {
            eprintln!(
                "PTN game state does not match actual game state: {:?} != {:?}",
                ptn.game_state, game.game_state
            );
            return None;
        }

        Some(game)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.board.validate()?;
        match self.ply_index % 2 {
            0 if self.current_player == TakPlayer::White => {}
            1 if self.current_player == TakPlayer::Black => {}
            _ => {
                return Err(format!(
                    "Current player {:?} does not match ply index {}",
                    self.current_player, self.ply_index
                ));
            }
        }
        let stones = &self.settings.stones;
        let stone_count = self.board.count_stones(TakPlayer::White);
        if stone_count.0 + self.hands[0].stones != stones.stones
            || stone_count.1 + self.hands[0].capstones != stones.capstones
        {
            return Err(format!(
                "Invalid stone count for White: {} in board, {} in hand, should be {}",
                stone_count.0, self.hands[0].stones, stones.stones
            ));
        }
        let stone_count = self.board.count_stones(TakPlayer::Black);
        if stone_count.0 + self.hands[1].stones != stones.stones
            || stone_count.1 + self.hands[1].capstones != stones.capstones
        {
            return Err(format!(
                "Invalid stone count for Black: {} in board, {} in hand, should be {}",
                stone_count.0, self.hands[1].stones, stones.stones
            ));
        }
        if self.action_history.len() > self.ply_index {
            return Err(format!(
                "Action history length {} exceeds turn index {}",
                self.action_history.len(),
                self.ply_index
            ));
        }
        if self.current_player.index() != self.ply_index % 2 {
            return Err(format!(
                "Current player {:?} does not match turn index {}",
                self.current_player, self.ply_index
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_ptn() {
        let ptn = r#"
[Site "PlayTak.com"]
[Event "Online Play"]
[Date "2025.06.26"]
[Time "19:32:14"]
[Player1 "Abyss"]
[Rating1 "2100"]
[Player2 "alion02"]
[Rating2 "2240"]
[Clock "15:0 +10"]
[Result "0-F"]
[Size "6"]
[Komi "2"]
[Flats "30"]
[Caps "1"]
[Opening "swap"]

1. a6 f1
2. d3 c4
3. d4 d5
4. c3 b3
5. c5 b4
6. c2 b2
7. c1 Cd2
8. b1 d1
9. c6 d2<
10. Cb5 d2
11. e3 e2
12. f3 a4
13. b5- a3
14. 2b4- a2
15. 3b3- a1
16. b1< b4
17. f2 2c2+
18. e5 e2+
19. a5 f4
20. e4 3c3>
21. c3 d6
22. c1> e6
23. f5 4d3+
24. 4b2+13 f6
25. 4b4> c2
26. Sd3 b2
27. e2 b5
28. b4 b1
29. b4+ b4
30. c1 b6
31. e2< d6<
32. c5+ b6>
33. Sb6 Sd6
34. b6> Sb6
35. 4c6- e1
36. c1+ d6<
37. d3> d6
38. 3e3-12 b6-
39. c1 b6
40. 5c4< c4 0-F
"#;
        let ptn = TakPtn::try_from_str(ptn).expect("Failed to parse PTN");
        let game = TakGame::try_from_ptn(ptn).expect("Failed to create game from PTN");
        assert_eq!(game.board.size, 6);
        assert_eq!(game.settings.komi.amount, 2);
        assert_eq!(game.settings.stones.stones, 30);
        assert_eq!(game.settings.stones.capstones, 1);
        assert_eq!(game.current_player, TakPlayer::White);
        assert_eq!(game.ply_index, 80);
        assert_eq!(
            game.game_state,
            TakGameState::Win(TakPlayer::Black, TakWinReason::Flat)
        );
        assert_eq!(
            game.to_tps().to_string(),
            "2,2,12S,2,2,2/1,212S,2121S,2,1,1/2,222221C,2,11112C,1,2/2,2,1,x2,1/2,2,21,21,1,1/21,2,1,21,221S,1 1 41"
        );
    }
}
