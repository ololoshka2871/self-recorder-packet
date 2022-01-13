#![feature(total_cmp)]
#![feature(mixed_integer_ops)]

mod test {
    use std::path::Path;

    use self_recorder_packet::{DataBlockPacker, DataBlockUnPacker};

    fn readfile<P: AsRef<Path>>(path: P) -> Vec<f32> {
        std::fs::read_to_string(path)
            .unwrap()
            .split("\n")
            .map(|s| {
                s.trim()
                    .parse::<f32>()
                    .map_err(|_| panic!("failed to parse \"{}\"", s))
                    .unwrap()
            })
            .collect()
    }

    #[test]
    fn convert_to_result_with_fixed_target() {
        const F_REF: u32 = 10_000_000;

        let data = readfile("tests/test_data/FP1.txt");
        let target = data[0].round() as u32;

        let result = result(data[0], target, F_REF);
        println!("result: {}", result);
    }

    #[test]
    fn process_data_set() {
        const BLOCK_SIZE: usize = 4096;
        const F_REF: u32 = 10_000_000;

        let experimental_data = readfile("tests/test_data/FP1.txt");

        let target = experimental_data[0].round() as u32;
        let experimental_data = experimental_data
            .iter()
            .map(|f| result(*f, target, F_REF))
            .collect::<Vec<_>>();

        let compressed_chain = compress(experimental_data.iter(), BLOCK_SIZE);
        print_staticstics(&compressed_chain, BLOCK_SIZE);

        let unpacked_data = unpack(compressed_chain);
        assert_eq!(
            &experimental_data[..unpacked_data.len()],
            &unpacked_data[..]
        );
    }

    #[test]
    fn process_data_set_diff() {
        const BLOCK_SIZE: usize = 4096;
        const F_REF: u32 = 10_000_000;

        let experimental_data = readfile("tests/test_data/FP1.txt");

        let target = experimental_data[0].round() as u32;
        let experimental_data = experimental_data
            .iter()
            .map(|f| result(*f, target, F_REF))
            .collect::<Vec<_>>();

        let compressed_chain = compress_diff(experimental_data.iter(), BLOCK_SIZE);
        print_staticstics(&compressed_chain, BLOCK_SIZE);

        let unpacked_data = unpack_diff(compressed_chain);
        assert_eq!(
            &experimental_data[..unpacked_data.len()],
            &unpacked_data[..]
        );
    }

    #[test]
    fn process_all_experimental_data() {
        const BLOCK_SIZE: usize = 4096;
        const F_REF: u32 = 10_000_000;

        std::fs::read_dir("tests/test_data")
            .unwrap()
            .for_each(|file| {
                if let Ok(f) = file {
                    println!("File: {:?}", f.path());
                    let experimental_data = readfile(f.path());
                    let target = experimental_data[0].round() as u32;
                    let experimental_data = experimental_data
                        .iter()
                        .map(|f| result(*f, target, F_REF))
                        .collect::<Vec<_>>();

                    let compressed_chain_plan = compress(experimental_data.iter(), BLOCK_SIZE);
                    let compressed_chain_diff = compress_diff(experimental_data.iter(), BLOCK_SIZE);
                    println!("== Plan compressing ==");
                    print_staticstics(&compressed_chain_plan, BLOCK_SIZE);
                    println!("== Diff compressing ==");
                    print_staticstics(&compressed_chain_diff, BLOCK_SIZE);

                    let unpacked_data_plan = unpack(compressed_chain_plan);
                    let unpacked_data_diff = unpack_diff(compressed_chain_diff);
                    assert_eq!(
                        &experimental_data[..unpacked_data_plan.len()],
                        &unpacked_data_plan[..]
                    );
                    assert_eq!(
                        &experimental_data[..unpacked_data_diff.len()],
                        &unpacked_data_diff[..]
                    );
                }
            });
    }

    fn result(f: f32, target: u32, fref: u32) -> u32 {
        // f = fref * target / result;
        // result = fref * target / f
        (fref as f32 * target as f32 / f).round() as u32
    }

    fn print_staticstics(compressed_chain: &Vec<(Vec<u8>, usize)>, block_size: usize) {
        #[derive(Default)]
        struct StaticticsItem {
            id: u32,
            src_size: usize,
            compressed_size: usize,
            usage_ratio: f32,
            compress_ratio: f32,
        }

        let mut staticstics = compressed_chain
            .iter()
            .enumerate()
            .map(|(i, item)| StaticticsItem {
                id: i as u32,
                src_size: item.1,
                compressed_size: block_size, //item.0.len(),
                compress_ratio: item.0.len() as f32 / item.1 as f32 * 100.0,
                usage_ratio: item.0.len() as f32 / block_size as f32 * 100.0,
            })
            .collect::<Vec<_>>();

        staticstics.sort_by(|x, y| x.compress_ratio.total_cmp(&y.compress_ratio));

        let worst_compres_ratio = staticstics.last().unwrap();
        let best_compres_ratio = staticstics.first().unwrap();

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
            r#"Totoal input: {} bytes -> {} bytes compressed ({} pages)
Avarage compressed ratio: {:.2} %
Avarage usage ratio: {:.2} % ({:.1} bytes)
Compression: Best {}: {:.2}%, Worst: {}: {:.2} %
    "#,
            avg.src_size,
            avg.compressed_size,
            staticstics.len(),
            avg.compress_ratio,
            avg.usage_ratio,
            avg.usage_ratio * block_size as f32,
            best_compres_ratio.id,
            best_compres_ratio.compress_ratio,
            worst_compres_ratio.id,
            worst_compres_ratio.compress_ratio,
        );
    }

    fn new_packer(id: &mut u32, block_size: usize) -> DataBlockPacker {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let packer = DataBlockPacker::builder()
            .set_ids(id.checked_sub(1).unwrap_or_default(), *id)
            .set_timestamp(timestamp)
            .set_size(block_size)
            .build();

        *id += 1;

        packer
    }

    fn compress<'a>(
        mut it: impl Iterator<Item = &'a u32>,
        block_size: usize,
    ) -> Vec<(Vec<u8>, usize)> {
        let mut current_block_id = 0u32;

        let mut compressed_chain = vec![];

        'compressor: loop {
            let mut packer = new_packer(&mut current_block_id, block_size);

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
                    // данные кончились, финализация не предусмотрена, просто выход
                    break 'compressor;
                }
            };
            compressed_chain.push((block, src_size));
        }

        compressed_chain
    }

    fn compress_diff<'a>(
        mut it: impl Iterator<Item = &'a u32>,
        block_size: usize,
    ) -> Vec<(Vec<u8>, usize)> {
        let mut current_block_id = 0u32;

        let mut compressed_chain = vec![];

        'compressor: loop {
            let mut packer = new_packer(&mut current_block_id, block_size);

            let mut prev = 0i32;
            let mut src_size = 0;
            let block = loop {
                if let Some(v) = it.next() {
                    let new_val = *v as i32;
                    let diff = new_val - prev;
                    prev = new_val;
                    match packer.push_val(diff) {
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
                    // данные кончились, финализация не предусмотрена, просто выход
                    break 'compressor;
                }
            };
            compressed_chain.push((block, src_size));
        }

        compressed_chain
    }

    fn unpack(compressed_chain: Vec<(Vec<u8>, usize)>) -> Vec<u32> {
        compressed_chain
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

                let mut data = unpacker.unpack_as::<u32>();
                acc.append(&mut data);
                acc
            })
    }

    fn unpack_diff(compressed_chain: Vec<(Vec<u8>, usize)>) -> Vec<u32> {
        compressed_chain
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

                let mut data = unpacker.unpack_as::<u32>();
                let mut prev = data[0];
                data[1..].iter_mut().for_each(|v| {
                    let this_value = prev
                        .checked_add_signed(unsafe { core::mem::transmute(*v) })
                        .unwrap();
                    prev = this_value;
                    *v = this_value;
                });

                acc.append(&mut data);
                acc
            })
    }
}
