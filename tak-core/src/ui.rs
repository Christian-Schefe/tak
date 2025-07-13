use std::collections::HashMap;

use crate::{
    TakAction, TakActionRecord, TakCoord, TakDir, TakGame, TakGameState, TakInvalidActionError,
    TakPieceVariant, TakPlayer, TakWinReason,
};

#[derive(Debug, Clone, PartialEq)]
pub struct TakPartialMove {
    pub take: usize,
    pub drops: Vec<usize>,
    pub pos: TakCoord,
    pub dir: Option<TakDir>,
}

impl TakPartialMove {
    pub fn new(take: usize, pos: TakCoord) -> Self {
        TakPartialMove {
            take,
            drops: Vec::new(),
            pos,
            dir: None,
        }
    }

    pub fn try_to_action(&self) -> Option<TakAction> {
        let dir = self.dir?;
        let drop_sum: usize = self.drops.iter().sum();
        if drop_sum > self.take {
            return None;
        }
        let mut drops = self.drops.clone();
        *drops.last_mut()? += self.take - drop_sum;
        Some(TakAction::MovePiece {
            take: self.take,
            drops,
            pos: self.pos,
            dir,
        })
    }

    pub fn is_valid(&self) -> bool {
        self.dir.is_some() && self.drops.iter().sum::<usize>() == self.take
    }
}

pub struct TakUIState {
    preview_game: TakGame,
    actual_game: TakGame,
    pub pieces: HashMap<usize, TakUIPiece>,
    pub tiles: HashMap<TakCoord, TakUITile>,
    pub partial_move: Option<TakPartialMove>,
    pub available_piece_types: [Vec<TakPieceVariant>; 2],
    pub flat_counts: [usize; 2],
    pub on_game_update: Vec<Box<dyn FnMut()>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TakUITile {
    pub owner: Option<TakPlayer>,
    pub bridges: Vec<(TakDir, bool)>,
    pub highlighted: bool,
    pub selectable: bool,
    pub last_action: bool,
}

impl TakUITile {
    pub fn get_center_corners(&self) -> Vec<(TakDir, TakDir)> {
        let pairs = [
            (TakDir::Up, TakDir::Left),
            (TakDir::Up, TakDir::Right),
            (TakDir::Down, TakDir::Left),
            (TakDir::Down, TakDir::Right),
        ];
        pairs
            .into_iter()
            .filter(|&(dir1, dir2)| {
                !self.bridges.contains(&(dir1, true)) && !self.bridges.contains(&(dir2, true))
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TakUIPiece {
    pub player: TakPlayer,
    pub variant: TakPieceVariant,
    pub pos: TakCoord,
    pub height: usize,
    pub is_floating: bool,
    pub z_priority: Option<usize>,
}

impl TakUIState {
    pub fn new(game: TakGame) -> Self {
        let mut state = Self {
            actual_game: game.clone(),
            preview_game: game,
            pieces: HashMap::new(),
            partial_move: None,
            tiles: HashMap::new(),
            available_piece_types: [Vec::new(), Vec::new()],
            flat_counts: [0, 0],
            on_game_update: Vec::new(),
        };
        state.on_game_update();
        state
    }

    pub fn add_listener<F>(&mut self, listener: F)
    where
        F: FnMut() + 'static,
    {
        self.on_game_update.push(Box::new(listener));
    }

    pub fn game(&self) -> &TakGame {
        &self.actual_game
    }

    pub fn game_mut(&mut self) -> &mut TakGame {
        &mut self.actual_game
    }

    pub fn set_time_remaining(&mut self, player: TakPlayer, time_remaining: u64) {
        self.actual_game.set_time_remaining(player, time_remaining);
        self.on_game_update();
    }

    pub fn try_do_action(&mut self, action: TakAction) -> Result<(), TakInvalidActionError> {
        self.actual_game.try_do_action(action)?;
        self.preview_game = self.actual_game.clone();
        self.partial_move = None;
        self.on_game_update();
        Ok(())
    }

    fn do_partial_move(&mut self, action: TakAction) {
        self.preview_game = self.actual_game.clone();
        self.preview_game
            .try_do_action(action.clone())
            .expect("Partial move should succeed");
        self.on_game_update();
    }

    pub fn add_square_to_partial_move(&mut self, new_pos: TakCoord) -> Option<()> {
        self.update_partial_move(new_pos);
        self.preview_game = self.actual_game.clone();

        if let Some(partial_move) = self.partial_move.as_ref() {
            if let Some(action) = partial_move.try_to_action() {
                if partial_move.is_valid() {
                    self.try_do_action(action)
                        .expect("Applying partial move should succeed");
                    self.partial_move = None;
                    return Some(());
                } else {
                    self.do_partial_move(action);
                    return None;
                }
            }
        }

        self.on_game_update();
        None
    }

    fn update_partial_move(&mut self, new_pos: TakCoord) {
        if self.actual_game.game_state != TakGameState::Ongoing {
            self.partial_move = None;
            return;
        }

        if let Some(TakPartialMove {
            take,
            drops,
            dir,
            pos,
        }) = &mut self.partial_move
        {
            let Some(stack) = self.actual_game.board.try_get_stack(*pos) else {
                self.partial_move = None;
                return;
            };
            let drop_pos = dir.map_or(*pos, |d| pos.offset_dir_many(d, drops.len() as i32));
            if new_pos == drop_pos {
                if let Some(last_drop) = drops.last_mut() {
                    *last_drop += 1;
                } else {
                    *take -= 1;
                    if *take == 0 {
                        self.partial_move = None;
                        return;
                    }
                }
            } else {
                let Some(new_dir) = new_pos.is_adjacent(&drop_pos) else {
                    self.partial_move = None;
                    return;
                };
                if let Some(dir) = dir {
                    if *dir != new_dir {
                        self.partial_move = None;
                        return;
                    }
                }
                if let Some(other_stack) = self.actual_game.board.try_get_stack(new_pos) {
                    if other_stack.variant == TakPieceVariant::Capstone {
                        self.partial_move = None;
                        return;
                    }
                    if other_stack.variant == TakPieceVariant::Wall {
                        let pieces_to_drop = *take - drops.iter().sum::<usize>();
                        if pieces_to_drop != 1 || stack.variant != TakPieceVariant::Capstone {
                            self.partial_move = None;
                            return;
                        }
                    }
                };
                *dir = Some(new_dir);
                drops.push(1);
            }
        } else {
            let Some(stack) = self.actual_game.board.try_get_stack(new_pos) else {
                return;
            };
            if stack.player() == self.actual_game.current_player {
                let take = stack.height().min(self.actual_game.board.size);
                self.partial_move = Some(TakPartialMove::new(take, new_pos));
            }
        }
    }

    fn get_stones_from_last_action_in_order(game: &TakGame) -> Vec<usize> {
        let Some(last_action) = game.action_history.last() else {
            return Vec::new();
        };
        match last_action {
            TakActionRecord::PlacePiece { pos, .. } => vec![
                game.board
                    .try_get_stack(*pos)
                    .unwrap()
                    .composition
                    .last()
                    .unwrap()
                    .id,
            ],
            TakActionRecord::MovePiece {
                pos, dir, drops, ..
            } => {
                let mut stones = vec![];
                for i in 0..drops.len() {
                    let new_pos = pos.offset_dir_many(*dir, (i + 1) as i32);
                    let stack = game.board.try_get_stack(new_pos).unwrap();
                    stones.extend(
                        stack.composition[stack.height() - drops[i]..]
                            .iter()
                            .map(|s| s.id),
                    );
                }
                stones
            }
        }
    }

    pub fn on_game_update(&mut self) {
        self.pieces.clear();
        self.tiles.clear();
        self.flat_counts = [0, 0];

        let drop_diff = match &self.partial_move {
            Some(TakPartialMove {
                take,
                drops,
                pos,
                dir,
            }) => {
                let drop_pos = dir.map_or(*pos, |d| pos.offset_dir_many(d, drops.len() as i32));
                Some((drop_pos, take.saturating_sub(drops.iter().sum())))
            }
            _ => None,
        };

        let priority_stones = if self.partial_move.is_none() {
            Self::get_stones_from_last_action_in_order(&self.actual_game)
        } else {
            Self::get_stones_from_last_action_in_order(&self.preview_game)
        };
        for (pos, stack) in self.preview_game.board.iter_pieces(None) {
            let stack_height = stack.height();
            let floating_threshold = drop_diff
                .filter(|x| x.0 == pos)
                .map(|x| stack_height.saturating_sub(x.1));
            for (height, stone) in stack.composition.iter().enumerate() {
                let priority_index = priority_stones.iter().position(|&id| id == stone.id);
                self.pieces.insert(
                    stone.id,
                    TakUIPiece {
                        player: stone.player,
                        pos,
                        height,
                        is_floating: floating_threshold.is_some_and(|x| height >= x),
                        z_priority: priority_index,
                        variant: if height + 1 == stack_height {
                            stack.variant
                        } else {
                            TakPieceVariant::Flat
                        },
                    },
                );
            }
        }

        let mut click_options = Vec::new();
        if let Some(partial_move) = &self.partial_move {
            let drop_pos = partial_move.dir.map_or(partial_move.pos, |d| {
                partial_move
                    .pos
                    .offset_dir_many(d, partial_move.drops.len() as i32)
            });
            click_options.push(drop_pos);
            let check_dirs = partial_move.dir.map_or(TakDir::ALL.to_vec(), |d| vec![d]);
            for dir in check_dirs {
                let new_pos = drop_pos.offset_dir(dir);
                if !new_pos.is_valid(self.preview_game.board.size) {
                    continue;
                }
                if let Some(other_stack) = self.preview_game.board.try_get_stack(new_pos) {
                    if other_stack.variant == TakPieceVariant::Flat {
                        click_options.push(new_pos);
                    } else if other_stack.variant == TakPieceVariant::Wall {
                        let stack = self
                            .actual_game
                            .board
                            .try_get_stack(partial_move.pos)
                            .expect("Partial move position should have a stack");
                        let drops_diff =
                            partial_move.take - partial_move.drops.iter().sum::<usize>();
                        if drops_diff == 1 && stack.variant == TakPieceVariant::Capstone {
                            click_options.push(new_pos);
                        }
                    }
                } else {
                    click_options.push(new_pos);
                }
            }
        }

        let mut highlighted_tiles = Vec::new();
        if let TakGameState::Win(player, TakWinReason::Road) = self.actual_game.game_state {
            let all_positions =
                TakCoord::iter_board(self.actual_game.board.size).collect::<Vec<TakCoord>>();
            let road = self
                .actual_game
                .board
                .check_for_road(&all_positions, player)
                .expect("Player should have a road");
            highlighted_tiles = self
                .actual_game
                .board
                .find_shortest_path(road.0, road.1)
                .expect("Should find a path for road");
            #[cfg(feature = "wasm")]
            dioxus::logger::tracing::info!("{:?}", highlighted_tiles);
        }

        let mut last_action_tiles = Vec::new();
        if highlighted_tiles.len() == 0 {
            if let Some(last_action) = self.actual_game.action_history.last() {
                last_action_tiles = match last_action {
                    TakActionRecord::PlacePiece { pos, .. } => vec![*pos],
                    TakActionRecord::MovePiece {
                        pos, dir, drops, ..
                    } => {
                        let mut tiles = vec![*pos];
                        for i in 1..=drops.len() {
                            tiles.push(pos.offset_dir_many(*dir, i as i32));
                        }
                        tiles
                    }
                };
            }
        }

        for pos in TakCoord::iter_board(self.preview_game.board.size) {
            let bridges = self.get_bridges(pos);
            let owner = bridges.as_ref().map(|(p, _)| *p);
            let bridges = TakDir::ALL
                .iter()
                .map(|&dir| {
                    (
                        dir,
                        bridges.as_ref().map_or(false, |(_, b)| b.contains(&dir)),
                    )
                })
                .collect::<Vec<_>>();
            if let Some(stack) = self.preview_game.board.try_get_stack(pos) {
                if stack.variant == TakPieceVariant::Flat {
                    self.flat_counts[stack.player().index()] += 1;
                }
            }
            self.tiles.insert(
                pos,
                TakUITile {
                    owner,
                    bridges,
                    highlighted: false,
                    selectable: false,
                    last_action: false,
                },
            );
        }
        for pos in click_options {
            self.tiles.get_mut(&pos).unwrap().selectable = true;
        }
        for pos in highlighted_tiles {
            self.tiles.get_mut(&pos).unwrap().highlighted = true;
        }
        for pos in last_action_tiles {
            self.tiles.get_mut(&pos).unwrap().last_action = true;
        }

        if self.actual_game.ply_index < 2 {
            self.available_piece_types = [vec![TakPieceVariant::Flat], vec![TakPieceVariant::Flat]];
        } else {
            self.available_piece_types =
                self.actual_game
                    .hands
                    .clone()
                    .map(|hand| match (hand.stones, hand.capstones) {
                        (0, 0) => vec![],
                        (0, _) => vec![TakPieceVariant::Capstone],
                        (_, 0) => vec![TakPieceVariant::Flat, TakPieceVariant::Wall],
                        (_, _) => vec![
                            TakPieceVariant::Flat,
                            TakPieceVariant::Wall,
                            TakPieceVariant::Capstone,
                        ],
                    });
        }

        for listener in self.on_game_update.iter_mut() {
            listener();
        }
    }

    fn get_bridges(&self, pos: TakCoord) -> Option<(TakPlayer, Vec<TakDir>)> {
        let mut bridges = Vec::new();
        let size = self.preview_game.board.size;

        let Some(stack) = self.preview_game.board.try_get_stack(pos) else {
            return None;
        };
        let player = stack.player();
        if stack.variant == TakPieceVariant::Wall {
            return None;
        }
        for dir in TakDir::ALL {
            let new_pos = pos.offset_dir(dir);
            if !new_pos.is_valid(size) {
                bridges.push(dir);
                continue;
            }
            if let Some(other_stack) = self.preview_game.board.try_get_stack(new_pos) {
                if other_stack.variant != TakPieceVariant::Wall && other_stack.player() == player {
                    bridges.push(dir);
                }
            }
        }
        Some((player, bridges))
    }
}
