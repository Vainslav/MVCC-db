use std::sync::{Arc, RwLock};

use crate::{
    dbcore::{DbValue, IsolationLevel, TransactionState},
    storage::Storage,
    txmanager::{TransactionManager, TransactionProcessingError},
};

pub enum Command {
    Put(String, String),
    Get(String),
    Delete(String),
    Begin(IsolationLevel),
    Commit,
    Abort,
}

#[derive(Debug, PartialEq)]
pub enum CommandExecutionError {
    Todo,
    NotFound,
    NoneVisible,
    NoActiveTransaction,
    TransactionAlreadyActive,
    SerializationError
}

impl From<TransactionProcessingError> for CommandExecutionError {
    fn from(value: TransactionProcessingError) -> Self {
        match value {
            TransactionProcessingError::Value => CommandExecutionError::SerializationError,
        }
    }
}

pub struct Connection {
    cur_tx: Option<usize>,
    store: Arc<RwLock<Storage>>,
    tx_manager: Arc<RwLock<TransactionManager>>,
}

impl Connection {
    pub fn new(
        store: Arc<RwLock<Storage>>,
        tx_manager: Arc<RwLock<TransactionManager>>,
    ) -> Connection {
        Connection {
            cur_tx: None,
            store,
            tx_manager,
        }
    }

    pub fn begin(&mut self, isolation: IsolationLevel) -> Result<String, CommandExecutionError> {
        if self.cur_tx.is_some() {
            return Err(CommandExecutionError::TransactionAlreadyActive);
        }

        let mut tx_manager = self.tx_manager.write().unwrap();
        self.cur_tx = Some(tx_manager.new_transaction(isolation));

        Ok(String::new())
    }

    pub fn commit(&mut self) -> Result<String, CommandExecutionError> {
        if self.cur_tx.is_none() {
            return Err(CommandExecutionError::NoActiveTransaction);
        }

        let mut tx_manager = self.tx_manager.write().unwrap();
        tx_manager.complete_transaction(self.cur_tx.unwrap(), TransactionState::Committed)?;
        self.cur_tx = None;

        Ok(String::new())
    }

    pub fn abort(&mut self) -> Result<String, CommandExecutionError> {
        if self.cur_tx.is_none() {
            return Err(CommandExecutionError::NoActiveTransaction);
        }

        let mut tx_manager = self.tx_manager.write().unwrap();
        tx_manager.complete_transaction(self.cur_tx.unwrap(), TransactionState::Aborted);
        self.cur_tx = None;

        Ok(String::new())
    }

    pub fn put(&mut self, id: String, value: String) -> Result<String, CommandExecutionError> {
        let cur_tx = self.current_tx()?;
        self.tx_manager
            .write()
            .unwrap()
            .add_to_write_set(cur_tx, vec![id.clone()]);

        let tx_manager_read = self.tx_manager.read().unwrap();
        let mut storage_read = self.store.write().unwrap();
        let data = &mut storage_read.data;

        if let Some(values) = data.get_mut(&id) {
            for val in values.iter_mut().rev() {
                if tx_manager_read.is_visible(cur_tx, &val) {
                    val.tx_end = cur_tx;
                }
            }

            values.push(DbValue::new(cur_tx, 0, value));
        } else {
            data.insert(id, vec![DbValue::new(cur_tx, 0, value)]);
        };

        Ok(String::new())
    }

    pub fn get(&mut self, id: String) -> Result<String, CommandExecutionError> {
        let cur_tx = self.current_tx()?;
        self.tx_manager
            .write()
            .unwrap()
            .add_to_read_set(cur_tx, vec![id.clone()]);

        let tx_manager_read = self.tx_manager.read().unwrap();
        let storage_read = self.store.read().unwrap();
        let data = &storage_read.data;

        let Some(values) = data.get(&id) else {
            return Err(CommandExecutionError::NotFound);
        };

        for val in values.iter().rev() {
            if tx_manager_read.is_visible(cur_tx, &val) {
                return Ok(val.value.clone());
            }
        }
        Err(CommandExecutionError::NoneVisible)
    }

    pub fn delete(&mut self, id: String) -> Result<String, CommandExecutionError> {
        let cur_tx = self.current_tx()?;
        self.tx_manager
            .write()
            .unwrap()
            .add_to_write_set(cur_tx, vec![id.clone()]);

        let tx_manager_read = self.tx_manager.read().unwrap();
        let mut storage_read = self.store.write().unwrap();
        let data = &mut storage_read.data;

        if let Some(values) = data.get_mut(&id) {
            let mut found = false;
            for val in values.iter_mut().rev() {
                if tx_manager_read.is_visible(cur_tx, &val) {
                    val.tx_end = cur_tx;
                    found = true;
                }
            }

            if found {
                Ok(String::new())
            } else {
                Err(CommandExecutionError::NoneVisible)
            }
        } else {
            Err(CommandExecutionError::NotFound)
        }
    }

    fn current_tx(&self) -> Result<usize, CommandExecutionError> {
        self.cur_tx
            .ok_or(CommandExecutionError::NoActiveTransaction)
    }

    #[cfg(test)]
    pub fn get_tx_manager(&mut self) -> &mut Arc<RwLock<TransactionManager>> {
        &mut self.tx_manager
    }
}

pub fn execute_command(
    con: &mut Connection,
    command: Command,
) -> Result<String, CommandExecutionError> {
    match command {
        Command::Put(id, value) => con.put(id, value),
        Command::Get(id) => con.get(id),
        Command::Delete(id) => con.delete(id),
        Command::Begin(isolation) => con.begin(isolation),
        Command::Commit => con.commit(),
        Command::Abort => con.abort(),
    }
}

#[cfg(test)]
mod command_tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn test_begin() {
        let store = Arc::new(RwLock::new(Storage::new()));
        let tx_manager = Arc::new(RwLock::new(TransactionManager::new()));

        let mut con = Connection::new(store, tx_manager);
        execute_command(&mut con, Command::Begin(IsolationLevel::ReadUncommitted)).unwrap();

        assert!(
            execute_command(&mut con, Command::Begin(IsolationLevel::ReadUncommitted)).is_err()
        );
        assert!(con.cur_tx.is_some());
        assert_eq!(con.cur_tx.unwrap(), 1);
    }

    #[test]
    fn test_commit() {
        let store = Arc::new(RwLock::new(Storage::new()));
        let tx_manager = Arc::new(RwLock::new(TransactionManager::new()));

        let mut con = Connection::new(store, tx_manager);
        assert!(execute_command(&mut con, Command::Commit).is_err());

        execute_command(&mut con, Command::Begin(IsolationLevel::ReadUncommitted)).unwrap();

        assert!(execute_command(&mut con, Command::Commit).is_ok());
        assert!(con.cur_tx.is_none());

        let mut tx_m = con.get_tx_manager().write().unwrap();
        let txs = tx_m.get_transactions();
        assert!(txs.len() == 1);

        let my_tx = txs.get(&1).unwrap();
        assert!(*my_tx.get_state() == TransactionState::Committed);
    }

    #[test]
    fn test_abort() {
        let store = Arc::new(RwLock::new(Storage::new()));
        let tx_manager = Arc::new(RwLock::new(TransactionManager::new()));

        let mut con = Connection::new(store, tx_manager);
        assert!(execute_command(&mut con, Command::Commit).is_err());

        execute_command(&mut con, Command::Begin(IsolationLevel::ReadUncommitted)).unwrap();

        assert!(execute_command(&mut con, Command::Abort).is_ok());
        assert!(con.cur_tx.is_none());

        let mut tx_m = con.get_tx_manager().write().unwrap();
        let txs = tx_m.get_transactions();
        assert!(txs.len() == 1);

        let my_tx = txs.get(&1).unwrap();
        assert!(*my_tx.get_state() == TransactionState::Aborted);
    }
}

#[cfg(test)]
mod isolation_tests {

    use super::*;

    #[test]
    fn test_read_uncommitted() {
        let store = Arc::new(RwLock::new(Storage::new()));
        let tx_manager = Arc::new(RwLock::new(TransactionManager::new()));

        let mut con = Connection::new(store.clone(), tx_manager.clone());
        let mut con2 = Connection::new(store, tx_manager.clone());

        execute_command(&mut con, Command::Begin(IsolationLevel::ReadUncommitted)).unwrap();
        execute_command(&mut con2, Command::Begin(IsolationLevel::ReadUncommitted)).unwrap();

        execute_command(&mut con, Command::Put("123".to_string(), "123".to_string())).unwrap();
        assert!(execute_command(&mut con2, Command::Get("123".to_string())).unwrap() == "123");
        assert!(tx_manager.clone().write().unwrap().get_transactions().len() == 2);
    }

    #[test]
    fn test_read_committed() {
        let store = Arc::new(RwLock::new(Storage::new()));
        let tx_manager = Arc::new(RwLock::new(TransactionManager::new()));

        let mut con = Connection::new(store.clone(), tx_manager.clone());
        let mut con2 = Connection::new(store.clone(), tx_manager.clone());

        execute_command(&mut con, Command::Begin(IsolationLevel::ReadCommitted)).unwrap();
        execute_command(&mut con2, Command::Begin(IsolationLevel::ReadCommitted)).unwrap();

        execute_command(&mut con, Command::Put("123".to_string(), "123".to_string())).unwrap();
        assert_eq!(
            execute_command(&mut con, Command::Get("123".to_string())).unwrap(),
            "123"
        );

        assert_eq!(
            execute_command(&mut con2, Command::Get("123".to_string()))
                .err()
                .unwrap(),
            CommandExecutionError::NoneVisible
        );

        execute_command(&mut con, Command::Commit).unwrap();
        assert_eq!(
            execute_command(&mut con2, Command::Get("123".to_string())).unwrap(),
            "123"
        );

        execute_command(&mut con, Command::Begin(IsolationLevel::ReadCommitted)).unwrap();

        execute_command(
            &mut con2,
            Command::Put("123".to_string(), "234".to_string()),
        )
        .unwrap();
        assert_eq!(
            execute_command(&mut con2, Command::Get("123".to_string())).unwrap(),
            "234"
        );

        assert_eq!(
            execute_command(&mut con, Command::Get("123".to_string())).unwrap(),
            "123"
        );

        execute_command(&mut con2, Command::Abort).unwrap();
        assert_eq!(
            execute_command(&mut con, Command::Get("123".to_string())).unwrap(),
            "123"
        );

        execute_command(&mut con2, Command::Begin(IsolationLevel::ReadCommitted)).unwrap();
        execute_command(&mut con2, Command::Delete("123".to_string())).unwrap();
        assert_eq!(
            execute_command(&mut con, Command::Get("123".to_string())).unwrap(),
            "123"
        );
        assert_eq!(
            execute_command(&mut con2, Command::Get("123".to_string()))
                .err()
                .unwrap(),
            CommandExecutionError::NoneVisible
        );

        execute_command(&mut con2, Command::Commit).unwrap();
        assert_eq!(
            execute_command(&mut con, Command::Get("123".to_string()))
                .err()
                .unwrap(),
            CommandExecutionError::NoneVisible
        );
    }

    #[test]
    fn test_repeatable_read() {
        let store = Arc::new(RwLock::new(Storage::new()));
        let tx_manager = Arc::new(RwLock::new(TransactionManager::new()));

        let mut con = Connection::new(store.clone(), tx_manager.clone());
        let mut con2 = Connection::new(store.clone(), tx_manager.clone());

        execute_command(&mut con, Command::Begin(IsolationLevel::RepeatableRead)).unwrap();
        execute_command(&mut con2, Command::Begin(IsolationLevel::RepeatableRead)).unwrap();

        execute_command(&mut con, Command::Put("123".to_string(), "123".to_string())).unwrap();
        assert_eq!(
            execute_command(&mut con, Command::Get("123".to_string())).unwrap(),
            "123"
        );
        assert_eq!(
            execute_command(&mut con2, Command::Get("123".to_string()))
                .err()
                .unwrap(),
            CommandExecutionError::NoneVisible
        );

        execute_command(&mut con, Command::Commit).unwrap();
        assert_eq!(
            execute_command(&mut con2, Command::Get("123".to_string()))
                .err()
                .unwrap(),
            CommandExecutionError::NoneVisible
        );

        let mut con3 = Connection::new(store.clone(), tx_manager.clone());
        execute_command(&mut con3, Command::Begin(IsolationLevel::RepeatableRead)).unwrap();
        assert_eq!(
            execute_command(&mut con3, Command::Get("123".to_string())).unwrap(),
            "123"
        );

        execute_command(
            &mut con3,
            Command::Put("123".to_string(), "234".to_string()),
        )
        .unwrap();
        assert_eq!(
            execute_command(&mut con3, Command::Get("123".to_string())).unwrap(),
            "234"
        );
        assert_eq!(
            execute_command(&mut con2, Command::Get("123".to_string()))
                .err()
                .unwrap(),
            CommandExecutionError::NoneVisible
        );

        let mut con4 = Connection::new(store.clone(), tx_manager.clone());
        execute_command(&mut con4, Command::Begin(IsolationLevel::RepeatableRead)).unwrap();
        assert_eq!(
            execute_command(&mut con4, Command::Get("123".to_string())).unwrap(),
            "123"
        );

        execute_command(&mut con4, Command::Delete("123".to_string())).unwrap();
        assert_eq!(
            execute_command(&mut con4, Command::Get("123".to_string()))
                .err()
                .unwrap(),
            CommandExecutionError::NoneVisible
        );
        assert_eq!(
            execute_command(&mut con2, Command::Get("123".to_string()))
                .err()
                .unwrap(),
            CommandExecutionError::NoneVisible
        );
        dbg!(store.read().unwrap().get_data());
        assert_eq!(
            execute_command(&mut con3, Command::Get("123".to_string())).unwrap(),
            "234"
        );
    }

    #[test]
    fn test_serializable() {
        let store = Arc::new(RwLock::new(Storage::new()));
        let tx_manager = Arc::new(RwLock::new(TransactionManager::new()));

        let mut con = Connection::new(store.clone(), tx_manager.clone());
        let mut con2 = Connection::new(store.clone(), tx_manager.clone());
        let mut con3 = Connection::new(store.clone(), tx_manager.clone());

        execute_command(&mut con, Command::Begin(IsolationLevel::Serializable)).unwrap();
        execute_command(&mut con2, Command::Begin(IsolationLevel::Serializable)).unwrap();
        execute_command(&mut con3, Command::Begin(IsolationLevel::Serializable)).unwrap();

        execute_command(&mut con, Command::Put("123".to_string(), "123".to_string())).unwrap();
        assert_eq!(
            execute_command(&mut con, Command::Get("123".to_string())).unwrap(),
            "123"
        );
        execute_command(&mut con, Command::Commit).unwrap();

        assert_eq!(
            execute_command(&mut con2, Command::Get("123".to_string()))
                .err()
                .unwrap(),
            CommandExecutionError::NoneVisible
        );
        assert_eq!(
            execute_command(&mut con2, Command::Commit).err().unwrap(),
            CommandExecutionError::SerializationError
        );

        execute_command(&mut con3, Command::Put("234".to_string(), "Pupupu".to_string())).unwrap();
        execute_command(&mut con3, Command::Commit).unwrap();
    }
}
