use std::collections::HashMap;

use crate::DbValue;

#[derive(Debug)]
pub struct StorageOld {
    pub data: HashMap<String, Vec<DbValue>>,
}

impl StorageOld {
    pub fn new() -> StorageOld {
        StorageOld {
            data: HashMap::new(),
        }
    }

    pub fn get_data(&self) -> &HashMap<String, Vec<DbValue>> {
        &self.data
    }
}
