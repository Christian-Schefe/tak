use crate::TakPlayer;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TakTps {
    pub position: String,
    pub player: TakPlayer,
    pub move_index: usize,
}

impl TakTps {
    pub fn new(position: String, ply_index: usize) -> Self {
        let player = if ply_index % 2 == 0 {
            TakPlayer::White
        } else {
            TakPlayer::Black
        };
        let move_index = ply_index / 2;
        TakTps {
            position,
            player,
            move_index,
        }
    }

    pub fn get_ply_index(&self) -> usize {
        self.move_index * 2
            + if self.player == TakPlayer::White {
                0
            } else {
                1
            }
    }

    pub fn new_empty(size: usize) -> Self {
        TakTps {
            position: vec![format!("x{}", size); size].join("/"),
            player: TakPlayer::White,
            move_index: 0,
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
            self.move_index + 1
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
        let move_index = parts[2].parse::<usize>().ok()?;
        if move_index == 0 {
            return None;
        }
        Some(TakTps {
            position,
            player,
            move_index: move_index - 1,
        })
    }
}

#[cfg(test)]
mod tests {
    pub use super::*;

    #[test]
    fn test_new_tps() {
        let tps = TakTps::new("x3/x3/x3".to_string(), 0);
        assert_eq!(tps.position, "x3/x3/x3");
        assert_eq!(tps.player, TakPlayer::White);
        assert_eq!(tps.move_index, 0);

        let tps = TakTps::new("x2/x2/x2".to_string(), 1);
        assert_eq!(tps.position, "x2/x2/x2");
        assert_eq!(tps.player, TakPlayer::Black);
        assert_eq!(tps.move_index, 0);

        let tps = TakTps::new("x4/x4/x4".to_string(), 18);
        assert_eq!(tps.position, "x4/x4/x4");
        assert_eq!(tps.player, TakPlayer::White);
        assert_eq!(tps.move_index, 9);
    }

    #[test]
    fn test_tps_to_string() {
        let tps = TakTps::new("x3/x3/x3".to_string(), 0);
        assert_eq!(tps.to_string(), "x3/x3/x3 1 1");

        let tps = TakTps::new("x3/x2,112C/x3".to_string(), 8);
        assert_eq!(tps.to_string(), "x3/x2,112C/x3 1 5");
    }

    #[test]
    fn test_tps_from_str() {
        let tps = TakTps::try_from_str("x3/x3/x3 1 1");
        assert_eq!(
            tps,
            Some(TakTps {
                position: "x3/x3/x3".to_string(),
                player: TakPlayer::White,
                move_index: 0
            })
        );

        let tps = TakTps::try_from_str("x3/x2,112C/x3 2 5");
        assert_eq!(
            tps,
            Some(TakTps {
                position: "x3/x2,112C/x3".to_string(),
                player: TakPlayer::Black,
                move_index: 4
            })
        );

        let invalid_tps = TakTps::try_from_str("x3/x2,112C/x3");
        assert!(invalid_tps.is_none());

        let invalid_tps = TakTps::try_from_str("x3/x2,112C/x3 1 a");
        assert!(invalid_tps.is_none());

        let invalid_tps = TakTps::try_from_str("x3/x2,112C/x3 a 1");
        assert!(invalid_tps.is_none());

        let invalid_tps = TakTps::try_from_str("x3/x2,112C/x3 1  ");
        assert!(invalid_tps.is_none());
    }
}
