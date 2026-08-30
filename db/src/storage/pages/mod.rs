pub mod bucket_page;
pub mod buffer_pool;
pub mod data_page;

use std::sync::{
    RwLock,
    atomic::{
        AtomicBool, AtomicUsize,
        Ordering::{Relaxed, SeqCst},
    },
};

use crate::storage::pages::{
    bucket_page::get_bucket_page_init_bytes, data_page::get_data_page_init_bytes,
};

pub const PAGE_SIZE: usize = 16 * 1024;
pub const _NUM_PAGES: usize = (100 * 1000 * 1024) / PAGE_SIZE;

const _PAGE_TYPE_OFFSET: usize = 0;

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum PageType {
    Bucket = 1,
    Data = 2,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct PageId {
    pub page_num: u16,
    pub file_id: u16,
}

pub struct Page {
    pub id: PageId,
    // pin_count_and_dirty: AtomicUsize,
    pub pin_count: AtomicUsize, // todo: Replace pin_count and dirty with one atomic
    pub dirty: AtomicBool, // todo: Think about adding io_inprogress flag, if there is a posibility for multiple page writers
    pub data: RwLock<[u8; PAGE_SIZE]>,
}

impl Page {
    pub fn get_page_type(&self) -> PageType {
        match self.data.read().expect("Not poisoned")[0] {
            1 => PageType::Bucket,
            2 => PageType::Data,
            _ => panic!("Invalid page type"),
        }
    }

    // pub fn get_dirty(&self) -> bool {
    //     (self.pin_count_and_dirty.load(Relaxed) >> 63) != 0
    // }

    // pub fn get_pin_count(&self) -> usize {
    //     (self.pin_count_and_dirty.load(Relaxed) << 1) >> 1
    // }

    // pub fn update_state(&self, new_dirty: bool, new_pin_count: usize, old_dirty: bool, old_pin_count: usize) -> Result<usize, usize> {
    //     if old_pin_count >> 63 != 0 || new_pin_count >> 63 != 0 {
    //         panic!("Only support 63 bit pin_count")
    //     }

    //     let old_value = ((old_dirty as usize) << 63) + old_pin_count;
    //     let new_value = ((new_dirty as usize) << 63) + new_pin_count;

    //     self.pin_count_and_dirty.compare_exchange(old_value, new_value, SeqCst, SeqCst)
    // }
}

pub fn get_init_page_bytes(page_type: PageType) -> [u8; PAGE_SIZE] {
    match page_type {
        PageType::Bucket => get_bucket_page_init_bytes(),
        PageType::Data => get_data_page_init_bytes(),
    }
}
