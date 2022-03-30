use core::iter::FromIterator;
use std::{fmt::Display, fs::File, io::Write, path::Path, time::Duration};

use alloc::vec::Vec;

use crate::{DataBlockUnPacker, DataPacketHeader};

#[derive(Clone, Copy, Default)]
pub struct Record {
    pub timesstamp: u64,
    pub freq: f32,
}

pub struct PageData {
    pub header: DataPacketHeader,
    pub consistant: bool,
    pub fp: Vec<Record>,
    pub ft: Vec<Record>,
}

pub struct PrettyDuration(pub Duration);

impl Display for PrettyDuration {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const SEC_PER_MINUTE: u64 = 60;
        const MIN_PER_HOUR: u64 = 60;
        const HOURS_PER_DAY: u64 = 24;

        let mut f = self.0;
        let days = self.0.as_secs() / (SEC_PER_MINUTE * MIN_PER_HOUR * HOURS_PER_DAY);
        if days > 0 {
            formatter.write_fmt(format_args!("{} d ", days))?;
        }
        f = f - Duration::from_secs(days * SEC_PER_MINUTE * MIN_PER_HOUR * HOURS_PER_DAY);

        let hours = f.as_secs() / (SEC_PER_MINUTE * MIN_PER_HOUR);
        formatter.write_fmt(format_args!("{:02}:", hours))?;
        f = f - Duration::from_secs(hours * SEC_PER_MINUTE * MIN_PER_HOUR);

        let minutes = f.as_secs() / SEC_PER_MINUTE;
        formatter.write_fmt(format_args!("{:02}:", minutes))?;
        f = f - Duration::from_secs(minutes * SEC_PER_MINUTE);

        let sec = f.as_secs();
        formatter.write_fmt(format_args!("{:02}.", sec))?;
        f = f - Duration::from_secs(sec);

        let ms = f.as_millis() as u16;
        formatter.write_fmt(format_args!("{:03}", ms))?;

        Ok(())
    }
}

impl PageData {
    pub fn save_as_csv<P: AsRef<Path>>(&self, file: P) -> std::io::Result<()> {
        let mut file = File::create(file)?;

        if self.header.is_initial() {
            file.write_fmt(format_args!(
                "Стартовая страница;{}\n",
                self.header.this_block_id
            ))?;
        } else {
            file.write_fmt(format_args!(
                "Страница {};предыдущая {}\n",
                self.header.this_block_id, self.header.prev_block_id
            ))?;
        }

        let start = Duration::from_millis(self.header.timestamp);
        file.write_fmt(format_args!(
            "Время начала страницы;{}\n",
            PrettyDuration(start)
        ))?;
        file.write_fmt(format_args!(
            "Базовый интервал;{};мс.\n",
            self.header.base_interval_ms
        ))?;
        file.write_fmt(format_args!(
            "Температура процессора;{};*С.;Заряд батареи;{};В\n",
            self.header.t_cpu, self.header.v_bat
        ))?;

        file.write("Время;Частота давления, Гц;Частота температуры, Гц\n".as_bytes())?;

        {
            let mut p_iter = self.fp.iter();
            let mut t_iter = self.ft.iter();
            let mut c_fp = if let Some(fp) = p_iter.next() {
                *fp
            } else {
                Record::default()
            };
            let mut c_ft = if let Some(ft) = t_iter.next() {
                *ft
            } else {
                Record::default()
            };

            file.write_fmt(format_args!(
                "{};{:.6};{:.6}\n",
                PrettyDuration(Duration::from_millis(self.header.timestamp)),
                c_fp.freq,
                c_ft.freq
            ))?;

            for i in 1.. {
                let timstamp = Duration::from_millis(
                    self.header.timestamp + (i * self.header.base_interval_ms) as u64,
                );
                let mut has_result = false;

                if i % self.header.interleave_ratio[0] == 0 {
                    if let Some(fp) = p_iter.next() {
                        c_fp = *fp;
                        has_result |= true;
                    } else {
                        break;
                    }
                }

                if i % self.header.interleave_ratio[1] == 0 {
                    if let Some(ft) = t_iter.next() {
                        c_ft = *ft;
                        has_result |= true;
                    } else {
                        break;
                    }
                }

                if has_result {
                    file.write_fmt(format_args!(
                        "{};{:.6};{:.6}\n",
                        PrettyDuration(timstamp),
                        c_fp.freq,
                        c_ft.freq
                    ))?
                }
            }
        }

        Ok(())
    }
}

pub fn calc_f(target: u32, result: u32, fref: f32) -> f32 {
    fref * target as f32 / result as f32
}

/// data - данные
/// page_size - размер страницы
/// fref - опорная частота из настроек
/// ignore_inconsistant - игнорировать ошибки и продлолжать
pub fn unpack_pages(data: &[u8], page_size: usize, fref_base: f32, ignore_inconsistant: bool) -> Vec<PageData> {
    let data = if data.len() % page_size == 0 {
        data
    } else {
        let full_pages = data.len() / page_size;
        println!(
            "Warning! allignment error: data size {} % page size {} != 0",
            data.len(),
            page_size
        );
        &data[..full_pages * page_size]
    };

    data.chunks(page_size)
        .map(|page| {
            let unpacker = DataBlockUnPacker::new(Vec::from_iter(page.iter().cloned()));
            let mut result = PageData {
                header: unpacker.hader(),
                consistant: unpacker.verify(),
                fp: Vec::new(),
                ft: Vec::new(),
            };

            let fref = if result.header.f_ref.is_normal() {
                result.header.f_ref
            } else {
                fref_base
            };

            if ignore_inconsistant || result.consistant  {
                // unpack data
                let mut data_iter = unpacker.unpack_as::<u32>().into_iter();

                let mut prev_p = 0u32;
                let mut prev_t = 0u32;
                for i in 0u32.. {
                    if result.header.interleave_ratio[0] == 0
                        || result.header.interleave_ratio[1] == 0
                    {
                        break;
                    }
                    if i % result.header.interleave_ratio[0] == 0 {
                        if let Some(v) = data_iter.next() {
                            let this_value = prev_p
                                .checked_add_signed(unsafe { core::mem::transmute(v) })
                                .unwrap_or_default();
                            prev_p = this_value;
                            result.fp.push(Record {
                                freq: calc_f(result.header.targets[0], this_value, fref),
                                timesstamp: result.header.timestamp
                                    + (i * result.header.base_interval_ms) as u64,
                            });
                        } else {
                            break;
                        }
                    }
                    if (result.header.interleave_ratio[1] != 0)
                        && (i % result.header.interleave_ratio[1] == 0)
                    {
                        if let Some(v) = data_iter.next() {
                            let this_value = prev_t
                                .checked_add_signed(unsafe { core::mem::transmute(v) })
                                .unwrap_or_default();
                            prev_t = this_value;
                            result.ft.push(Record {
                                freq: calc_f(result.header.targets[1], this_value, fref),
                                timesstamp: result.header.timestamp
                                    + (i * result.header.base_interval_ms) as u64,
                            });
                        } else {
                            break;
                        }
                    }
                }
            }

            result
        })
        .collect()
}
