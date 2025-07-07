use crate::tak::{Direction, TakCoord, TakPieceType};

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

#[derive(Clone, Debug, PartialEq)]
pub enum TakActionResult {
    PlacePiece {
        position: TakCoord,
        piece_type: TakPieceType,
    },
    MovePiece {
        from: TakCoord,
        direction: Direction,
        take: usize,
        drops: Vec<usize>,
        flattened: bool,
    },
}

impl TakAction {
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

impl TakActionResult {
    pub fn to_ptn(&self) -> String {
        match self {
            TakActionResult::PlacePiece {
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
            TakActionResult::MovePiece {
                from,
                direction,
                take,
                drops,
                flattened,
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
                let flattened_str = if *flattened { "*" } else { "" };
                format!(
                    "{}{}{}{}{}{}",
                    take_str, file, rank, direction_str, drops_str, flattened_str
                )
            }
        }
    }
}
