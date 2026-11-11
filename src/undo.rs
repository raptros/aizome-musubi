//! Observed transaction history; replay uses stable identities, never old global IDs.
use crate::app::Transaction;

#[derive(Default)]
pub struct History {
    undo: Vec<usize>,
    redo: Vec<usize>,
}

impl History {
    pub fn record(&mut self, index: usize) {
        self.undo.push(index);
        self.redo.clear();
    }

    pub fn undo(&mut self) -> Option<usize> {
        self.undo.pop()
    }

    pub fn redo(&mut self) -> Option<usize> {
        self.redo.pop()
    }

    pub fn after_undo(&mut self, replay: Option<usize>) {
        if let Some(index) = replay {
            self.redo.push(index);
        }
    }

    pub fn after_redo(&mut self, replay: Option<usize>) {
        if let Some(index) = replay {
            self.undo.push(index);
        }
    }

    pub fn undo_label(&self, transactions: &[Transaction]) -> Option<&'static str> {
        self.undo.last().and_then(|&index| transactions.get(index)).map(Transaction::label)
    }

    pub fn redo_label(&self, transactions: &[Transaction]) -> Option<&'static str> {
        self.redo.last().and_then(|&index| transactions.get(index)).map(Transaction::label)
    }
}
