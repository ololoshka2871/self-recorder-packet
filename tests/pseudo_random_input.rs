#![feature(mixed_integer_ops)]

mod test {
    use rand::{prelude::ThreadRng, Rng};
    use self_recorder_packet::{DataBlockPacker, DataBlockUnPacker};

    const INITIAL_RESULT: u32 = 12_000_000;
    const OFFSET_MAX: i32 = 50;
    const MAX_RESULT: u32 = (INITIAL_RESULT as f32 * 1.2) as u32;
    const MIN_RESULT: u32 = (INITIAL_RESULT as f32 * 0.8) as u32;

    struct ResultGenerator {
        step: f32,
        prev_val: u32,
        rng: ThreadRng,
    }

    impl ResultGenerator {
        fn new() -> Self {
            Self {
                step: 0.0,
                prev_val: INITIAL_RESULT,
                rng: rand::thread_rng(),
            }
        }
    }

    impl Iterator for ResultGenerator {
        type Item = u32;

        fn next(&mut self) -> Option<Self::Item> {
            let new_offset = (self.step.sin() * OFFSET_MAX as f32
                + self.rng.gen_range(-OFFSET_MAX..OFFSET_MAX) as f32 / 5.0)
                as i32;
            self.step += 0.001;
            if let Some(v) = self.prev_val.checked_add_signed(new_offset) {
                if v > MIN_RESULT && v < MAX_RESULT {
                    return Some(v);
                }
            }

            self.next()
        }
    }

    #[test]
    fn generate_result_sequence() {
        let generator = ResultGenerator::new();

        let res = generator.take(10000).fold(String::new(), |mut acc, v| {
            acc.push_str(format!("{}\n", v).as_str());
            acc
        });

        std::fs::write("/tmp/vals.txt", res).unwrap();
    }

    #[test]
    fn compress_pseudo_random() {
        const BLOCK_SIZE: usize = 4096;

        let mut generator = ResultGenerator::new();
        let mut input_count = 0;
        let mut block = DataBlockPacker::builder().set_size(BLOCK_SIZE).build();

        let result = loop {
            match block.push_val(generator.next().unwrap()) {
                self_recorder_packet::PushResult::Success => {
                    input_count += std::mem::size_of::<u32>();
                }
                self_recorder_packet::PushResult::Full => {
                    input_count += std::mem::size_of::<u32>();
                    break block.to_result_trimmed(|_| 0).unwrap();
                }
                _ => panic!(),
            }
        };

        assert!(input_count > result.len());
        println!("{} compressed to {} bytes", input_count, result.len());
    }

    #[test]
    fn compress_decompress_pseudo_random() {
        const BLOCK_SIZE: usize = 4096;

        let mut generator = ResultGenerator::new();
        let mut input_data = Vec::new();
        let mut block = DataBlockPacker::builder()
            .set_ids(35, 36)
            .set_timestamp(0x01300aafa0170)
            .set_size(BLOCK_SIZE)
            .build();

        let result = loop {
            let v = generator.next().unwrap();
            match block.push_val(v) {
                self_recorder_packet::PushResult::Success => {
                    input_data.push(v);
                }
                self_recorder_packet::PushResult::Full => {
                    input_data.push(v);
                    break block.to_result_trimmed(|_| 0).unwrap();
                }
                _ => panic!(),
            }
        };

        let res_len = result.len();
        let unpacker = DataBlockUnPacker::new(result);

        assert_eq!(input_data, unpacker.unpack_as());
        println!(
            "Packed {} values to block ({} bytes)",
            input_data.len(),
            res_len
        );
    }

    #[test]
    fn compress_decompress_floats() {
        const BLOCK_SIZE: usize = 4096;

        let mut generator = ResultGenerator::new();
        let mut input_data = Vec::new();
        let mut block = DataBlockPacker::builder()
            .set_ids(45, 46)
            .set_timestamp(0x71389aaf60180)
            .set_size(BLOCK_SIZE)
            .build();

        let result = loop {
            let v = generator.next().unwrap() as f32 / INITIAL_RESULT as f32;
            match block.push_val(v) {
                self_recorder_packet::PushResult::Success => {
                    input_data.push(v);
                }
                self_recorder_packet::PushResult::Full => {
                    input_data.push(v);
                    break block
                        .to_result_trimmed(|_| 0)
                        .unwrap();
                }
                _ => panic!(),
            }
        };

        let res_len = result.len();
        let unpacker = DataBlockUnPacker::new(result);

        let unpacked = unpacker.unpack_as();
        assert_eq!(input_data, unpacked);
        println!(
            "Packed {} values to block ({} bytes)",
            input_data.len(),
            res_len
        );
    }
}
