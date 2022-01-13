#[cfg(feature = "unpacker")]
#[cfg(unix)]
mod test {
    use std::{convert::TryInto, path::Path};

    use self_recorder_packet::{unpack_pages, DataBlockPacker};

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

    fn result(f: f32, target: u32, fref: u32) -> u32 {
        // f = fref * target / result;
        // result = fref * target / f
        (fref as f32 * target as f32 / f).round() as u32
    }

    fn push_value(
        packer: &mut DataBlockPacker,
        counter: u32,
        i: usize,
        freqs: &[f32; 2],
        prevs: &mut [i32; 2],
        fref: u32,
    ) -> bool {
        if counter % packer.header.interleave_ratio[i] == 0 {
            let result = result(freqs[i], packer.header.targets[i], fref);
            let diff = result as i32 - prevs[i];
            prevs[i] = result as i32;
            match packer.push_val(diff) {
                self_recorder_packet::PushResult::Success => false,
                self_recorder_packet::PushResult::Full => true,
                _ => panic!(),
            }
        } else {
            false
        }
    }

    #[test]
    fn compress_simulation() {
        const BASE_INTERVAL_MS: u32 = 1000;
        const F_REF: u32 = 10_000_000;
        const BLOCK_SIZE: usize = 4096;
        const INTERLEAVE_RATIO: [u32; 2] = [1, 1];

        let fp = readfile("tests/test_data/FP1.txt");
        let ft = readfile("tests/test_data/FT1.txt");

        // симуляция измеренной частоты, при каждом обращении будем читать обновленное значение
        let mut src = fp.iter().cloned().zip(ft.iter().cloned());

        // Это типо наша флешка
        let mut storage: Vec<[u8; BLOCK_SIZE]> = Vec::new();

        let mut id = 0u32;
        'compressor: loop {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let targets = if let Some((fp, ft)) = src.next() {
                [fp.round() as u32, ft.round() as u32]
            } else {
                break;
            };

            let mut packer = DataBlockPacker::builder()
                .set_ids(id.checked_sub(1).unwrap_or_default(), id)
                .set_timestamp(timestamp)
                .set_write_cfg(BASE_INTERVAL_MS, INTERLEAVE_RATIO)
                .set_targets(targets)
                .set_size(BLOCK_SIZE)
                .build();
            id += 1;

            let mut prevs = [0i32, 0];
            'page: for counter in 1u32.. {
                if let Some((fp, ft)) = src.next() {
                    if push_value(&mut packer, counter, 0, &[fp, ft], &mut prevs, F_REF)
                        || push_value(&mut packer, counter, 1, &[fp, ft], &mut prevs, F_REF)
                    {
                        // место закончилось
                        storage.push(
                            packer
                                .to_result_full(|data| {
                                    let mut hasher = crc32fast::Hasher::new();
                                    hasher.update(data);
                                    hasher.finalize()
                                })
                                .unwrap()
                                .try_into()
                                .unwrap(),
                        );
                        break 'page;
                    }
                } else {
                    break 'compressor;
                }
            }
        }

        println!("Compressed {} pages", storage.len());

        // Типо прочитано из файла
        let storage = storage.iter().flatten().cloned().collect::<Vec<_>>();

        let unpacked_pages = unpack_pages(storage.as_slice(), BLOCK_SIZE, F_REF as f32);
        let dir =
            tempdir::TempDir::new("compress_simulation").expect("Failed to create result dir");

        unpacked_pages.into_iter().for_each(|page| {
            assert!(page.consistant);

            page.save_as_csv(dir.path().join(format!(
                "{}-0x{:08X}.csv",
                page.header.this_block_id, page.header.data_crc32,
            )))
            .expect("Faild to save page");
        });

        // don't delete tempdir
        core::mem::forget(dir);
    }
}
