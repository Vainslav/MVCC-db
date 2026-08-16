use std::{fs::{File, OpenOptions}, sync::LazyLock};

#[repr(C)]
struct IndexFileHeader {
    bucket_count: u32
}

const FILE_PATH: &str = "db.idx";

static file: LazyLock<File> = LazyLock::new(init_index_file);

fn init_index_file() -> File {
    let mut new_file = File::create_new(FILE_PATH);

    if new_file.is_err() {
        new_file = OpenOptions::new().read(true).write(true).open(FILE_PATH);
    }

    new_file.expect("Index file could not be created")
}

fn alloc_bucket_page() -> usize {
    todo!()
}

fn write_page() {

}