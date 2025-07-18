use crate::Action;

/// [([[<player>]; <height>], [<variant excluding flat>]); <pos>]
pub type ZobristTable = [([[u64; 2]; 64], [u64; 2]); 64];

include!(concat!(env!("OUT_DIR"), "/zobrist.rs"));

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TranspositionNodeType {
    Exact,
    LowerBound,
    UpperBound,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TranspositionEntry {
    pub zobrist: u64,
    pub score: i32,
    pub depth: usize,
    pub node_type: TranspositionNodeType,
    pub best_move: Option<Action>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TranspositionTable {
    pub size: usize,
    pub entries: Vec<Option<TranspositionEntry>>,
}

impl TranspositionTable {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            entries: vec![None; 1 << size],
        }
    }

    pub fn insert(&mut self, entry: TranspositionEntry) {
        let index = (entry.zobrist % (1 << self.size)) as usize;
        self.entries[index] = Some(entry);
    }

    pub fn get(&self, zobrist: u64) -> Option<&TranspositionEntry> {
        let index = (zobrist % (1 << self.size)) as usize;
        self.entries[index]
            .as_ref()
            .filter(|e| e.zobrist == zobrist)
    }

    pub fn clear(&mut self) {
        self.entries = vec![None; 1 << self.size];
    }
}
