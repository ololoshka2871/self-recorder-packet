#![cfg_attr(not(feature = "unpacker"), no_std)]

extern crate alloc;

#[derive(PartialEq, Debug, Clone)]
pub struct DataPacketHeader {
    /// номер этого блока
    pub prev_block_id: u32,
    /// номер предыдущего блока в цепочке
    pub this_block_id: u32,

    /// таймштамп, время от старта записи
    pub timestamp: u64,
    /// опорная частота, она могла меняться между цепочками
    pub f_ref: f32,

    /// таргеты
    pub targets: [u32; 2],

    /// базовый интервал записи, мс
    pub base_interval_ms: u32,
    /// делители базового интервала
    pub interleave_ratio: [u32; 2],

    /// температура процессора
    pub t_cpu: f32,
    /// заряд батареи
    pub v_bat: f32,

    /// Фактическое количество значащих байт в блоке, не считая еиспользованные с конц байты
    pub data_len: u32,
    /// CRC32 (zlib)
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

// https://github.com/sdleffler/empty-box-rs
mod empty_box;
pub use empty_box::EmptyBox;

pub(crate) mod add_signed;