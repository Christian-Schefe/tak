use crate::TakPlayer;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TakTps {
    pub position: String,
    pub player: TakPlayer,
    pub turn_index: usize,
}

impl TakTps {
    pub fn new(position: String, player: TakPlayer, turn_index: usize) -> Self {
        TakTps {
            position,
            player,
            turn_index,
        }
    }

    pub fn new_empty(size: usize) -> Self {
        TakTps {
            position: vec![format!("x{}", size); size].join("/"),
            player: TakPlayer::White,
            turn_index: 0,
        }
    }

    pub fn to_string(&self) -> String {
        format!(
            "{} {} {}",
            self.position,
            match self.player {
                TakPlayer::White => '1',
                TakPlayer::Black => '2',
            },
            self.turn_index + 1
        )
    }

    pub fn try_from_str(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() != 3 {
            return None;
        }
        let position = parts[0].to_string();
        let player = match parts[1] {
            "1" => TakPlayer::White,
            "2" => TakPlayer::Black,
            _ => return None,
        };
        let turn = parts[2].parse::<usize>().ok()?;
        if turn == 0 {
            return None; // Turn index should start from 1
        }
        Some(TakTps {
            position,
            player,
            turn_index: turn - 1,
        })
    }
}
