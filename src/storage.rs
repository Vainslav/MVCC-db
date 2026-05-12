use std::collections::HashMap;

use crate::dbcore::DbValue;

pub struct Storage {
    pub data: HashMap<String, Vec<DbValue>>,
}

impl Storage {
    pub fn new() -> Storage {
        Storage {
            data: HashMap::new(),
        }
    }
}
