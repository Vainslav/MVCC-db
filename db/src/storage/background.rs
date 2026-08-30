use std::sync::{Arc, atomic::Ordering::Relaxed};

use crate::{
    DiskManager,
    storage::pages::{Page, buffer_pool::ClockBufferPool},
};

pub struct BackgroundWriter {
    page_cache: Arc<ClockBufferPool>,
    disk_manager: Arc<DiskManager>,
}

impl BackgroundWriter {
    pub fn run(&self) {
        self.page_cache.iter().for_each(|page| {
            flush_page(&page, &self.disk_manager);
        });
    }

    pub fn new(page_cache: Arc<ClockBufferPool>, disk_manager: Arc<DiskManager>) -> Self {
        Self {
            page_cache,
            disk_manager,
        }
    }
}

fn flush_page(page: &Page, disk_manager: &DiskManager) {
    let old = page.dirty.load(Relaxed);
    if old {
        let _lock = page.data.read().unwrap();
        if page
            .dirty
            .compare_exchange(old, false, Relaxed, Relaxed)
            .is_ok()
        {
            disk_manager.write_page(page).unwrap();
        }
    }
}
