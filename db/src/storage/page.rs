use std::sync::{
    RwLock,
    atomic::{AtomicBool, AtomicUsize},
};

pub static PAGE_SIZE: usize = 16 * 1024;
pub static NUM_PAGES: usize = (100 * 1000 * 1024) / PAGE_SIZE;

pub struct Page {
    pub id: usize,
    pub pin_count: AtomicUsize, // todo: Replace pin_count and dirty with one atomic
    pub dirty: AtomicBool, // todo: Think about adding io_inprogress flag, if there is a posibility for multiple page writers
    pub data: RwLock<[u8; PAGE_SIZE]>,
}
