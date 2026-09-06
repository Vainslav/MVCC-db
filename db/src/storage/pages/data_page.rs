use std::ops::{Deref, DerefMut, Range};

use crate::storage::{
    NewRecordId, RECORD_ID_SIZE, Record,
    pages::{PAGE_SIZE, PageType},
};

pub struct DataPageView<B> {
    buf: B,
}

struct _DataPageHeader {
    entry_count: u16,
    next_free: u16,
}

const ENTRY_COUNT_RANGE: Range<usize> = 1..3;
const NEXT_FREE_RANGE: Range<usize> = 3..5;

const RECORDS_START: usize = 5;

impl<B> DataPageView<B> {
    pub fn new(buf: B) -> Self {
        DataPageView { buf }
    }
}

impl<B: Deref<Target = [u8; PAGE_SIZE]>> DataPageView<B> {
    pub fn entry_count(&self) -> u16 {
        u16::from_le_bytes(self.buf.as_ref()[ENTRY_COUNT_RANGE].try_into().unwrap())
    }

    pub fn get_record(&self, page_offset: usize) -> Record {
        let disk_record = DataPageRecord::from_compacted_bytes(
            self.buf[page_offset..page_offset + RECORD_SIZE]
                .try_into()
                .unwrap(),
        );

        let string_bytes = self.buf
            [page_offset + RECORD_SIZE..page_offset + RECORD_SIZE + disk_record.data_len as usize]
            .to_vec();

        let str = String::from_utf8(string_bytes).unwrap();

        Record::from_data_page_record_and_string(disk_record, str)
    }

    fn next_free(&self) -> usize {
        u16::from_le_bytes(
            self.buf[NEXT_FREE_RANGE]
                .try_into()
                .expect("Should be 2 bytes"),
        ) as usize
    }
}

impl<B: DerefMut<Target = [u8; PAGE_SIZE]>> DataPageView<B> {
    pub fn append_record(&mut self, record: Record) -> Result<u16, ()> {
        let entry_count = self.entry_count();
        let next_free = self.next_free();

        let needed = record_len(&record);

        if next_free + needed > PAGE_SIZE {
            return Err(());
        }

        write_record_to_buf(&mut self.buf, next_free, record);

        self.write_entry_count(entry_count + 1);
        self.write_next_free((next_free + needed) as u16);

        Ok(next_free as u16)
    }

    pub fn change_xmax(&mut self, offset: usize, new_xmax: u32) {
        let xmax_range_updated: Range<usize> =
            XMAX_RANGE.min().unwrap() + offset..XMAX_RANGE.last().unwrap() + offset + 1;

        self.buf[xmax_range_updated].copy_from_slice(&new_xmax.to_le_bytes());
    }

    fn write_entry_count(&mut self, new_entry_count: u16) {
        self.buf[ENTRY_COUNT_RANGE].copy_from_slice(&new_entry_count.to_le_bytes());
    }

    fn write_next_free(&mut self, new_next_free: u16) {
        self.buf[NEXT_FREE_RANGE].copy_from_slice(&new_next_free.to_le_bytes());
    }
}

impl Record {
    fn from_data_page_record_and_string(disk_record: DataPageRecord, str: String) -> Self {
        Record {
            xmin: disk_record.xmin,
            xmax: disk_record.xmax,
            prev: disk_record.prev,
            value: str,
        }
    }
}

#[derive(Debug)]
struct DataPageRecord {
    pub xmin: u32,
    pub xmax: u32,
    pub prev: Option<NewRecordId>,
    pub data_len: u16,
}

const RECORD_SIZE: usize = std::mem::size_of::<u32>()
    + std::mem::size_of::<u32>()
    + RECORD_ID_SIZE
    + std::mem::size_of::<u16>();

const XMIN_RANGE: Range<usize> = 0..4;
const XMAX_RANGE: Range<usize> = 4..8;
const PREV_RECORD_RANGE: Range<usize> = 8..8 + RECORD_ID_SIZE;
const DATA_LEN_RANGE: Range<usize> = 8 + RECORD_ID_SIZE..8 + RECORD_ID_SIZE + 2;

impl DataPageRecord {
    fn from_compacted_bytes(buf: &[u8; RECORD_SIZE]) -> Self {
        let xmin = u32::from_le_bytes(buf[XMIN_RANGE].try_into().unwrap());
        let xmax = u32::from_le_bytes(buf[XMAX_RANGE].try_into().unwrap());
        let prev = NewRecordId::from_compacted_bytes(buf[PREV_RECORD_RANGE].try_into().unwrap());
        let data_len = u16::from_le_bytes(buf[DATA_LEN_RANGE].try_into().unwrap());

        DataPageRecord {
            xmin,
            xmax,
            prev,
            data_len,
        }
    }
}

fn write_record_to_buf(buf: &mut [u8; PAGE_SIZE], offset: usize, record: Record) {
    let xmin_range_updated: Range<usize> =
        XMIN_RANGE.min().unwrap() + offset..XMIN_RANGE.last().unwrap() + offset + 1;
    let xmax_range_updated: Range<usize> =
        XMAX_RANGE.min().unwrap() + offset..XMAX_RANGE.last().unwrap() + offset + 1;
    let prev_range_updated: Range<usize> =
        PREV_RECORD_RANGE.min().unwrap() + offset..PREV_RECORD_RANGE.last().unwrap() + offset + 1;
    let data_len_range_updated: Range<usize> =
        DATA_LEN_RANGE.min().unwrap() + offset..DATA_LEN_RANGE.last().unwrap() + offset + 1;

    buf[xmin_range_updated].copy_from_slice(&record.xmin.to_le_bytes());
    buf[xmax_range_updated].copy_from_slice(&record.xmax.to_le_bytes());
    buf[prev_range_updated].copy_from_slice(
        &record
            .prev
            .map_or_else(|| [0; RECORD_ID_SIZE], |r| r.to_le_bytes()),
    );
    buf[data_len_range_updated].copy_from_slice(&(record.value.len() as u16).to_le_bytes());

    buf[DATA_LEN_RANGE.last().unwrap() + 1 + offset
        ..DATA_LEN_RANGE.last().unwrap() + offset + record.value.len() + 1]
        .copy_from_slice(record.value.as_bytes());
}

fn record_len(record: &Record) -> usize {
    RECORD_SIZE + record.value.len()
}

pub fn get_data_page_init_bytes() -> [u8; PAGE_SIZE] {
    let mut buf = [0; PAGE_SIZE]; // could be changed to uninit in the future, but will require some extra work

    // type
    buf[0] = PageType::Data as u8;

    // page header
    buf[NEXT_FREE_RANGE].copy_from_slice(&(RECORDS_START as u16).to_le_bytes());

    buf
}
