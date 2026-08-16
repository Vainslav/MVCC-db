use std::ops::Range;

use crate::storage::{NewRecordId, pages::PAGE_SIZE};

const OVERFLOW_RANGE: Range<usize> = 1..5;
const ENTRIES_COUNT_RANGE: Range<usize> = 5..7;
const FREE_RANGE: Range<usize> = 7..9;
const ENTRIES_START: usize = 9;

pub struct BucketPageView<B> {
    buf: B,
}

#[repr(C)]
pub struct BucketPageHeader {
    next_overflow_page: u64,
    entry_count: u16,
    next_free_entry: u16,
}

pub const BUCKET_PAGE_HEADER_SIZE: usize = std::mem::size_of::<u64>() + // overflow
    std::mem::size_of::<u16>() + // entry_count
    std::mem::size_of::<u16>(); // next_free

#[repr(C)]
struct BucketEntryHeader {
    hash: u64,
    record_id: NewRecordId,
    key_len: u16,
}

const ENTRY_HEADER_SIZE: usize = std::mem::size_of::<u64>() + // hash
    std::mem::size_of::<u64>() + // record_id
    std::mem::size_of::<u16>(); // key_len

const ENTRY_PAGE_ID_OFFSET: usize = 8;
const ENTRY_SEGMENT_IF_OFFSET: usize = 12;
const ENTRY_PAGE_OFFSET_OFFSET: usize = 14;
const ENTRY_KEY_LEN_OFFSET: usize = 16;
const ENTRY_KEY_BYTES_OFFSET: usize = 18;

impl BucketEntryHeader {
    pub fn from_compacted_bytes(buf: &[u8], offset_in_buf: usize) -> BucketEntryHeader {
        let hash = u64::from_le_bytes(buf[offset_in_buf..offset_in_buf + ENTRY_PAGE_ID_OFFSET].try_into().unwrap());
        let page_id = u32::from_le_bytes(
            buf[offset_in_buf + ENTRY_PAGE_ID_OFFSET..offset_in_buf + ENTRY_SEGMENT_IF_OFFSET]
                .try_into()
                .unwrap(),
        );
        let segment_id = u16::from_le_bytes(
            buf[offset_in_buf + ENTRY_SEGMENT_IF_OFFSET..offset_in_buf + ENTRY_PAGE_OFFSET_OFFSET]
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
            segment_id,
            page_offset,
        };

        BucketEntryHeader {
            hash,
            record_id,
            key_len,
        }
    }
}

impl<B: AsRef<[u8; PAGE_SIZE]>> BucketPageView<B> {
    pub fn entry_count(&self) -> u16 {
        u16::from_le_bytes(
            self.buf.as_ref()[ENTRIES_COUNT_RANGE]
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
}

impl<B: AsMut<[u8; PAGE_SIZE]> + AsRef<[u8; PAGE_SIZE]>> BucketPageView<B> {
    pub fn write_entry(&mut self, key: &[u8], hash: u64, record_id: NewRecordId) -> Result<(), ()> {
        let entry_count = self.entry_count();
        let buf_mut = self.buf.as_mut();
        let needed = 20 + key.len();

        let next_free =
            u16::from_le_bytes(buf_mut[FREE_RANGE].try_into().expect("Should be 2 bytes")) as usize;

        if next_free + needed > PAGE_SIZE {
            return Err(());
        }

        buf_mut[next_free..next_free + ENTRY_PAGE_ID_OFFSET].copy_from_slice(&hash.to_le_bytes());
        buf_mut[next_free + ENTRY_PAGE_ID_OFFSET..next_free + ENTRY_SEGMENT_IF_OFFSET].copy_from_slice(&record_id.page_id.to_le_bytes());
        buf_mut[next_free + ENTRY_SEGMENT_IF_OFFSET..next_free + ENTRY_PAGE_OFFSET_OFFSET]
            .copy_from_slice(&record_id.segment_id.to_le_bytes());
        buf_mut[next_free + ENTRY_PAGE_OFFSET_OFFSET..next_free + ENTRY_KEY_LEN_OFFSET]
            .copy_from_slice(&record_id.page_offset.to_le_bytes());
        buf_mut[next_free + ENTRY_KEY_LEN_OFFSET..next_free + ENTRY_KEY_BYTES_OFFSET].copy_from_slice(&(key.len() as u16).to_le_bytes());
        buf_mut[next_free + ENTRY_KEY_BYTES_OFFSET..next_free + ENTRY_KEY_BYTES_OFFSET + key.len()].copy_from_slice(key);

        buf_mut[FREE_RANGE].copy_from_slice(&((next_free as usize + needed) as u16).to_le_bytes());
        buf_mut[ENTRIES_COUNT_RANGE].copy_from_slice(&(entry_count + 1).to_le_bytes());

        Ok(())
    }
}
