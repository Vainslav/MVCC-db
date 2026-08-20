use crate::storage::{RecordId, pages::PAGE_SIZE};

pub struct DataPageView<B> {
    buf: B,
}

impl<B: AsRef<[u8; PAGE_SIZE]>> DataPageView<B> {
    pub fn entry_count(&self) -> u16 {
        u16::from_le_bytes(self.buf.as_ref()[5..7].try_into().unwrap())
    }

    pub fn find(&self) -> Option<RecordId> {
        todo!()
    }
}

impl<B: AsMut<[u8; PAGE_SIZE]> + AsRef<[u8; PAGE_SIZE]>> DataPageView<B> {}

struct DataPageHeader {
    next_overflow_page: usize,
    entry_count: u16,
}

#[repr(C)]
struct DataPageEntry {
    hash: u64,
    key_len: u16,
    record_id: RecordId,
}
