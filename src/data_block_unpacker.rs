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

    pub fn hader(&self) -> DataPacketHeader {
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

    #[cfg(feature = "unpacker")]
    pub fn verify(&self) -> bool {
        use crc32fast::Hasher;

        let header = self.hader();

        let mut hasher = Hasher::new();
        hasher.update(
            &self.data[core::mem::size_of::<DataPacketHeader>()
                ..(core::mem::size_of::<DataPacketHeader>() + header.data_len as usize)],
        );
        let checksum = hasher.finalize();

        header.data_crc32 == checksum
    }

    pub fn unpack_data(&self) -> Vec<u8> {
        let header = self.hader();
        let decoder = HeatshrinkDecoder::source(
            self.data
                .iter()
                .skip(core::mem::size_of::<DataPacketHeader>())
                .take(header.data_len as usize)
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
