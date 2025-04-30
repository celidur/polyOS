use crate::kernel::KERNEL;
use fatfs::{IoBase, Read, Seek, SeekFrom, Write};

use super::block_dev::BlockDeviceError;

const BLOCK_SIZE: usize = 512;

#[derive(Debug)]
pub struct BufStream {
    id: usize,
    pos: usize,
}

impl BufStream {
    pub fn new(id: usize) -> Self {
        Self { id, pos: 0 }
    }
}

impl IoBase for BufStream {
    type Error = BlockDeviceError;
}

impl Read for BufStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, BlockDeviceError> {
        let mut total_read = 0;

        while total_read < buf.len() {
            let block_index = self.pos / BLOCK_SIZE;
            let offset_in_block = self.pos % BLOCK_SIZE;
            let to_read = core::cmp::min(BLOCK_SIZE - offset_in_block, buf.len() - total_read);

            let mut block_buf = [0u8; BLOCK_SIZE];

            match KERNEL.read_sectors(self.id, block_index as u64, 1, &mut block_buf) {
                Ok(_) => {
                    buf[total_read..total_read + to_read]
                        .copy_from_slice(&block_buf[offset_in_block..offset_in_block + to_read]);

                    self.pos += to_read;
                    total_read += to_read;
                }
                Err(_) => {
                    return Err(BlockDeviceError::IoError);
                }
            }
        }

        Ok(total_read)
    }
}

impl Write for BufStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize, BlockDeviceError> {
        let mut total_written = 0;

        while total_written < buf.len() {
            let block_index = self.pos / BLOCK_SIZE;
            let offset_in_block = self.pos % BLOCK_SIZE;
            let to_write = core::cmp::min(BLOCK_SIZE - offset_in_block, buf.len() - total_written);

            let mut block_buf = [0u8; BLOCK_SIZE];

            // Only read the block first if we're doing a partial write
            if offset_in_block != 0 || to_write < BLOCK_SIZE {
                match KERNEL.read_sectors(self.id, block_index as u64, 1, &mut block_buf) {
                    Ok(_) => {}
                    Err(_) => {
                        return Err(BlockDeviceError::IoError);
                    }
                }
            }

            // Copy data from input buffer into block buffer
            block_buf[offset_in_block..offset_in_block + to_write]
                .copy_from_slice(&buf[total_written..total_written + to_write]);

            match KERNEL.write_sectors(self.id, block_index as u64, 1, &block_buf) {
                Ok(_) => {
                    self.pos += to_write;
                    total_written += to_write;
                }
                Err(_) => {
                    return Err(BlockDeviceError::IoError);
                }
            }
        }

        Ok(total_written)
    }

    fn flush(&mut self) -> Result<(), BlockDeviceError> {
        KERNEL.sync();
        Ok(())
    }
}

impl Seek for BufStream {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, BlockDeviceError> {
        let new_pos = match pos {
            SeekFrom::Current(_) => panic!("SeekFrom::Current not supported"),
            SeekFrom::End(_) => panic!("SeekFrom::End not supported"),
            SeekFrom::Start(x) => x as usize,
        };

        self.pos = new_pos;
        Ok(self.pos as u64)
    }
}
