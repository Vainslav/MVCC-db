use std::{
    io,
    ops::Deref,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use dashmap::{DashMap, Entry};

use crate::storage::{
    disk::DiskManager,
    pages::{Page, PageId, PageType},
};

pub struct ClockBufferPool {
    frames: Vec<Frame>,
    index: DashMap<PageId, usize>,
    clock_hand: AtomicUsize,
    disk_manager: Arc<DiskManager>,
}

impl ClockBufferPool {
    pub fn new(capacity: usize, disk_manager: Arc<DiskManager>) -> Self {
        Self {
            frames: (0..capacity).map(|_| Frame::empty()).collect(),
            index: DashMap::new(),
            clock_hand: AtomicUsize::new(0),
            disk_manager,
        }
    }

    pub fn fetch(&self, id: PageId, page_type: PageType) -> io::Result<PageHandle> {
        if let Some(idx) = self.index.get(&id) {
            let slot = self.frames[*idx].slot.read().unwrap();
            if let Some(page) = slot.as_ref() {
                if page.id == id {
                    page.pin_count.fetch_add(1, Ordering::AcqRel);
                    self.frames[*idx].ref_bit.store(true, Ordering::Release);
                    return Ok(PageHandle(page.clone()));
                }
            }
        }

        match self.index.entry(id) {
            Entry::Occupied(occ) => {
                let idx = *occ.get();
                let slot = self.frames[idx].slot.read().unwrap();
                let page = slot
                    .as_ref()
                    .filter(|p| p.id == id)
                    .expect("index says page loaded, but frame mismatch");
                page.pin_count.fetch_add(1, Ordering::AcqRel);
                self.frames[idx].ref_bit.store(true, Ordering::Release);
                Ok(PageHandle(page.clone()))
            }
            Entry::Vacant(vac) => {
                let page = self.disk_manager.read_page(&id, page_type)?;

                let victim_idx = self.find_victim_frame().expect("buffer pool exhausted");
                let mut slot = self.frames[victim_idx].slot.write().unwrap();
                if let Some(old) = slot.take() {
                    if old.dirty.load(Ordering::Acquire) {
                        self.disk_manager.write_page(&old)?;
                    }
                    self.index.remove(&old.id);
                }
                let arc = Arc::new(page);

                arc.pin_count.fetch_add(1, Ordering::AcqRel);
                *slot = Some(arc.clone());
                drop(slot);

                self.frames[victim_idx]
                    .ref_bit
                    .store(true, Ordering::Release);
                vac.insert(victim_idx);

                Ok(PageHandle(arc))
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = PageHandle> {
        self.frames.iter().filter_map(|f| {
            let slot = f.slot.read().unwrap();
            slot.as_ref().map(|p| {
                p.pin_count.fetch_add(1, Ordering::AcqRel);
                PageHandle(p.clone())
            })
        })
    }

    pub fn next_writable(&self, page_type: PageType) -> io::Result<PageHandle> {
        let id = self.disk_manager.next_writable(page_type)?;

        self.fetch(id, page_type)
    }

    fn find_victim_frame(&self) -> Option<usize> {
        let n = self.frames.len();
        for _ in 0..(2 * n) {
            let idx = self
                .clock_hand
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |h| Some((h + 1) % n))
                .unwrap();
            let slot = self.frames[idx].slot.read().unwrap();
            match slot.as_ref() {
                None => return Some(idx),
                Some(page) => {
                    if self.frames[idx].ref_bit.swap(false, Ordering::AcqRel) {
                        continue;
                    }
                    if page.pin_count.load(Ordering::Acquire) == 0 {
                        return Some(idx);
                    }
                }
            }
        }
        None
    }
}

struct Frame {
    slot: RwLock<Option<Arc<Page>>>,
    ref_bit: AtomicBool,
}

impl Frame {
    fn empty() -> Self {
        Self {
            slot: RwLock::new(None),
            ref_bit: AtomicBool::new(false),
        }
    }
}

pub struct PageHandle(Arc<Page>);

impl Deref for PageHandle {
    type Target = Page;
    fn deref(&self) -> &Page {
        &self.0
    }
}

impl Drop for PageHandle {
    fn drop(&mut self) {
        self.0.pin_count.fetch_sub(1, Ordering::AcqRel);
    }
}
