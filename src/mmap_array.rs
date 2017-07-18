use memmap::{Mmap, Protection};
use errors::Error;
use std::path::Path;
use std::fs::OpenOptions;
use constants::*;
use enc;
use std::mem;

const HEADER_SIZE: u64 = MAGIC_KEY_SIZE + VERSION_SIZE;

#[derive(Debug)]
pub struct JumpTable {
    data: Mmap,
    version: u32,
    magic: u32,
    length: u64,
}

impl JumpTable {
    /// Calculate the file length required to save the list
    fn calculate_length(capacity : u64) -> u64 {
        HEADER_SIZE + capacity * mem::size_of::<u64>() as u64
    }

    fn create_mmap(path: &Path, capacity: u64) -> Result<Mmap, Error> {
        let data_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        let length = JumpTable::calculate_length(capacity);
        let meta_data = data_file.metadata()?;
        if length < meta_data.len() {
            panic!("Capacity is lower than the current file size which will result in file truncation.");
            //return Error();
        }

        // Set length of the file equal to the sum of all the header fields plus the number
        // of items required
        data_file.set_len(length)?;
        Ok(Mmap::open(&data_file, Protection::ReadWrite)?)
    }

    pub fn new(path: &Path, capacity: u64) -> Result<JumpTable, Error> {
        let data_mmap = JumpTable::create_mmap(path, capacity)?;
        let ptr = data_mmap.ptr();

        enc::encode(ptr, MAGIC_KEY);
        enc::encode_with_offset(ptr, MAGIC_KEY_SIZE, VERSION);

        Ok(JumpTable {
               data: data_mmap,
               version: VERSION,
               magic: MAGIC_KEY,
               length: capacity,
           })
    }

    pub fn expand() {}

    pub fn set(&self, index: u64, value: u64) {
        if self.length < index {
            enc::encode_with_offset(self.data.ptr(), 64 * index + HEADER_SIZE, value);
        } else {
            panic!("Index out of bound");
        }
    }

    pub fn get(&self, index: u64) -> u64 {
        if self.length < index {
            enc::from_ptr_with_offset::<u64>(self.data.ptr(), 64 * index + HEADER_SIZE)
        } else {
            panic!("Index out of bound");
        }
    }
}
