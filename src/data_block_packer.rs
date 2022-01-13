use core::mem::swap;

use alloc::vec::Vec;

use heatshrink_rust::encoder_to_vec::HeatshrinkEncoderToVec;

use crate::DataPacketHeader;

pub struct DataBlockPacker {
    pub header: DataPacketHeader,
    encoder: Option<HeatshrinkEncoderToVec>,
    result: Option<Vec<u8>>,
}

pub struct DataBlockPackerBuilder {
    header: DataPacketHeader,
    size: usize,
}

#[derive(PartialEq, Debug)]
pub enum PushResult {
    Success,
    Full,
    Overflow,
    Finished,
}

impl DataBlockPackerBuilder {
    pub fn set_ids(mut self, prev_block_id: u32, this_block_id: u32) -> Self {
        self.header.prev_block_id = prev_block_id;
        self.header.this_block_id = this_block_id;
        self
    }

    pub fn set_targets(mut self, targets: (u32, u32)) -> Self {
        self.header.targets = targets;
        self
    }

    pub fn set_write_cfg(mut self, base_interval_ms: u32, interleave_ratio: (u32, u32)) -> Self {
        self.header.base_interval_ms = base_interval_ms;
        self.header.interleave_ratio = interleave_ratio;
        self
    }

    pub fn set_tcpu(mut self, t_cpu: f32) -> Self {
        self.header.t_cpu = t_cpu;
        self
    }

    pub fn set_vbat(mut self, v_bat: f32) -> Self {
        self.header.v_bat = v_bat;
        self
    }

    pub fn set_timestamp(mut self, timestamp: u64) -> Self {
        self.header.timestamp = timestamp;
        self
    }

    pub fn set_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    pub fn build(self) -> DataBlockPacker {
        assert!(self.size > core::mem::size_of::<DataPacketHeader>());
        DataBlockPacker {
            header: self.header,
            encoder: Some(HeatshrinkEncoderToVec::dest(
                Vec::with_capacity(self.size - core::mem::size_of::<DataPacketHeader>()),
                core::mem::size_of::<DataPacketHeader>(),
            )),
            result: None,
        }
    }
}

impl Default for DataBlockPackerBuilder {
    fn default() -> Self {
        Self {
            header: DataPacketHeader {
                prev_block_id: 0,
                this_block_id: 0,

                timestamp: 0,
                targets: (0, 0),

                base_interval_ms: 1000,
                interleave_ratio: (1, 1),

                t_cpu: 0.0,
                v_bat: 0.0,

                data_crc32: 0,
            },
            size: 4096,
        }
    }
}

impl DataBlockPacker {
    pub fn builder() -> DataBlockPackerBuilder {
        DataBlockPackerBuilder::default()
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
                    d.as_mut_ptr() as *mut DataPacketHeader,
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
    use crate::{data_block_packer::PushResult, DataBlockPacker};

    #[test]
    #[should_panic]
    fn create_too_small() {
        let _ = DataBlockPacker::builder().set_size(16).build();
    }

    #[test]
    fn crate_push() {
        const DATA_SIZE: usize = 4096;
        let mut packer = DataBlockPacker::builder().set_size(DATA_SIZE).build();

        for i in 0.. {
            match packer.push_byte((i & 0xff) as u8) {
                PushResult::Success => {}
                PushResult::Full => break,
                _ => panic!(),
            }
        }

        assert_eq!(packer.push_byte(0), PushResult::Finished);

        let res = packer.to_result().unwrap();
        assert!(res.len() > DATA_SIZE / 2 && res.len() <= DATA_SIZE);
    }
}
