#![cfg_attr(not(feature = "unpacker"), no_std)]
#![feature(mixed_integer_ops)]

extern crate alloc;

#[derive(PartialEq, Debug, Clone)]
pub struct DataPacketHeader {
    pub prev_block_id: u32,
    pub this_block_id: u32,

    pub timestamp: u64,

    pub targets: [u32; 2],

    pub base_interval_ms: u32,
    pub interleave_ratio: [u32; 2],

    pub t_cpu: f32,
    pub v_bat: f32,

    pub data_len: u32,
    pub data_crc32: u32,
}

impl DataPacketHeader {
    pub fn is_initial(&self) -> bool {
        self.prev_block_id == 0 && self.this_block_id == 0
    }
}

mod data_block_packer;
pub use data_block_packer::{DataBlockPacker, PushResult};

mod data_block_unpacker;
pub use data_block_unpacker::DataBlockUnPacker;

#[cfg(feature = "unpacker")]
mod data_unpacker;
#[cfg(feature = "unpacker")]
pub use data_unpacker::*;