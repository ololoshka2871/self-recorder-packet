#![feature(mixed_integer_ops)]

mod test {
    use rand::{prelude::ThreadRng, Rng};
    use self_recorder_packet::DataBlockPacker;

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
        let mut block = DataBlockPacker::new(0, 0, 0x00000000, BLOCK_SIZE);

        let result = loop {
            match block.push_val(generator.next()) {
                self_recorder_packet::PushResult::Success => {
                    input_count += std::mem::size_of::<u32>();
                }
                self_recorder_packet::PushResult::Full => {
                    input_count += std::mem::size_of::<u32>();
                    break block.to_result().unwrap();
                }
                _ => panic!(),
            }
        };

        assert!(input_count > result.len());
        println!("{} compressed to {} bytes", input_count, result.len());
    }
}