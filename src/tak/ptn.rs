use crate::tak::{TakKomi, TakSettings, TakStones, TakTimeMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtnAttribute {
    Size(usize),
    Clock(usize, usize),
    Komi(usize, bool),
    Flats(usize),
    Caps(usize),
}

impl PtnAttribute {
    pub fn to_str(&self) -> String {
        match self {
            PtnAttribute::Size(size) => format!("[Size \"{}\"]", size),
            PtnAttribute::Clock(time, increment) => {
                let mins = time / 60;
                let secs = time % 60;
                format!("[Clock \"{}:{} +{}\"]", mins, secs, increment)
            }
            PtnAttribute::Komi(komi, add_half) => {
                if *add_half {
                    format!("[Komi \"{}.5\"]", komi)
                } else {
                    format!("[Komi \"{}\"]", komi)
                }
            }
            PtnAttribute::Flats(flats) => format!("[Flats \"{}\"]", flats),
            PtnAttribute::Caps(caps) => format!("[Caps \"{}\"]", caps),
        }
    }

    pub fn from_str(str: &str) -> Option<Self> {
        let parts: Vec<&str> = str
            .trim_matches(|c| c == '[' || c == ']')
            .split_whitespace()
            .collect();
        if parts.is_empty() {
            return None;
        }

        match parts[0] {
            "Size" => parts
                .get(1)
                .and_then(|s| s.trim_matches(|c| c == '"').parse::<usize>().ok())
                .map(PtnAttribute::Size),
            "Clock" => {
                if parts.len() < 3 {
                    return None;
                }
                let time_parts: Vec<&str> =
                    parts[1].trim_matches(|c| c == '"').split(':').collect();
                if time_parts.len() != 2 {
                    return None;
                }
                let mins = time_parts[0].parse::<usize>().ok()?;
                let secs = time_parts[1].parse::<usize>().ok()?;
                let increment = parts[2]
                    .trim_matches(|c| c == '"' || c == '+')
                    .parse::<usize>()
                    .ok()?;
                Some(PtnAttribute::Clock(mins * 60 + secs, increment))
            }
            "Komi" => {
                if parts.len() < 2 {
                    return None;
                }
                let num = parts[1].trim_matches(|c| c == '"').parse::<f32>().ok()?;
                let add_half = num.trunc() != num;
                let num = num.trunc() as usize;
                Some(PtnAttribute::Komi(num, add_half))
            }
            "Flats" => parts
                .get(1)
                .and_then(|s| s.trim_matches(|c| c == '"').parse::<usize>().ok())
                .map(PtnAttribute::Flats),
            "Caps" => parts
                .get(1)
                .and_then(|s| s.trim_matches(|c| c == '"').parse::<usize>().ok())
                .map(PtnAttribute::Caps),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ptn {
    pub attributes: Vec<PtnAttribute>,
    pub turns: Vec<Vec<String>>,
}

impl Ptn {
    pub fn get_settings(&self) -> Option<TakSettings> {
        let mut size = None;
        let mut komi = None;
        let mut flats = None;
        let mut caps = None;
        let mut clock = None;
        for attr in &self.attributes {
            match attr {
                PtnAttribute::Size(s) => size = Some(*s),
                PtnAttribute::Komi(k, add_half) => komi = Some((*k, *add_half)),
                PtnAttribute::Flats(f) => flats = Some(*f),
                PtnAttribute::Caps(c) => caps = Some(*c),
                PtnAttribute::Clock(time, increment) => clock = Some((*time, *increment)),
            }
        }
        if size.is_some() && komi.is_some() && flats.is_some() && caps.is_some() && clock.is_some()
        {
            Some(TakSettings {
                size: size.unwrap(),
                komi: TakKomi {
                    whole: komi.unwrap().0,
                    half: komi.unwrap().1,
                },
                stones: TakStones {
                    stones: flats.unwrap(),
                    capstones: caps.unwrap(),
                },
                time_mode: TakTimeMode {
                    time_limit: clock.unwrap().0,
                    time_increment: clock.unwrap().1,
                },
            })
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
        self.turns.iter().enumerate().for_each(|(i, turn)| {
            result.push_str(&format!("{}.", i + 1));
            turn.iter().for_each(|mv| {
                result.push_str(&format!(" {}", mv));
            });
            result.push('\n');
        });
        result
    }

    pub fn from_str(s: &str) -> Option<Self> {
        let mut lines = s.lines();
        let mut attributes = Vec::new();
        let mut turns = Vec::new();

        while let Some(line) = lines.next() {
            dioxus::logger::tracing::info!("Parsing line: {}", line);
            if line.starts_with('[') && line.ends_with(']') {
                if let Some(attr) = PtnAttribute::from_str(line) {
                    attributes.push(attr);
                } else {
                    return None;
                }
            } else if !line.trim().is_empty() {
                let turn: Vec<String> = line
                    .split_whitespace()
                    .filter(|s| {
                        !(s[..s.len() - 1].chars().all(|c| c.is_digit(10)) && s.ends_with('.'))
                    })
                    .map(String::from)
                    .collect();
                if turn.len() > 2 {
                    return None;
                }
                turns.push(turn.try_into().ok()?);
            }
        }

        Some(Self { attributes, turns })
    }
}
