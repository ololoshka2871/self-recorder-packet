#![feature(total_cmp)]

mod test {
    use std::path::Path;

    use self_recorder_packet::{DataBlockPacker, DataBlockUnPacker};

    fn readfile<P: AsRef<Path>>(path: P) -> Vec<f32> {
        std::fs::read_to_string(path)
            .unwrap()
            .split("\n")
            .map(|s| s.trim().parse::<f32>().unwrap())
            .collect()
    }

    #[test]
    fn compress_1_block_real_data() {
        const BLOCK_SIZE: usize = 4096;

        let experimental_data = readfile("tests/test_data/P1.txt");

        let mut it = experimental_data.iter();
        let mut input_count = 0;

        let mut block = DataBlockPacker::new(0, 0, 0x00000000, BLOCK_SIZE);

        let result = loop {
            match block.push_val(it.next().unwrap()) {
                self_recorder_packet::PushResult::Success => {
                    input_count += 1;
                }
                self_recorder_packet::PushResult::Full => {
                    input_count += 1;
                    break block.to_result().unwrap();
                }
                _ => panic!(),
            }
        };

        println!(
            "{} floats compressed to {} bytes",
            input_count,
            result.len()
        );
    }

    #[test]
    fn compress_decompress_1_block_real_data() {
        const BLOCK_SIZE: usize = 4096;

        let experimental_data = readfile("tests/test_data/T1.txt");
        let mut it = experimental_data.iter();
        let mut block = DataBlockPacker::new(56, 57, 0x000100080, BLOCK_SIZE);

        let result = loop {
            match block.push_val(*it.next().unwrap()) {
                self_recorder_packet::PushResult::Success => {}
                self_recorder_packet::PushResult::Full => {
                    break block.to_result().unwrap();
                }
                _ => panic!(),
            }
        };

        let unpacker = DataBlockUnPacker::new(result);
        let unpacked = unpacker.unpack_as();
        let exp_fragment = experimental_data
            .iter()
            .cloned()
            .take(unpacked.len())
            .collect::<Vec<_>>();
        assert_eq!(exp_fragment, unpacked);
    }

    #[test]
    fn process_data_set() {
        #[derive(Default)]
        struct StaticticsItem {
            id: u32,
            src_size: usize,
            compressed_size: usize,
            usage_ratio: f32,
            compress_ratio: f32,
        }

        const BLOCK_SIZE: usize = 4096;

        let experimental_data = readfile("tests/test_data/P1.txt");
        let mut it = experimental_data.iter();
        let mut current_block_id = 0u32;

        let mut compressed_chain = vec![];

        'compressor: loop {
            let timstamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let mut packer = DataBlockPacker::new(
                current_block_id.checked_sub(1).unwrap_or_default(),
                current_block_id,
                timstamp,
                BLOCK_SIZE,
            );
            current_block_id += 1;

            let mut src_size = 0;
            let block = loop {
                if let Some(v) = it.next() {
                    match packer.push_val(*v) {
                        self_recorder_packet::PushResult::Success => {
                            src_size += std::mem::size_of::<f32>();
                        }
                        self_recorder_packet::PushResult::Full => {
                            src_size += std::mem::size_of::<f32>();
                            break packer.to_result().unwrap();
                        }
                        _ => panic!(),
                    }
                } else {
                    // данные кончились, финализации нет, просто выход
                    break 'compressor;
                }
            };
            compressed_chain.push((block, src_size));
        }

        let mut staticstics = compressed_chain
            .iter()
            .enumerate()
            .map(|(i, item)| StaticticsItem {
                id: i as u32,
                src_size: item.1,
                compressed_size: BLOCK_SIZE, //item.0.len(),
                compress_ratio: item.0.len() as f32 / item.1 as f32 * 100.0,
                usage_ratio: item.0.len() as f32 / BLOCK_SIZE as f32,
            })
            .collect::<Vec<_>>();

        staticstics.sort_by(|x, y| x.compress_ratio.total_cmp(&y.compress_ratio));

        let worst_compres_ratio = staticstics.first().unwrap();
        let best_compres_ratio = staticstics.last().unwrap();

        let mut avg = staticstics
            .iter()
            .fold(StaticticsItem::default(), |mut acc, item| {
                acc.src_size += item.src_size;
                acc.compressed_size += item.compressed_size;
                acc.usage_ratio += item.usage_ratio;
                acc.compress_ratio += item.compress_ratio;

                acc
            });
        avg.usage_ratio /= staticstics.len() as f32;
        avg.compress_ratio /= staticstics.len() as f32;

        println!(
            r#"Totoal input: {} bytes -> {} bytes compressed
Avarage compressed ratio: {:.2} %
Avarage usage ratio: {:.2} % ({:.1} bytes)
Compression: Best {}: {:.2}%, Worst: {}: {:.2} %
        "#,
            avg.src_size,
            avg.compressed_size,
            avg.compress_ratio,
            avg.usage_ratio,
            avg.usage_ratio * BLOCK_SIZE as f32,
            best_compres_ratio.id,
            best_compres_ratio.compress_ratio,
            worst_compres_ratio.id,
            worst_compres_ratio.compress_ratio,
        );

        // unpack back
        let unpacked_data = compressed_chain
            .iter()
            .cloned()
            .enumerate()
            .fold(vec![], |mut acc, (pocket_id, block)| {
                let unpacker = DataBlockUnPacker::new(block.0);
                let h = unpacker.hader();
                assert_eq!(pocket_id as u32, h.this_block_id);
                assert_eq!(
                    (pocket_id as u32).checked_sub(1).unwrap_or_default(),
                    h.prev_block_id
                );

                let mut data = unpacker.unpack_as::<f32>();
                acc.append(&mut data);
                acc
            });
        assert_eq!(&experimental_data[..unpacked_data.len()], &unpacked_data[..]);
    }
}
