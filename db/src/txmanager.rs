use std::collections::{BTreeMap, BTreeSet};

use crate::dbcore::{DbValue, IsolationLevel, TransactionState};

#[derive(Debug)]
pub struct Transaction {
    id: usize,
    isolation: IsolationLevel,
    state: TransactionState,
    in_progress: BTreeSet<usize>,
    writes: BTreeSet<String>,
    reads: BTreeSet<String>,
}

impl Transaction {
    pub fn new(id: usize, isolation: IsolationLevel) -> Transaction {
        Transaction {
            id,
            isolation,
            state: TransactionState::InProgress,
            in_progress: BTreeSet::new(),
            writes: BTreeSet::new(),
            reads: BTreeSet::new(),
        }
    }

    #[cfg(test)]
    pub fn get_state(&self) -> &TransactionState {
        &self.state
    }
}

#[derive(PartialEq)]
pub enum TransactionProcessingError {
    SerializableError,
}

pub struct TransactionManager {
    transactions: BTreeMap<usize, Transaction>,
    next_transaction_id: usize,
}

impl Default for TransactionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionManager {
    pub fn new() -> TransactionManager {
        TransactionManager {
            transactions: BTreeMap::new(),
            next_transaction_id: 1,
        }
    }

    pub fn new_transaction(&mut self, isolation: IsolationLevel) -> usize {
        let tx_id = self.next_transaction_id;
        self.next_transaction_id += 1;

        let mut tx = Transaction::new(tx_id, isolation);
        tx.in_progress = self.in_progress();
        self.transactions.insert(tx_id, tx);
        tx_id
    }

    pub fn complete_transaction(
        &mut self,
        id: usize,
        state: TransactionState,
    ) -> Result<(), TransactionProcessingError> {
        if state == TransactionState::InProgress {
            panic!("Illegal state in complete_transaction")
        }
        {
            let tx = self.transactions.get(&id).unwrap();
            if tx.isolation == IsolationLevel::Serializable
                && state == TransactionState::Committed
                && self.has_conflict(tx, |t1, t2| -> bool {
                    !t1.writes.is_disjoint(&t2.reads) || !t1.reads.is_disjoint(&t2.writes)
                })
            {
                return Err(TransactionProcessingError::SerializableError);
            }
        }

        let tx_mut = self.transactions.get_mut(&id).unwrap();

        tx_mut.state = state;
        Ok(())
    }

    pub fn add_to_read_set(&mut self, id: usize, ids: Vec<String>) {
        let tx = self.transactions.get_mut(&id).unwrap();
        for i in ids {
            tx.reads.insert(i);
        }
    }

    pub fn add_to_write_set(&mut self, id: usize, ids: Vec<String>) {
        let tx = self.transactions.get_mut(&id).unwrap();
        for i in ids {
            tx.writes.insert(i);
        }
    }

    pub fn is_visible(&self, id: usize, db_value: &DbValue) -> bool {
        let tx = self.transactions.get(&id).unwrap();

        match tx.isolation {
            IsolationLevel::ReadUncommitted => {
                db_value.tx_end == 0
            }
            IsolationLevel::ReadCommitted => {
                if db_value.tx_start != id
                    && self.transactions.get(&db_value.tx_start).unwrap().state
                        != TransactionState::Committed
                {
                    return false;
                }

                if db_value.tx_end == id {
                    return false;
                }

                if db_value.tx_end > 0
                    && self.transactions.get(&db_value.tx_end).unwrap().state
                        == TransactionState::Committed
                {
                    return false;
                }

                true
            }
            IsolationLevel::RepeatableRead | IsolationLevel::Serializable => {
                if db_value.tx_start > tx.id {
                    return false;
                }

                if tx.in_progress.contains(&db_value.tx_start) {
                    return false;
                }

                if self.transactions.get(&db_value.tx_start).unwrap().state
                    != TransactionState::Committed
                    && db_value.tx_start != tx.id
                {
                    return false;
                }

                if db_value.tx_end == tx.id {
                    return false;
                }

                if db_value.tx_end < tx.id
                    && db_value.tx_end > 0
                    && self.transactions.get(&db_value.tx_end).unwrap().state
                        == TransactionState::Committed
                    && !tx.in_progress.contains(&db_value.tx_end)
                {
                    return false;
                }

                true
            }
        }
    }

    fn in_progress(&self) -> BTreeSet<usize> {
        self.transactions
            .iter()
            .filter(|(_, t)| -> bool { t.state == TransactionState::InProgress })
            .map(|(id, _)| -> usize { *id })
            .collect()
    }

    fn has_conflict(
        &self,
        tx: &Transaction,
        conflict_fn: fn(&Transaction, &Transaction) -> bool,
    ) -> bool {
        let iter = self.transactions.values();

        let any_in_progress_has_conflict = tx.in_progress.iter().any(|t_id| -> bool {
            let t = self.transactions.get(t_id).unwrap();
            conflict_fn(tx, t)
        });

        if any_in_progress_has_conflict {
            return true;
        }

        iter
            .filter(|t| -> bool { t.id > tx.id })
            .filter(|t| -> bool { t.state == TransactionState::Committed })
            .any(|t| -> bool { conflict_fn(tx, t) })
    }

    #[cfg(test)]
    pub fn get_transactions(&mut self) -> &mut BTreeMap<usize, Transaction> {
        &mut self.transactions
    }
}
