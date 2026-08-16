use std::sync::{Arc, atomic::Ordering::Relaxed};

use crate::storage::pages::{Page, page_cache::PageCache};

pub struct BackgroundWriter<T>
where
    T: PageCache,
{
    page_cache: Arc<T>,
}

impl<T: PageCache> BackgroundWriter<T> {
    pub fn run(&self) {
        self.page_cache.iter().for_each(|page| {
            flush_page(&page);
        });
    }
}

fn flush_page(page: &Page) {
    let old = page.dirty.load(Relaxed);
    if old {
        let _lock = page.data.read().unwrap();
        if page
            .dirty
            .compare_exchange(old, false, Relaxed, Relaxed)
            .is_ok()
        {
            write_to_disk(page);
        }
    }
}

fn write_to_disk(page: &Page) {
    println!("Imaging this is writing to disk");
}
