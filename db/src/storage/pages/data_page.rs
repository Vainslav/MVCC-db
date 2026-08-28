use std::{
    io,
    ops::{Deref, DerefMut, Range},
};

use crate::storage::{NewRecordId, RECORD_ID_SIZE, RecordId, pages::PAGE_SIZE};

pub struct DataPageView<B> {
    buf: B,
}

impl<B: Deref<Target = [u8; PAGE_SIZE]>> DataPageView<B> {
    pub fn entry_count(&self) -> u16 {
        u16::from_le_bytes(self.buf.as_ref()[5..7].try_into().unwrap())
    }

    pub fn get_record(&self, page_offset: usize) -> Record {
        let disk_record = DiskRecord::from_compacted_bytes(
            self.buf[page_offset..page_offset + RECORD_SIZE]
                .try_into()
                .unwrap(),
        );

        let string_bytes = self.buf
            [page_offset + RECORD_SIZE..page_offset + RECORD_SIZE + disk_record.data_len as usize]
            .to_vec();

        let str = String::from_utf8(string_bytes).unwrap();

        Record::from_disk_record_and_string(disk_record, str)
    }
}

impl<B: DerefMut<Target = [u8; PAGE_SIZE]> + Deref<Target = [u8; PAGE_SIZE]>> DataPageView<B> {
    pub fn write_record(&mut self, offset: usize, record: Record) {
        write_record_to_buf(&mut self.buf, offset, record);
    }
}

struct DataPageHeader {
    entry_count: u16,
}

pub struct Record {
    pub xmin: u32,
    pub xmax: u32,
    pub prev: NewRecordId,
    pub data: String,
}

impl Record {
    fn from_disk_record_and_string(disk_record: DiskRecord, str: String) -> Self {
        Record {
            xmin: disk_record.xmin,
            xmax: disk_record.xmax,
            prev: disk_record.prev,
            data: str,
        }
    }
}

struct DiskRecord {
    pub xmin: u32,
    pub xmax: u32,
    pub prev: NewRecordId,
    pub data_len: u16,
}

const RECORD_SIZE: usize = std::mem::size_of::<u32>()
    + std::mem::size_of::<u32>()
    + RECORD_ID_SIZE
    + std::mem::size_of::<u16>();

const XMIN_RANGE: Range<usize> = 0..4;
const XMAX_RANGE: Range<usize> = 4..8;
const PREV_RECORD_RANGE: Range<usize> = 8..8 + RECORD_ID_SIZE;
const DATA_LEN_RANGE: Range<usize> = 8 + RECORD_ID_SIZE..10 + RECORD_ID_SIZE;

impl DiskRecord {
    fn from_compacted_bytes(buf: &[u8; RECORD_SIZE]) -> Self {
        let xmin = u32::from_le_bytes(buf[XMIN_RANGE].try_into().unwrap());
        let xmax = u32::from_le_bytes(buf[XMAX_RANGE].try_into().unwrap());
        let prev = NewRecordId::from_compacted_bytes(buf[PREV_RECORD_RANGE].try_into().unwrap());
        let data_len = u16::from_le_bytes(buf[XMAX_RANGE].try_into().unwrap());

        DiskRecord {
            xmin,
            xmax,
            prev,
            data_len,
        }
    }
}

fn write_record_to_buf(buf: &mut [u8; PAGE_SIZE], offset: usize, record: Record) {
    let xmin_range_updated: Range<usize> =
        XMIN_RANGE.min().unwrap() + offset..XMIN_RANGE.last().unwrap() + offset;
    let xmax_range_updated: Range<usize> =
        XMAX_RANGE.min().unwrap() + offset..XMAX_RANGE.last().unwrap() + offset;
    let prev_range_updated: Range<usize> =
        PREV_RECORD_RANGE.min().unwrap() + offset..PREV_RECORD_RANGE.last().unwrap() + offset;
    let data_len_range_updated: Range<usize> =
        DATA_LEN_RANGE.min().unwrap() + offset..DATA_LEN_RANGE.last().unwrap() + offset;

    buf[xmin_range_updated].copy_from_slice(&record.xmin.to_le_bytes());
    buf[xmax_range_updated].copy_from_slice(&record.xmax.to_le_bytes());
    buf[prev_range_updated].copy_from_slice(&record.prev.to_le_bytes());
    buf[data_len_range_updated].copy_from_slice(&(record.data.len() as u16).to_le_bytes());

    buf[DATA_LEN_RANGE.last().unwrap()..DATA_LEN_RANGE.last().unwrap() + record.data.len()]
        .copy_from_slice(record.data.as_bytes());
}
