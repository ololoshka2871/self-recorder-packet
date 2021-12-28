#![no_std]

extern crate alloc;

use alloc::vec::Vec;


pub struct DataPacketHeader<ID, TS> {
    pub prev_block_id: ID,
    pub this_block_id: ID,

    pub timestamp: TS,

    pub p_target: u32,
    pub t_target: u32,
    pub p_initial_result: u32,
    pub t_initial_result: u32,

    pub t_cpu: f32,
    pub v_bat: f32,

    pub crc32: u32,
}

pub struct DataBlock<ID, TS> {
    pub header: DataPacketHeader<ID, TS>,
    data: Vec<u8>,
}

impl<ID, TS> DataBlock<ID, TS> {
    pub fn new(prev_block_id: ID, this_block_id: ID, timestamp: TS, size: usize) -> Self {
        assert!(size > core::mem::size_of::<DataPacketHeader<ID, TS>>());
        Self {
            header: DataPacketHeader {
                prev_block_id,
                this_block_id,

                timestamp,
                p_target: 0,
                t_target: 0,
                p_initial_result: 0,
                t_initial_result: 0,

                t_cpu: 0.0,
                v_bat: 0.0,

                crc32: 0,
            },
            data: Vec::with_capacity(size - core::mem::size_of::<DataPacketHeader<ID, TS>>()),
        }
    }

    /// push bytes to storage
    /// return true is success
    pub fn push_bytes(&mut self, data: &[u8]) -> bool {
        let have_space = self.avalable() >= data.len();
        if have_space {
            self.data.extend_from_slice(data);
        }
        have_space
    }

    /// push byte to storage
    /// return true is success
    pub fn push_byte(&mut self, byte: u8) -> bool {
        let avalable = self.avalable() > 0;
        if avalable {
            self.data.push(byte);
        }
        avalable
    }

    /// is data space avalable?
    pub fn avalable(&self) -> usize {
        self.data.capacity() - self.data.len()
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use crate::DataBlock;

    #[test]
    #[should_panic]
    fn create_too_small() {
        let _ = DataBlock::new(0u32, 0, 0u64, 16);
    }

    #[test]
    fn crate_push() {
        const DATA_SIZE: usize = 64;
        let mut block = DataBlock::new(0u32, 0, 0u64, DATA_SIZE);

        for n in 0..block.avalable() as u8 {
            assert!(block.push_byte(n))
        }
        
        assert_eq!(block.avalable(), 0);
        assert_eq!(block.push_byte(0), false)
    }

    #[test]
    fn crate_push_slice() {
        const DATA_SIZE: usize = 64;
        let mut block = DataBlock::new(0u32, 0, 0u64, DATA_SIZE);

        let d = vec![0u8; block.avalable()];
        assert!(block.push_bytes(d.as_slice()));

        assert_eq!(block.avalable(), 0);
        assert_eq!(block.push_byte(0), false)
    }
}
