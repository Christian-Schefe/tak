use std::{cell::RefCell, sync::LazyLock};

use crate::Action;

/// ([([[<player>]; <height>], [<variant excluding flat>]); <pos>], [<player>])
pub type ZobristTable = ([([[u64; 2]; 64], [u64; 2]); 64], [u64; 2]);

include!(concat!(env!("OUT_DIR"), "/zobrist.rs"));

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TranspositionNodeType {
    Exact,
    Alpha,
    Beta,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TranspositionEntry {
    pub zobrist: u64,
    pub score: i32,
    pub depth: usize,
    pub ply: usize,
    pub node_type: TranspositionNodeType,
    pub best_move: Option<Action>,
}

thread_local! {
    pub static TRANSPOSITION_TABLE: LazyLock<RefCell<TranspositionTable>> =
    LazyLock::new(|| RefCell::new(TranspositionTable::new(20)));
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

    fn index(&self, zobrist: u64) -> usize {
        (zobrist & ((1 << self.size) - 1)) as usize
    }

    pub fn maybe_insert(&mut self, entry: TranspositionEntry) {
        let index = self.index(entry.zobrist);
        if let Some(existing) = &self.entries[index] {
            if existing.depth >= entry.depth {
                if entry.zobrist == existing.zobrist {
                    return;
                } else if existing.ply >= entry.ply {
                    return;
                }
            }
        }
        self.entries[index] = Some(entry);
    }

    pub fn get(&self, zobrist: u64) -> Option<&TranspositionEntry> {
        let index = self.index(zobrist);
        self.entries[index]
            .as_ref()
            .filter(|e| e.zobrist == zobrist)
    }

    pub fn clear(&mut self) {
        self.entries = vec![None; 1 << self.size];
    }

    pub fn count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_some()).count()
    }
}
