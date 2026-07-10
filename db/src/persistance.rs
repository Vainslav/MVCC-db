use std::{sync::{Arc, atomic::Ordering::Relaxed}, thread, time::Duration};

use scheduled_executor::CoreExecutor;

use crate::storage::{PageCache, PageHandle};

pub struct BackgroundWriter<T>
where
    T: PageCache,
{
    page_cache: Arc<T>,
}

impl<T: PageCache> BackgroundWriter<T> {
    pub fn run(&self) {
        self.page_cache.iter().for_each(|page| {
            loop {
                let old = page.dirty.load(Relaxed);
                if old {
                    let _lock = page.data.read().unwrap();
                    if page.dirty.compare_exchange(old, false, Relaxed, Relaxed).is_ok(){
                        println!("Imaging this is writing to disk");
                        return;
                    }
                }
            }
        });
    }

    pub fn write_page(&self, id: usize) {
        let page = self.page_cache.get(&id).unwrap();
        loop {
            let old = page.dirty.load(Relaxed);
            if old {
                while page.pin_count.load(Relaxed) != 0 {} // todo, replace pin_ount and dirty with one Atomic
                if page.dirty.compare_exchange(old, false, Relaxed, Relaxed).is_ok(){
                    println!("Imaging this is writing to disk");
                    return;
                }
            }
        }
    }
}
