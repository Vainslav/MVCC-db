mod background;
mod dashmap_cache;
mod disk;
mod page;
mod page_cache;

#[derive(Debug)]
pub struct RecordId(pub u64);

#[derive(Debug)]
pub struct DbValue {
    pub tx_start: usize,
    pub tx_end: usize,
    pub prev: RecordId,
    pub value: String,
}

impl DbValue {
    pub fn new(tx_start: usize, tx_end: usize, value: String) -> DbValue {
        DbValue {
            tx_start,
            tx_end,
            prev: RecordId(0),
            value,
        }
    }
}
