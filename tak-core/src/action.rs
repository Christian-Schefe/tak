use crate::{TakCoord, TakDir, TakPieceVariant, TakPlayer};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

impl TakActionRecord {
    pub fn to_ptn(&self) -> String {
        match self {
            Self::PlacePiece {
                pos,
                variant,
                player: _,
            } => {
                let prefix = match variant {
                    TakPieceVariant::Flat => "",
                    TakPieceVariant::Wall => "S",
                    TakPieceVariant::Capstone => "C",
                };
                let file = (b'a' + pos.x as u8) as char;
                let rank = pos.y + 1;
                format!("{}{}{}", prefix, file, rank)
            }
            Self::MovePiece {
                pos,
                dir,
                take,
                drops,
                flattened,
            } => {
                let take_str = if *take == 1 {
                    String::new()
                } else {
                    format!("{}", take)
                };
                let file = (b'a' + pos.x as u8) as char;
                let rank = pos.y + 1;
                let direction_str = match dir {
                    TakDir::Up => "+",
                    TakDir::Down => "-",
                    TakDir::Left => "<",
                    TakDir::Right => ">",
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

impl TakAction {
    fn parse_place(size: i32, input: &str) -> Option<TakAction> {
        let chars: Vec<char> = input.chars().collect();
        let (variant, file, rank) = match chars.as_slice() {
            [prefix @ ('S' | 'C'), file @ 'a'..='z', rank @ '1'..='9'] => (
                match prefix {
                    'S' => TakPieceVariant::Wall,
                    'C' => TakPieceVariant::Capstone,
                    _ => unreachable!(),
                },
                *file,
                *rank,
            ),
            // Without prefix
            [file @ 'a'..='z', rank @ '1'..='9'] => (TakPieceVariant::Flat, *file, *rank),
            _ => return None,
        };
        let file_index = (file as u8 - b'a') as i32;
        let rank_index = size - 1 - (rank as u8 - b'1') as i32;
        Some(TakAction::PlacePiece {
            pos: TakCoord::new(file_index, rank_index),
            variant,
        })
    }

    fn parse_move(size: i32, input: &str) -> Option<TakAction> {
        let mut chars = input.chars().peekable();

        let take = if let Some(c @ '1'..='9') = chars.peek().copied() {
            chars.next();
            (c as u8 - b'0') as usize
        } else {
            1
        };

        let file = match chars.next()? {
            c @ 'a'..='z' => c as u8 - b'a',
            _ => return None,
        } as i32;

        let rank = size
            - 1
            - match chars.next()? {
                c @ '1'..='9' => c as u8 - b'1',
                _ => return None,
            } as i32;

        let dir = match chars.next()? {
            '+' => TakDir::Up,
            '-' => TakDir::Down,
            '<' => TakDir::Left,
            '>' => TakDir::Right,
            _ => return None,
        };

        let mut drops = Vec::new();
        while let Some(c @ '1'..='9') = chars.peek().copied() {
            drops.push((c as u8 - b'0') as usize);
            chars.next();
        }
        if drops.is_empty() {
            drops.push(take);
        }

        if let Some('*') = chars.peek().copied() {
            chars.next();
        }

        if chars.next().is_some() {
            return None;
        }

        Some(TakAction::MovePiece {
            pos: TakCoord::new(file, rank),
            dir,
            take,
            drops,
        })
    }

    pub fn from_ptn(size: i32, ptn: &str) -> Option<Self> {
        if ptn.is_empty() {
            return None;
        }
        if let Some(action) = Self::parse_place(size, ptn) {
            return Some(action);
        }
        Self::parse_move(size, ptn)
    }
}
