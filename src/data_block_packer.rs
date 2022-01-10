
use core::mem::swap;

use alloc::vec::Vec;

use heatshrink_rust::encoder_to_vec::HeatshrinkEncoderToVec;

use crate::DataPacketHeader;

pub struct DataBlockPacker<ID, TS> {
    pub header: DataPacketHeader<ID, TS>,
    encoder: Option<HeatshrinkEncoderToVec>,
    result: Option<Vec<u8>>,
}

#[derive(PartialEq, Debug)]
pub enum PushResult {
    Success,
    Full,
    Overflow,
    Finished,
}

impl<ID, TS> DataBlockPacker<ID, TS> {
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
            encoder: Some(HeatshrinkEncoderToVec::dest(
                Vec::with_capacity(size - core::mem::size_of::<DataPacketHeader<ID, TS>>()),
                core::mem::size_of::<DataPacketHeader<ID, TS>>(),
            )),
            result: None,
        }
    }

    fn get_encoder(&mut self) -> Option<HeatshrinkEncoderToVec> {
        let mut enc = None;
        swap(&mut self.encoder, &mut enc);
        enc
    }

    fn process_push_result(&mut self, res: heatshrink_rust::encoder_to_vec::Result) -> PushResult {
        match res {
            heatshrink_rust::encoder_to_vec::Result::Ok(enc) => {
                self.encoder = Some(enc);
                PushResult::Success
            }
            heatshrink_rust::encoder_to_vec::Result::Done(res) => {
                self.result = Some(res);
                PushResult::Full
            }
            heatshrink_rust::encoder_to_vec::Result::Overflow => PushResult::Overflow,
        }
    }

    /// push bytes to storage
    /// return true is success
    pub fn push_bytes(&mut self, data: &[u8]) -> PushResult {
        if let Some(enc) = self.get_encoder() {
            self.process_push_result(enc.push_bytes(data))
        } else {
            PushResult::Finished
        }
    }

    /// push byte to storage
    /// return true is success
    pub fn push_byte(&mut self, byte: u8) -> PushResult {
        if let Some(enc) = self.get_encoder() {
            self.process_push_result(
                enc.push_bytes(unsafe { core::slice::from_raw_parts(&byte, 1) }),
            )
        } else {
            PushResult::Finished
        }
    }

    /// push any value
    /// return true is success
    pub fn push_val<T: Copy>(&mut self, v: T) -> PushResult {
        if let Some(enc) = self.get_encoder() {
            self.process_push_result(enc.push(v))
        } else {
            PushResult::Finished
        }
    }

    pub fn to_result(self) -> Option<Vec<u8>> {
        if let Some(mut d) = self.result {
            unsafe {
                core::ptr::copy_nonoverlapping(
                    &self.header,
                    d.as_mut_ptr() as *mut DataPacketHeader<ID, TS>,
                    1,
                )
            };
            Some(d)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{DataBlockPacker, data_block_packer::PushResult};

    #[test]
    #[should_panic]
    fn create_too_small() {
        let _ = DataBlockPacker::new(0u32, 0, 0u64, 16);
    }

    #[test]
    fn crate_push() {
        const DATA_SIZE: usize = 4096;
        let mut block = DataBlockPacker::new(0u32, 0, 0u64, DATA_SIZE);

        for i in 0.. {
            match block.push_byte((i & 0xff) as u8) {
                PushResult::Success => {}
                PushResult::Full => break,
                _ => panic!(),
            }
        }

        assert_eq!(block.push_byte(0), PushResult::Finished);

        let res = block.to_result().unwrap();
        assert!(res.len() > DATA_SIZE / 2 && res.len() <= DATA_SIZE);
    }
}
