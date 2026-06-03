use std::collections::HashMap;

use crate::dbcore::DbValue;

#[derive(Default)]
pub struct Storage {
    pub data: HashMap<String, Vec<DbValue>>,
}

impl Storage {
    pub fn new() -> Storage {
        Storage {
            data: HashMap::new(),
        }
    }

    pub fn get_data(&self) -> &HashMap<String, Vec<DbValue>> {
        &self.data
    }
}
