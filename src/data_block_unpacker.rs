use core::mem::MaybeUninit;

use alloc::vec::Vec;
use heatshrink_rust::decoder::HeatshrinkDecoder;

use crate::DataPacketHeader;

pub struct DataBlockUnPacker {
    data: Vec<u8>,
}

impl DataBlockUnPacker {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn header(&self) -> DataPacketHeader {
        let mut res = unsafe { MaybeUninit::zeroed().assume_init() };

        unsafe {
            core::ptr::copy_nonoverlapping(
                self.data.as_ptr() as *const DataPacketHeader,
                &mut res,
                1,
            );
        }

        res
    }

    pub fn unpack_data(&self) -> Vec<u8> {
        let decoder = HeatshrinkDecoder::source(
            self.data
                .iter()
                .skip(core::mem::size_of::<DataPacketHeader>())
                .cloned(),
        );

        decoder.collect()
    }

    pub fn unpack_as<T: Copy + Default>(&self) -> Vec<T> {
        let data = self.unpack_data();

        assert!(
            data.len() % core::mem::size_of::<T>() == 0,
            "Data allignment invalid"
        );

        data.chunks(core::mem::size_of::<T>())
            .map(|b| unsafe { *(b.as_ptr() as *const T) })
            .collect()
    }
}
