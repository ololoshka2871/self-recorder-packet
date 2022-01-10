#![no_std]
#![feature(mixed_integer_ops)]

extern crate alloc;

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

mod data_block_packer;
pub use data_block_packer::{DataBlockPacker, PushResult};

mod data_block_unpacker;
pub use data_block_unpacker::*;
