#![no_std]
#![feature(mixed_integer_ops)]

extern crate alloc;

#[derive(PartialEq, Debug, Clone)]
pub struct DataPacketHeader {
    pub prev_block_id: u32,
    pub this_block_id: u32,

    pub timestamp: u64,

    pub p_target: u32,
    pub t_target: u32,

    pub t_cpu: f32,
    pub v_bat: f32,

    pub data_crc32: u32,
}

mod data_block_packer;
pub use data_block_packer::{DataBlockPacker, PushResult};

mod data_block_unpacker;
pub use data_block_unpacker::DataBlockUnPacker;
