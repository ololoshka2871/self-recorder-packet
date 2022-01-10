use core::{marker::PhantomData, mem::MaybeUninit};

use alloc::vec::Vec;
use heatshrink_rust::decoder::HeatshrinkDecoder;

use crate::DataPacketHeader;

pub struct DataBlockUnPacker<ID, TS> {
    data: Vec<u8>,
    id: PhantomData<ID>,
    ts: PhantomData<TS>,
}

impl<ID, TS> DataBlockUnPacker<ID, TS> {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            id: PhantomData,
            ts: PhantomData,
        }
    }

    pub fn header(&self) -> DataPacketHeader<ID, TS> {
        let mut res = unsafe { MaybeUninit::zeroed().assume_init() };

        unsafe {
            core::ptr::copy_nonoverlapping(
                self.data.as_ptr() as *const DataPacketHeader<ID, TS>,
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
                .skip(core::mem::size_of::<DataPacketHeader<ID, TS>>())
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
