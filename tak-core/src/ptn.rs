use crate::{
    TakGameSettings, TakGameState, TakKomi, TakPlayer, TakStones, TakTimeMode, TakTps, TakWinReason,
};

#[derive(Debug, Clone, PartialEq)]
pub enum TakPtnAttr {
    Size(usize),
    Clock(usize, usize),
    Komi(usize, bool),
    Flats(usize),
    Caps(usize),
    TPS(TakTps),
    Unknown(String),
}

impl TakPtnAttr {
    pub fn to_str(&self) -> String {
        match self {
            TakPtnAttr::Size(size) => format!("[Size \"{}\"]", size),
            TakPtnAttr::Clock(time, increment) => {
                let mins = time / 60;
                let secs = time % 60;
                format!("[Clock \"{}:{} +{}\"]", mins, secs, increment)
            }
            TakPtnAttr::Komi(amount, tiebreak) => {
                if *tiebreak {
                    format!("[Komi \"{}.5\"]", amount)
                } else {
                    format!("[Komi \"{}\"]", amount)
                }
            }
            TakPtnAttr::Flats(flats) => format!("[Flats \"{}\"]", flats),
            TakPtnAttr::Caps(caps) => format!("[Caps \"{}\"]", caps),
            TakPtnAttr::TPS(attr) => format!("[TPS \"{}\"]", attr.to_string()),
            TakPtnAttr::Unknown(attr) => format!("[{}]", attr),
        }
    }

    pub fn from_str(str: &str) -> Option<Self> {
        if str.is_empty() || !str.starts_with('[') || !str.ends_with(']') {
            return None;
        }
        let patterns = ["Size", "Clock", "Komi", "Flats", "Caps", "TPS"];
        let mut matching = None;

        for pattern in patterns {
            if str.starts_with(&format!("[{} \"", pattern)) && str.ends_with("\"]") {
                let inner = &str[pattern.len() + 3..str.len() - 2];
                matching = Some((pattern, inner));
                break;
            }
        }
        if matching.is_none() {
            let inner = &str[1..str.len() - 1];
            return Some(TakPtnAttr::Unknown(inner.to_string()));
        }
        let (pattern, inner) = matching?;

        match pattern {
            "Size" => inner.parse::<usize>().ok().map(TakPtnAttr::Size),
            "Clock" => {
                let parts = inner
                    .split(|c| c == ':' || c == '+')
                    .map(str::trim)
                    .collect::<Vec<&str>>();
                if parts.len() != 3 {
                    return None;
                }
                let mins = parts[0].parse::<usize>().ok()?;
                let secs = parts[1].parse::<usize>().ok()?;
                let increment = parts[2].parse::<usize>().ok()?;
                Some(TakPtnAttr::Clock(mins * 60 + secs, increment))
            }
            "Komi" => {
                let num = inner.parse::<f32>().ok()?;
                let add_half = num.trunc() != num;
                let num = num.trunc() as usize;
                Some(TakPtnAttr::Komi(num, add_half))
            }
            "Flats" => inner.parse::<usize>().ok().map(TakPtnAttr::Flats),
            "Caps" => inner.parse::<usize>().ok().map(TakPtnAttr::Caps),
            "TPS" => TakTps::try_from_str(inner).map(TakPtnAttr::TPS),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TakPtn {
    pub attributes: Vec<TakPtnAttr>,
    pub turns: Vec<(usize, Option<String>, Option<String>)>,
    pub game_state: TakGameState,
}

impl TakPtn {
    pub fn new(turns: Vec<String>, first_turn_index: usize, game_state: TakGameState) -> Self {
        let mut turn_vec = if first_turn_index % 2 == 0 {
            Vec::new()
        } else {
            vec![None]
        };
        turn_vec.extend(turns.into_iter().map(Option::Some));
        let turns = turn_vec
            .chunks(2)
            .enumerate()
            .map(|(i, chunk)| {
                let white_turn = chunk.get(0).cloned().flatten();
                let black_turn = chunk.get(1).cloned().flatten();
                (i + first_turn_index / 2, white_turn, black_turn)
            })
            .collect();
        TakPtn {
            attributes: Vec::new(),
            turns,
            game_state,
        }
    }

    pub fn get_settings(&self) -> Option<TakGameSettings> {
        let mut size = None;
        let mut komi = None;
        let mut flats = None;
        let mut caps = None;
        let mut clock = None;
        let mut tps = None;
        for attr in &self.attributes {
            match attr {
                TakPtnAttr::Size(s) => size = Some(*s),
                TakPtnAttr::Komi(k, add_half) => komi = Some((*k, *add_half)),
                TakPtnAttr::Flats(f) => flats = Some(*f),
                TakPtnAttr::Caps(c) => caps = Some(*c),
                TakPtnAttr::Clock(time, increment) => clock = Some((*time, *increment)),
                TakPtnAttr::TPS(t) => tps = Some(t.clone()),
                TakPtnAttr::Unknown(_) => {}
            }
        }
        if size.is_some() && komi.is_some() {
            let time_mode = clock.map(|(time, increment)| TakTimeMode::new(time, increment));
            let mut stones = TakStones::from_size(size.unwrap());
            if let Some(flats) = flats {
                stones.stones = flats;
            }
            if let Some(caps) = caps {
                stones.capstones = caps;
            }
            if let Some(tps) = tps {
                return Some(TakGameSettings::new_with_position(
                    size.unwrap(),
                    tps,
                    Some(stones),
                    TakKomi::new(komi.unwrap().0, komi.unwrap().1),
                    time_mode,
                ));
            }
            Some(TakGameSettings::new(
                size.unwrap(),
                Some(stones),
                TakKomi::new(komi.unwrap().0, komi.unwrap().1),
                time_mode,
            ))
        } else {
            None
        }
    }

    pub fn to_str(&self) -> String {
        let mut result = String::new();
        self.attributes.iter().for_each(|attr| {
            result.push_str(&attr.to_str());
            result.push('\n');
        });
        for (i, white_turn, black_turn) in self.turns.iter() {
            result.push_str(&format!("{}.", i + 1));
            result.push_str(&format!(
                " {}",
                white_turn.as_ref().unwrap_or(&"--".to_string())
            ));
            if let Some(black_turn) = black_turn {
                result.push_str(&format!(" {}", black_turn));
            }
            if *i == self.turns.len() - 1 && self.game_state != TakGameState::Ongoing {
                result.push_str(&format!(
                    " {}",
                    match self.game_state {
                        TakGameState::Win(TakPlayer::White, TakWinReason::Road) =>
                            "R-0".to_string(),
                        TakGameState::Win(TakPlayer::Black, TakWinReason::Road) =>
                            "0-R".to_string(),
                        TakGameState::Win(TakPlayer::White, TakWinReason::Flat) =>
                            "F-0".to_string(),
                        TakGameState::Win(TakPlayer::Black, TakWinReason::Flat) =>
                            "0-F".to_string(),
                        TakGameState::Win(TakPlayer::White, TakWinReason::Timeout) =>
                            "1-0".to_string(),
                        TakGameState::Win(TakPlayer::Black, TakWinReason::Timeout) =>
                            "0-1".to_string(),
                        TakGameState::Draw => "1/2-1/2".to_string(),
                        TakGameState::Ongoing => unreachable!(),
                    }
                ));
            }
            result.push('\n');
        }
        result
    }

    pub fn try_from_str(s: &str) -> Option<Self> {
        let mut lines = s.lines();
        let mut attributes = Vec::new();
        let mut turns: Vec<(usize, Option<String>, Option<String>)> = Vec::new();
        let mut game_state = TakGameState::Ongoing;

        while let Some(line) = lines.next() {
            if line.starts_with('[') && line.ends_with(']') {
                if let Some(attr) = TakPtnAttr::from_str(line) {
                    attributes.push(attr);
                } else {
                    return None;
                }
            } else if !line.trim().is_empty() {
                let mut turn: Vec<String> = line.split_whitespace().map(String::from).collect();

                let maybe_result = turn.last().and_then(|x| match x.as_str() {
                    "R-0" => Some(TakGameState::Win(TakPlayer::White, TakWinReason::Road)),
                    "0-R" => Some(TakGameState::Win(TakPlayer::Black, TakWinReason::Road)),
                    "F-0" => Some(TakGameState::Win(TakPlayer::White, TakWinReason::Flat)),
                    "0-F" => Some(TakGameState::Win(TakPlayer::Black, TakWinReason::Flat)),
                    "1-0" => Some(TakGameState::Win(TakPlayer::White, TakWinReason::Timeout)),
                    "0-1" => Some(TakGameState::Win(TakPlayer::Black, TakWinReason::Timeout)),
                    "1/2-1/2" => Some(TakGameState::Draw),
                    _ => None,
                });
                if let Some(result) = maybe_result {
                    game_state = result;
                    turn.pop();
                }

                if turn.len() < 2 || turn.len() > 3 {
                    return None;
                }
                let turn_index = turn[0].trim_end_matches('.').parse::<usize>().ok()?;
                if turn_index == 0 {
                    return None;
                }
                if turns.len() > 0 && turns.last().unwrap().0 + 1 != turn_index - 1 {
                    return None;
                }
                let white_turn =
                    Some(turn[1].trim()).filter(|s| s.chars().any(|c| c != '.' && c != '-'));
                let black_turn = turn.get(2).map(|s| s.trim());

                turns.push((
                    turn_index - 1,
                    white_turn.map(|x| x.to_string()),
                    black_turn.map(|x| x.to_string()),
                ));
            }
        }

        Some(Self {
            attributes,
            turns,
            game_state,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ptn_attr_to_str() {
        let attr = TakPtnAttr::Size(5);
        assert_eq!(attr.to_str(), "[Size \"5\"]");

        let attr = TakPtnAttr::Clock(300, 10);
        assert_eq!(attr.to_str(), "[Clock \"5:0 +10\"]");

        let attr = TakPtnAttr::Komi(6, false);
        assert_eq!(attr.to_str(), "[Komi \"6\"]");

        let attr = TakPtnAttr::Flats(21);
        assert_eq!(attr.to_str(), "[Flats \"21\"]");

        let attr = TakPtnAttr::Caps(4);
        assert_eq!(attr.to_str(), "[Caps \"4\"]");

        let attr = TakPtnAttr::TPS(TakTps::new("x3/x3/x3".to_string(), 0));
        assert_eq!(attr.to_str(), "[TPS \"x3/x3/x3 1 1\"]");

        let attr = TakPtnAttr::TPS(TakTps::new("x3/x2,112C/x3".to_string(), 9));
        assert_eq!(attr.to_str(), "[TPS \"x3/x2,112C/x3 2 5\"]");

        let attr = TakPtnAttr::Unknown("Unknown".to_string());
        assert_eq!(attr.to_str(), "[Unknown]");
    }
}
