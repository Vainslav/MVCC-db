use std::{sync::{Arc, atomic::{AtomicBool, Ordering::Relaxed}}, thread::{self, JoinHandle}, time::Duration};

use crate::{
    DiskManager,
    storage::pages::{Page, buffer_pool::ClockBufferPool},
};

pub struct BackgroundWriter {
    page_cache: Arc<ClockBufferPool>,
    disk_manager: Arc<DiskManager>,
    shutdown: AtomicBool,
}

impl BackgroundWriter {
    fn run(&self) {
        self.page_cache.iter().for_each(|page| {
            flush_page(&page, &self.disk_manager);
        });
    }

    pub fn start(self: Arc<Self>, interval: Duration) -> JoinHandle<()> {
        thread::spawn(move || {
            loop {
                if !self.shutdown.load(Relaxed) {
                    thread::sleep(interval);
                    self.run();
                }
            }
        })
    }

    pub fn new(page_cache: Arc<ClockBufferPool>, disk_manager: Arc<DiskManager>) -> Self {
        Self {
            page_cache,
            disk_manager,
            shutdown: false.into()
        }
    }

    pub fn stop(&self) {
        self.shutdown.store(true, Relaxed);
    }
}

fn flush_page(page: &Page, disk_manager: &DiskManager) {
    let _lock = page.data.read().unwrap();
    let old = page.dirty.load(Relaxed);
    if old {
        if page
            .dirty
            .compare_exchange(old, false, Relaxed, Relaxed)
            .is_ok()
        {
            disk_manager.write_page(page).expect("Background writer failed :(");
        }
    }
}
