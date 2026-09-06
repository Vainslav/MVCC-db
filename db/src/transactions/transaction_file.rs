use std::{
    fs::{File, OpenOptions},
    io::{self, Seek, SeekFrom},
    ops::Range,
    sync::RwLock,
};

use crate::{
    TransactionState,
    write_read_impl::{read_exact_at_impl, write_at_impl},
};

struct TransactionFileHeader {
    next_transaction: u32,
}

const TRANSACTION_FILE_HEADER_SIZE: usize = std::mem::size_of::<u32>();

const NEXT_TRANSACTION_RANGE: Range<usize> = 0..4;

impl TransactionFileHeader {
    fn read_header_from_file(file: &File) -> io::Result<Self> {
        let mut buf = [0u8; TRANSACTION_FILE_HEADER_SIZE];
        read_exact_at_impl(file, &mut buf, 0)?;
        Ok(Self {
            next_transaction: u32::from_le_bytes(buf[NEXT_TRANSACTION_RANGE].try_into().unwrap()),
        })
    }

    fn new() -> Self {
        Self {
            next_transaction: 1,
        }
    }

    fn write_header_to_file(&self, file: &File) -> io::Result<()> {
        let mut buf = [0u8; TRANSACTION_FILE_HEADER_SIZE];
        buf[NEXT_TRANSACTION_RANGE].copy_from_slice(&self.next_transaction.to_le_bytes());
        write_at_impl(file, &buf, 0)
    }
}

pub struct TransactionFile {
    file: File,
    header: RwLock<TransactionFileHeader>,
}

impl TransactionFile {
    pub fn open(path: &str) -> io::Result<Self> {
        let (file, header) = match File::create_new(path) {
            Ok(f) => {
                let header = TransactionFileHeader::new();
                header.write_header_to_file(&f)?;
                (f, header)
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                let f = OpenOptions::new().read(true).write(true).open(path)?;
                let header = TransactionFileHeader::read_header_from_file(&f)?;
                (f, header)
            }
            Err(e) => return Err(e),
        };

        Ok(Self {
            file,
            header: header.into(),
        })
    }

    pub fn read_all_commited_transaction(&mut self) -> Vec<u32> {
        self.file
            .seek(SeekFrom::Start(TRANSACTION_FILE_HEADER_SIZE as u64 + 1))
            .expect("Not failing");
        (0..self.header.read().unwrap().next_transaction - 1)
            .map(|id| (id, self.read_transaction_state(id)))
            .filter(|(_, read_state_result)| {
                let state = read_state_result.as_ref().unwrap();
                *state == TransactionState::Committed
            })
            .map(|(id, _)| id)
            .collect()
    }

    pub fn read_transaction_state(&self, id: u32) -> io::Result<TransactionState> {
        let mut buf = [0u8; 1];
        read_exact_at_impl(&self.file, &mut buf, Self::transaction_offset(id))?;
        Ok(buf[0].into())
    }

    pub fn write_transaction_state(&self, id: u32, state: TransactionState) -> io::Result<()> {
        let buf = [state as u8];
        write_at_impl(&self.file, &buf, Self::transaction_offset(id))
    }

    fn transaction_offset(id: u32) -> u64 {
        TRANSACTION_FILE_HEADER_SIZE as u64 + id as u64
    }
}
