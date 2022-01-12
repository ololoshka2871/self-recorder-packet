#![feature(int_abs_diff)]
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
    fn interleave_generation() {
        const INTERLEAVE_RATIO: (u32, u32) = (3, 5);

        let mut fp = readfile("tests/test_data/FP1.txt").into_iter();
        let mut ft = readfile("tests/test_data/FT1.txt").into_iter();

        assert_eq!(fp.len(), ft.len());

        let mut count = (0u32, 0u32);

        for i in 0.. {
            if fp.next().is_none() {
                break;
            }
            if ft.next().is_none() {
                break;
            }
            if i % INTERLEAVE_RATIO.0 == 0 {
                count.0 += 1;
            }

            if i % INTERLEAVE_RATIO.1 == 0 {
                count.1 += 1;
            }
        }

        assert!(
            (count.0 * INTERLEAVE_RATIO.0).abs_diff(count.1 * INTERLEAVE_RATIO.1)
                <= u32::max(INTERLEAVE_RATIO.0, INTERLEAVE_RATIO.1)
        );
    }

    #[test]
    fn construct_interleave() {
        const INTERLEAVE_RATIO: (u32, u32) = (2, 3);

        let fp = readfile("tests/test_data/FP1.txt");
        let ft = readfile("tests/test_data/FT1.txt");

        let merged = fp
            .iter()
            .zip(ft.iter())
            .enumerate()
            .flat_map(|(i, (fp, ft))| {
                match (
                    i as u32 % INTERLEAVE_RATIO.0 == 0,
                    i as u32 % INTERLEAVE_RATIO.1 == 0,
                ) {
                    (false, false) => vec![],
                    (false, true) => vec![*ft],
                    (true, false) => vec![*fp],
                    (true, true) => vec![*fp, *ft],
                }
            })
            .collect::<Vec<_>>();

        let mut fp_up = vec![];
        let mut ft_up = vec![];
        let mut up_iter = merged.into_iter();
        for i in 0.. {
            if i as u32 % INTERLEAVE_RATIO.0 == 0 {
                if let Some(v) = up_iter.next() {
                    fp_up.push(v);
                } else {
                    break;
                }
            }
            if i as u32 % INTERLEAVE_RATIO.1 == 0 {
                if let Some(v) = up_iter.next() {
                    ft_up.push(v);
                } else {
                    break;
                }
            }
        }

        fp_up
            .iter()
            .enumerate()
            .for_each(|(i, v)| assert_eq!(*v, fp[i * INTERLEAVE_RATIO.0 as usize]));
        ft_up
            .iter()
            .enumerate()
            .for_each(|(i, v)| assert_eq!(*v, ft[i * INTERLEAVE_RATIO.1 as usize]));
    }

    #[test]
    fn code_decode_interleave() {
        const BLOCK_SIZE: usize = 4096;
        const INTERLEAVE_RATIO: (u32, u32) = (2, 3);
        const F_REF: u32 = 10_000_000;

        let fp = readfile("tests/test_data/FP1.txt");
        let ft = readfile("tests/test_data/FT1.txt");

        let p_target = fp[0].round() as u32;
        let t_target = ft[0].round() as u32;

        let fp = convert_to_results(fp, p_target, F_REF);
        let ft = convert_to_results(ft, t_target, F_REF);

        let merged = fp
            .iter()
            .zip(ft.iter())
            .enumerate()
            .flat_map(|(i, (fp, ft))| {
                match (
                    i as u32 % INTERLEAVE_RATIO.0 == 0,
                    i as u32 % INTERLEAVE_RATIO.1 == 0,
                ) {
                    (false, false) => vec![],
                    (false, true) => vec![*ft],
                    (true, false) => vec![*fp],
                    (true, true) => vec![*fp, *ft],
                }
            })
            .collect::<Vec<_>>();

        let compressed_chain = compress_diff(merged.iter(), BLOCK_SIZE);
        print_staticstics(&compressed_chain, BLOCK_SIZE);
        let unpacked_data: Vec<u32> = unpack_diff(compressed_chain);

        assert_eq!(&unpacked_data[..], &merged[..unpacked_data.len()]);

        let mut fp_up = vec![];
        let mut ft_up = vec![];
        let mut up_iter = unpacked_data.into_iter();
        for i in 0.. {
            if i as u32 % INTERLEAVE_RATIO.0 == 0 {
                if let Some(v) = up_iter.next() {
                    fp_up.push(v);
                } else {
                    break;
                }
            }
            if i as u32 % INTERLEAVE_RATIO.1 == 0 {
                if let Some(v) = up_iter.next() {
                    ft_up.push(v);
                } else {
                    break;
                }
            }
        }

        fp_up
            .iter()
            .enumerate()
            .for_each(|(i, v)| assert_eq!(*v, fp[i * INTERLEAVE_RATIO.0 as usize]));
        ft_up
            .iter()
            .enumerate()
            .for_each(|(i, v)| assert_eq!(*v, ft[i * INTERLEAVE_RATIO.1 as usize]));
    }

    fn convert_to_results(experimental_data: Vec<f32>, target: u32, f_ref: u32) -> Vec<u32> {
        experimental_data
            .iter()
            .map(|f| result(*f, target, f_ref))
            .collect::<Vec<_>>()
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
        let timstamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let packer = DataBlockPacker::new(
            id.checked_sub(1).unwrap_or_default(),
            *id,
            timstamp,
            block_size,
        );
        *id += 1;

        packer
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
