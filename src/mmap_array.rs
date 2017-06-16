use memmap::{Mmap, Protection};
use errors::Error;
use std::path::Path;
use std::fs::{File, OpenOptions, metadata};
use constants::*;
use enc;
use std::mem;

const HEADER_SIZE: u64 = MAGIC_KEY_SIZE + VERSION_SIZE;

#[derive(Debug)]
pub struct MmapArray {
    data: Mmap,
    version: u32,
    magic: u32,
    length: u64,
}

impl MmapArray {
    pub fn new(path: &Path, capacity: u64) -> Result<MmapArray, Error> {
        let data_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        // Set length of the file equal to the sum of all the header fields plus the number
        // of items required
        data_file
            .set_len(HEADER_SIZE + capacity * mem::size_of::<u64>() as u64)?;
        let data_mmap = Mmap::open(&data_file, Protection::ReadWrite)?;
        let ptr = data_mmap.ptr();

        enc::encode(ptr, MAGIC_KEY);
        enc::encode_with_offset(ptr, MAGIC_KEY_SIZE, VERSION);

        Ok(MmapArray {
               data: data_mmap,
               version: VERSION,
               magic: MAGIC_KEY,
               length: capacity,
           })
    }
}
