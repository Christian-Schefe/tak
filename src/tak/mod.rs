#[derive(Clone, Debug, PartialEq)]
pub struct TakGame {
    pub size: usize,
    pub board: Vec<TakTile>,
    pub current_player: Player,
    pub actions: Vec<TakAction>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TakHand {
    player: Player,
    stones: usize,
    capstones: usize,
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

#[derive(Clone, Debug, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn try_get_positions(
        &self,
        position: &(usize, usize),
        times: usize,
        size: usize,
    ) -> Option<Vec<(usize, usize)>> {
        match self {
            Direction::Left => {
                if position.0 >= times {
                    Some((1..=times).map(|i| (position.0 - i, position.1)).collect())
                } else {
                    None
                }
            }
            Direction::Right => {
                if position.0 + times < size {
                    Some((1..=times).map(|i| (position.0 + i, position.1)).collect())
                } else {
                    None
                }
            }
            Direction::Down => {
                if position.1 >= times {
                    Some((1..=times).map(|i| (position.0, position.1 - i)).collect())
                } else {
                    None
                }
            }
            Direction::Up => {
                if position.1 + times < size {
                    Some((1..=times).map(|i| (position.0, position.1 + i)).collect())
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TakAction {
    PlacePiece {
        position: (usize, usize),
        piece_type: TakPieceType,
    },
    MovePiece {
        from: (usize, usize),
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
                let file = (b'a' + position.0 as u8) as char;
                let rank = position.1 + 1;
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
                let file = (b'a' + from.0 as u8) as char;
                let rank = from.1 + 1;
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
                position: (file, rank),
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
                from: (file, rank),
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
            if game.try_play_action(action).is_none() {
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
                let pos = (x, y);
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

    pub fn try_play_action(&mut self, action: TakAction) -> Option<()> {
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
        Some(())
    }

    pub fn validate_position(&self, position: &(usize, usize)) -> Option<()> {
        if position.0 < self.size && position.1 < self.size {
            Some(())
        } else {
            None
        }
    }

    fn get_tile(&self, position: &(usize, usize)) -> &TakTile {
        self.validate_position(position).unwrap();
        &self.board[position.0 * self.size + position.1]
    }

    fn get_tile_mut(&mut self, position: &(usize, usize)) -> &mut TakTile {
        self.validate_position(position).unwrap();
        &mut self.board[position.0 * self.size + position.1]
    }

    fn try_get_tile_mut(&mut self, position: &(usize, usize)) -> Option<&mut TakTile> {
        self.validate_position(position)?;
        Some(&mut self.board[position.0 * self.size + position.1])
    }

    fn try_get_tile(&self, position: &(usize, usize)) -> Option<&TakTile> {
        self.validate_position(position)?;
        Some(&self.board[position.0 * self.size + position.1])
    }

    fn try_get_tower(&self, position: &(usize, usize)) -> Option<&TakTower> {
        self.validate_position(position)?;
        self.board[position.0 * self.size + position.1].as_ref()
    }

    fn try_place_piece(
        &mut self,
        position: &(usize, usize),
        piece_type: &TakPieceType,
    ) -> Option<()> {
        let player = if self.actions.len() >= 2 {
            self.current_player
        } else {
            self.current_player.opponent()
        };
        let tile = self.try_get_tile_mut(position)?;
        if let None = tile {
            *tile = Some(TakTower {
                top_type: *piece_type,
                composition: vec![player],
            });
        } else {
            return None;
        }
        Some(())
    }

    fn try_move_piece(
        &mut self,
        from: &(usize, usize),
        direction: &Direction,
        take: usize,
        drops: &Vec<usize>,
    ) -> Option<()> {
        let from_tower = self.try_get_tower(from)?;
        let from_top_type = from_tower.top_type;
        let from_composition_len = from_tower.composition.len();
        if from_tower.composition[from_composition_len - 1] != self.current_player {
            return None;
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
            return None;
        }
        let positions = direction.try_get_positions(from, drop_len, self.size)?;
        for i in 0..drop_len {
            if let Some(tower) = self.get_tile(&positions[i]) {
                if tower.top_type != TakPieceType::Flat {
                    let can_flatten = tower.top_type == TakPieceType::Wall
                        && from_top_type == TakPieceType::Capstone
                        && drops[i] == 1
                        && i == drop_len - 1;
                    if !can_flatten {
                        return None;
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
        Some(())
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
        position: (0, 0),
        piece_type: TakPieceType::Flat,
    })
    .unwrap();
    game.try_play_action(TakAction::PlacePiece {
        position: (1, 1),
        piece_type: TakPieceType::Flat,
    })
    .unwrap();
    game.try_play_action(TakAction::MovePiece {
        from: (1, 1),
        direction: Direction::Left,
        take: 1,
        drops: vec![1],
    })
    .unwrap();
    game.try_play_action(TakAction::MovePiece {
        from: (0, 0),
        direction: Direction::Up,
        take: 1,
        drops: vec![1],
    })
    .unwrap();
    game.try_play_action(TakAction::PlacePiece {
        position: (1, 1),
        piece_type: TakPieceType::Flat,
    });
    game.try_play_action(TakAction::MovePiece {
        from: (0, 1),
        direction: Direction::Right,
        take: 2,
        drops: vec![1, 1],
    })
    .unwrap();

    game.debug_print();
    println!("PTN: {}", game.to_ptn());
}
