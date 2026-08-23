use std::{io, ops::{Deref, DerefMut, Range}};

use crate::storage::{
    NewRecordId, pages::{PAGE_SIZE, Page, PageId, PageType, buffer_pool::{ClockBufferPool, PageHandle}},
};

const OVERFLOW_PAGE_ID_RANGE: Range<usize> = 1..3;
const OVERFLOW_FILE_ID_RANGE: Range<usize> = 3..5;
const ENTRIES_COUNT_RANGE: Range<usize> = 5..7;
const FREE_RANGE: Range<usize> = 7..9;
const ENTRIES_START: usize = 9;

pub struct BucketPageView<B> {
    buf: B,
}

#[repr(C)]
pub struct BucketPageHeader {
    next_overflow_page: PageId,
    entry_count: u16,
    next_free_entry: u16,
}

pub const BUCKET_PAGE_HEADER_SIZE: usize = std::mem::size_of::<u16>() + // next_overflow_page_id
    std::mem::size_of::<u16>() + // next_overflow_file_id
    std::mem::size_of::<u16>() + // entry_count
    std::mem::size_of::<u16>(); // next_free

#[repr(C)]
struct BucketEntryHeader {
    hash: u64,
    record_id: NewRecordId,
    key_len: u16,
}

const ENTRY_HEADER_SIZE: usize = std::mem::size_of::<u64>() + // hash
    std::mem::size_of::<u16>() +
    std::mem::size_of::<u16>() +
    std::mem::size_of::<u16>() + // record_id
    std::mem::size_of::<u16>(); // key_len

const ENTRY_PAGE_ID_OFFSET: usize = 8;
const ENTRY_SEGMENT_ID_OFFSET: usize = 10;
const ENTRY_PAGE_OFFSET_OFFSET: usize = 12;
const ENTRY_KEY_LEN_OFFSET: usize = 14;
const ENTRY_KEY_BYTES_OFFSET: usize = 16;

impl BucketEntryHeader {
    pub fn from_compacted_bytes(buf: &[u8], offset_in_buf: usize) -> BucketEntryHeader {
        let hash = u64::from_le_bytes(
            buf[offset_in_buf..offset_in_buf + ENTRY_PAGE_ID_OFFSET]
                .try_into()
                .unwrap(),
        );
        let page_id = u16::from_le_bytes(
            buf[offset_in_buf + ENTRY_PAGE_ID_OFFSET..offset_in_buf + ENTRY_SEGMENT_ID_OFFSET]
                .try_into()
                .unwrap(),
        );
        let segment_id = u16::from_le_bytes(
            buf[offset_in_buf + ENTRY_SEGMENT_ID_OFFSET..offset_in_buf + ENTRY_PAGE_OFFSET_OFFSET]
                .try_into()
                .unwrap(),
        );
        let page_offset = u16::from_le_bytes(
            buf[offset_in_buf + ENTRY_PAGE_OFFSET_OFFSET..offset_in_buf + ENTRY_KEY_LEN_OFFSET]
                .try_into()
                .unwrap(),
        );
        let key_len = u16::from_le_bytes(
            buf[offset_in_buf + ENTRY_KEY_LEN_OFFSET..offset_in_buf + ENTRY_KEY_BYTES_OFFSET]
                .try_into()
                .unwrap(),
        );

        let record_id = NewRecordId {
            page_id,
            file_id: segment_id,
            page_offset,
        };

        BucketEntryHeader {
            hash,
            record_id,
            key_len,
        }
    }
}

impl<B> BucketPageView<B> {
    
    pub fn new(buf: B) -> Self {
        BucketPageView { buf}
    }
}

impl<B: Deref<Target = [u8; PAGE_SIZE]>> BucketPageView<B> {
    pub fn entry_count(&self) -> u16 {
        u16::from_le_bytes(
            self.buf[ENTRIES_COUNT_RANGE]
                .try_into()
                .expect("Buf should be convertable to bytes"),
        )
    }

    pub fn find_entry(&self, key: &[u8], target_hash: u64) -> Option<NewRecordId> {
        let buf_ref = self.buf.as_ref();
        let entry_count = self.entry_count();

        let mut offset = ENTRIES_START;
        for _ in 0..entry_count {
            let header = BucketEntryHeader::from_compacted_bytes(buf_ref, offset);
            let key_bytes = &buf_ref
                [offset + ENTRY_HEADER_SIZE..offset + ENTRY_HEADER_SIZE + header.key_len as usize];

            offset += ENTRY_HEADER_SIZE + header.key_len as usize;
            if header.hash == target_hash && key_bytes == key {
                return Some(header.record_id);
            }
        }
        None
    }

    pub fn find_key(&self, key: &[u8], target_hash: u64) -> Option<usize> {
        let buf_ref = self.buf.as_ref();
        let entry_count = self.entry_count();

        let mut offset = ENTRIES_START;
        for _ in 0..entry_count {
            let header = BucketEntryHeader::from_compacted_bytes(buf_ref, offset);
            let key_bytes = &buf_ref
                [offset + ENTRY_HEADER_SIZE..offset + ENTRY_HEADER_SIZE + header.key_len as usize];
            
            if header.hash == target_hash && key_bytes == key {
                return Some(offset);
            }

            offset += ENTRY_HEADER_SIZE + header.key_len as usize;
        }
        None
    }

    pub fn next_overflow_page(&self) -> PageId {
        let next_page_id = u16::from_le_bytes(self.buf[OVERFLOW_PAGE_ID_RANGE].try_into().unwrap());
        let next_file_id = u16::from_le_bytes(self.buf[OVERFLOW_FILE_ID_RANGE].try_into().unwrap());

        PageId { page_num: next_page_id, file_id: next_file_id }
    }
}

impl<B: DerefMut<Target = [u8; PAGE_SIZE]> + Deref<Target = [u8; PAGE_SIZE]>> BucketPageView<B> {
    pub fn write_entry(&mut self, key: &[u8], hash: u64, record_id: NewRecordId) -> Result<(), ()> {
        let entry_count = self.entry_count();
        let buf_mut = self.buf.as_mut();
        let needed = ENTRY_HEADER_SIZE + key.len();

        let next_free =
            u16::from_le_bytes(buf_mut[FREE_RANGE].try_into().expect("Should be 2 bytes")) as usize;

        if next_free + needed > PAGE_SIZE {
            return Err(());
        }

        buf_mut[next_free..next_free + ENTRY_PAGE_ID_OFFSET].copy_from_slice(&hash.to_le_bytes());
        buf_mut[next_free + ENTRY_PAGE_ID_OFFSET..next_free + ENTRY_SEGMENT_ID_OFFSET]
            .copy_from_slice(&record_id.page_id.to_le_bytes());
        buf_mut[next_free + ENTRY_SEGMENT_ID_OFFSET..next_free + ENTRY_PAGE_OFFSET_OFFSET]
            .copy_from_slice(&record_id.file_id.to_le_bytes());
        buf_mut[next_free + ENTRY_PAGE_OFFSET_OFFSET..next_free + ENTRY_KEY_LEN_OFFSET]
            .copy_from_slice(&record_id.page_offset.to_le_bytes());
        buf_mut[next_free + ENTRY_KEY_LEN_OFFSET..next_free + ENTRY_KEY_BYTES_OFFSET]
            .copy_from_slice(&(key.len() as u16).to_le_bytes());
        buf_mut[next_free + ENTRY_KEY_BYTES_OFFSET..next_free + ENTRY_KEY_BYTES_OFFSET + key.len()]
            .copy_from_slice(key);

        buf_mut[FREE_RANGE].copy_from_slice(&((next_free as usize + needed) as u16).to_le_bytes());
        buf_mut[ENTRIES_COUNT_RANGE].copy_from_slice(&(entry_count + 1).to_le_bytes());

        Ok(())
    }
}

pub fn get_bucket_page_init_bytes() -> [u8; PAGE_SIZE] {
    let mut buf = [0; PAGE_SIZE]; // could be changed to uninit in the future, but will require some extra work

    // type
    buf[0] = PageType::Bucket as u8;

    // page header
    buf[FREE_RANGE].copy_from_slice(&(ENTRIES_START as u16).to_le_bytes());

    buf
}

pub struct BucketChainIter<'a> {
    pool: &'a ClockBufferPool,
    next_page: Option<PageId>
}

impl<'a> BucketChainIter<'a> {
    pub fn new(pool: &'a ClockBufferPool, page_id: PageId) -> Self {
        Self { pool, next_page: Some(page_id) }
    }
}

impl<'a> Iterator for BucketChainIter<'a> {
    type Item = io::Result<PageHandle>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.next_page.clone()?;

        let handle = match self.pool.fetch(next, PageType::Bucket) {
            Ok(h) => h,
            Err(e) => {
                self.next_page = None;
                return Some(Err(e));
            }
        };

        let overflow = {
            let buf = handle.data.read().unwrap();
            BucketPageView::new(&*buf).next_overflow_page()
        };

        self.next_page = if overflow.file_id == 0 && overflow.page_num == 0 { None } else { Some(overflow) };

        Some(Ok(handle))
    }
}
