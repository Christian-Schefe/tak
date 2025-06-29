#[derive(Clone, Debug, PartialEq)]
pub struct TakGame {
    pub size: usize,
    pub board: Vec<TakTile>,
    pub current_player: Player,
    pub actions: Vec<TakAction>,
    pub hands: [TakHand; 2],
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
}

#[derive(Clone, Debug, PartialEq)]
pub struct TakHand {
    stones: usize,
    capstones: usize,
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

    fn try_get_positions(
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
pub enum Player {
    White,
    Black,
}

impl Player {
    pub fn opponent(&self) -> Player {
        match self {
            Player::White => Player::Black,
            Player::Black => Player::White,
        }
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
    top_type: TakPieceType,
    composition: Vec<Player>,
}

impl TakTower {
    pub fn controlling_player(&self) -> Player {
        self.composition[self.composition.len() - 1]
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
}

#[derive(Clone, Debug, PartialEq)]
pub enum TakAction {
    PlacePiece {
        position: TakCoord,
        piece_type: TakPieceType,
    },
    MovePiece {
        from: TakCoord,
        direction: Direction,
        take: usize,
        drops: Vec<usize>,
    },
}

impl TakAction {
    pub fn to_ptn(&self) -> String {
        match self {
            TakAction::PlacePiece {
                position,
                piece_type,
            } => {
                let prefix = match piece_type {
                    TakPieceType::Flat => "",
                    TakPieceType::Wall => "S",
                    TakPieceType::Capstone => "C",
                };
                let file = (b'a' + position.x as u8) as char;
                let rank = position.y + 1;
                format!("{}{}{}", prefix, file, rank)
            }
            TakAction::MovePiece {
                from,
                direction,
                take,
                drops,
            } => {
                let take_str = if *take == 1 {
                    String::new()
                } else {
                    format!("{}", take)
                };
                let file = (b'a' + from.x as u8) as char;
                let rank = from.y + 1;
                let direction_str = match direction {
                    Direction::Up => "+",
                    Direction::Down => "-",
                    Direction::Left => "<",
                    Direction::Right => ">",
                };
                let drops_str: String = drops
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join("");
                format!("{}{}{}{}{}", take_str, file, rank, direction_str, drops_str)
            }
        }
    }
    pub fn from_ptn(ptn: &str) -> Option<Self> {
        let place_regex = regex::Regex::new(r"^([SC]?)([a-z])([1-9])$").unwrap();
        let move_regex = regex::Regex::new(r"^([1-9]?)([a-z])([1-9])([+-<>])([1-9]*)\*?$").unwrap();

        if let Some(captures) = place_regex.captures(ptn) {
            let piece_type = match &captures[1] {
                "" => TakPieceType::Flat,
                "S" => TakPieceType::Wall,
                "C" => TakPieceType::Capstone,
                _ => return None,
            };
            let file = captures[2].chars().next()?.to_ascii_lowercase() as usize - 'a' as usize;
            let rank = captures[3].parse::<usize>().ok()? - 1;
            Some(TakAction::PlacePiece {
                position: TakCoord::new(file, rank),
                piece_type,
            })
        } else if let Some(captures) = move_regex.captures(ptn) {
            let take = captures[1].parse::<usize>().unwrap_or(1);
            let file = captures[2].chars().next()?.to_ascii_lowercase() as usize - 'a' as usize;
            let rank = captures[3].parse::<usize>().ok()? - 1;
            let direction = match &captures[4] {
                "+" => Direction::Up,
                "-" => Direction::Down,
                "<" => Direction::Left,
                ">" => Direction::Right,
                _ => return None,
            };
            let drops_str = &captures[5];
            let drops: Vec<usize> = if !drops_str.is_empty() {
                drops_str
                    .chars()
                    .filter_map(|d| d.to_digit(10))
                    .map(|d| d as usize)
                    .collect()
            } else {
                vec![take]
            };
            Some(TakAction::MovePiece {
                from: TakCoord::new(file, rank),
                direction,
                take,
                drops,
            })
        } else {
            None
        }
    }
}

impl TakGame {
    pub fn new(size: usize) -> Self {
        TakGame {
            size,
            board: vec![None; size * size],
            current_player: Player::White,
            actions: Vec::new(),
            hands: [TakHand::new(size), TakHand::new(size)],
        }
    }

    fn get_hand_mut(&mut self, player: Player) -> &mut TakHand {
        match player {
            Player::White => &mut self.hands[0],
            Player::Black => &mut self.hands[1],
        }
    }

    pub fn from_ptn(size: usize, ptn: &str) -> Option<Self> {
        let parts = ptn.split_whitespace().collect::<Vec<_>>();
        let actions = parts
            .iter()
            .filter_map(|part| TakAction::from_ptn(part))
            .collect::<Vec<_>>();
        let mut game = Self::new(size);
        for action in actions {
            println!("{}", action.to_ptn());
            if game.try_play_action(action).is_err() {
                game.debug_print();
                return None; // Invalid action
            }
            println!("Success");
        }
        Some(game)
    }

    pub fn to_ptn(&self) -> String {
        self.actions
            .iter()
            .map(|action| action.to_ptn())
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn debug_print(&self) {
        for y in (0..self.size).rev() {
            for x in 0..self.size {
                let pos = TakCoord::new(x, y);
                match self.get_tile(&pos) {
                    Some(tower) => {
                        print!("{:?}{:?} ", tower.top_type, tower.composition);
                    }
                    None => {
                        print!("_ ");
                    }
                }
            }
            println!();
        }
    }

    pub fn try_play_action(&mut self, action: TakAction) -> TakFeedback {
        match &action {
            TakAction::PlacePiece {
                position,
                piece_type,
            } => self.try_place_piece(position, piece_type),
            TakAction::MovePiece {
                from,
                direction,
                take,
                drops,
            } => self.try_move_piece(from, direction, *take, drops),
        }?;
        self.actions.push(action);
        self.current_player = match self.current_player {
            Player::White => Player::Black,
            Player::Black => Player::White,
        };
        Ok(())
    }

    fn get_tile(&self, position: &TakCoord) -> &TakTile {
        position.validate(self.size).unwrap();
        &self.board[position.y * self.size + position.x]
    }

    fn get_tile_mut(&mut self, position: &TakCoord) -> &mut TakTile {
        position.validate(self.size).unwrap();
        &mut self.board[position.y * self.size + position.x]
    }

    fn try_get_tile_mut(&mut self, position: &TakCoord) -> TakResult<&mut TakTile> {
        position.validate(self.size)?;
        Ok(&mut self.board[position.y * self.size + position.x])
    }

    pub fn try_get_tile(&self, position: &TakCoord) -> TakResult<&TakTile> {
        position.validate(self.size)?;
        Ok(&self.board[position.y * self.size + position.x])
    }

    pub fn try_get_tower(&self, position: &TakCoord) -> TakResult<&TakTower> {
        position.validate(self.size)?;
        self.board[position.y * self.size + position.x]
            .as_ref()
            .ok_or(TakInvalidAction::TileEmpty)
    }

    fn try_place_piece(&mut self, position: &TakCoord, piece_type: &TakPieceType) -> TakFeedback {
        let player = if self.actions.len() >= 2 {
            self.current_player
        } else {
            self.current_player.opponent()
        };
        let tile = self.try_get_tile(position)?;
        if let None = tile {
            self.get_hand_mut(player).try_take_stone()?;
            *self.get_tile_mut(position) = Some(TakTower {
                top_type: *piece_type,
                composition: vec![player],
            });
        } else {
            return Err(TakInvalidAction::TileOccupied);
        }
        Ok(())
    }

    fn try_move_piece(
        &mut self,
        from: &TakCoord,
        direction: &Direction,
        take: usize,
        drops: &Vec<usize>,
    ) -> TakFeedback {
        let from_tower = self.try_get_tower(from)?;
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
            || drop_len < 1
            || drop_sum != take
            || drops.iter().any(|&i| i < 1)
        {
            return Err(TakInvalidAction::InvalidAction);
        }
        let positions = from
            .try_get_positions(direction, drop_len, self.size)
            .ok_or(TakInvalidAction::InvalidAction)?;
        for i in 0..drop_len {
            if let Some(tower) = self.get_tile(&positions[i]) {
                if tower.top_type != TakPieceType::Flat {
                    let can_flatten = tower.top_type == TakPieceType::Wall
                        && from_top_type == TakPieceType::Capstone
                        && drops[i] == 1
                        && i == drop_len - 1;
                    if !can_flatten {
                        return Err(TakInvalidAction::InvalidAction);
                    }
                }
            }
        }

        let taken = {
            let from_tower = self.get_tile_mut(from);
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
        Ok(())
    }
}

pub fn test_read_tak_game() {
    let game = TakGame::from_ptn(
        6,
        "
1. a6 f1
2. d3 c4 
3. d4  d5 
4. c3  b3
5. c5 b4 
6. c2  b2
7. c1  Cd2 
8. b1  d1 
9. c6  d2< 
10. Cb5 d2 
11. e3 e2 
12. f3  a4
13. b5-  a3 
14. 2b4-  a2 
15. 3b3-  a1 
16. b1<  b4 
17. f2  2c2+ 
18. e5 e2+ 
19. a5  f4 
20. e4  3c3> 
21. c3  d6 
22. c1> e6 
23. f5 4d3+
24. 4b2+13  f6 
25. 4b4>  c2 
26. Sd3  b2 
27. e2  b5
28. b4  b1
29. b4+  b4 
30. c1  b6 
31. e2<  d6< 
32. c5+  b6> 
33. Sb6  Sd6 
34. b6>  Sb6 
35. 4c6-  e1 
36. c1+  d6< 
37. d3> d6 
38. 3e3-12 b6- 
39. c1 b6 
40. 5c4< c4
    ",
    )
    .unwrap();
    game.debug_print();
    println!("PTN: {}", game.to_ptn());
}

pub fn test_tak_game() {
    let mut game = TakGame::new(5);
    game.try_play_action(TakAction::PlacePiece {
        position: TakCoord::new(0, 0),
        piece_type: TakPieceType::Flat,
    })
    .unwrap();
    game.try_play_action(TakAction::PlacePiece {
        position: TakCoord::new(1, 1),
        piece_type: TakPieceType::Flat,
    })
    .unwrap();
    game.try_play_action(TakAction::MovePiece {
        from: TakCoord::new(1, 1),
        direction: Direction::Left,
        take: 1,
        drops: vec![1],
    })
    .unwrap();
    game.try_play_action(TakAction::MovePiece {
        from: TakCoord::new(0, 0),
        direction: Direction::Up,
        take: 1,
        drops: vec![1],
    })
    .unwrap();
    game.try_play_action(TakAction::PlacePiece {
        position: TakCoord::new(1, 1),
        piece_type: TakPieceType::Flat,
    })
    .unwrap();
    game.try_play_action(TakAction::MovePiece {
        from: TakCoord::new(0, 1),
        direction: Direction::Right,
        take: 2,
        drops: vec![1, 1],
    })
    .unwrap();

    game.debug_print();
    println!("PTN: {}", game.to_ptn());
}
