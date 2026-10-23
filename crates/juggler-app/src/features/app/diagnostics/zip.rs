//! A small zip writer for diagnostics: deflated text files, UTF-8 names,
//! no zip64 (a diagnostics zip is a few hundred KB). The `zip` crate
//! stays out of the tree (its advisories; see Cargo.toml), and flate2,
//! already here, gives the deflate and the CRC-32.

use std::io::Write as _;

/// One file in the archive.
pub struct Entry<'a> {
    /// Its path inside the archive (`logs/supervision.jsonl`).
    pub name: &'a str,
    /// Its bytes.
    pub data: &'a [u8],
}

const LOCAL: u32 = 0x0403_4b50;
const CENTRAL: u32 = 0x0201_4b50;
const END: u32 = 0x0605_4b50;
/// Version 2.0: deflate.
const VERSION: u16 = 20;
/// Made on Unix (3), so the mode below is read as one.
const MADE_BY: u16 = (3 << 8) | VERSION;
/// Bit 11: the names are UTF-8.
const UTF8: u16 = 1 << 11;
const DEFLATE: u16 = 8;
/// A regular file, rw-r--r--.
const MODE: u32 = 0o100_644 << 16;

/// The archive's bytes: `entries` deflated, stamped with `at_ms` (UTC).
///
/// # Errors
///
/// When an entry or the archive is too big for a plain zip (4 GB, 65 535
/// entries), or deflating fails.
pub fn write(entries: &[Entry<'_>], at_ms: i64) -> std::io::Result<Vec<u8>> {
    let too_big = || std::io::Error::other("too big for a diagnostics zip");
    let (time, date) = dos_time(at_ms);
    let mut out = Vec::new();
    let mut central = Vec::new();
    for entry in entries {
        let mut crc = flate2::Crc::new();
        crc.update(entry.data);
        let mut deflate =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        deflate.write_all(entry.data)?;
        let packed = deflate.finish()?;
        let name = entry.name.as_bytes();
        let sizes = [
            u32::try_from(packed.len()).map_err(|_| too_big())?,
            u32::try_from(entry.data.len()).map_err(|_| too_big())?,
        ];
        let name_len = u16::try_from(name.len()).map_err(|_| too_big())?;
        let offset = u32::try_from(out.len()).map_err(|_| too_big())?;
        // The local header, then the data.
        put32(&mut out, LOCAL);
        for field in [VERSION, UTF8, DEFLATE, time, date] {
            put16(&mut out, field);
        }
        for field in [crc.sum(), sizes[0], sizes[1]] {
            put32(&mut out, field);
        }
        put16(&mut out, name_len);
        put16(&mut out, 0);
        out.extend_from_slice(name);
        out.extend_from_slice(&packed);
        // Its central directory record.
        put32(&mut central, CENTRAL);
        for field in [MADE_BY, VERSION, UTF8, DEFLATE, time, date] {
            put16(&mut central, field);
        }
        for field in [crc.sum(), sizes[0], sizes[1]] {
            put32(&mut central, field);
        }
        // Name length, then no extra field, comment, disk or internal attributes.
        for field in [name_len, 0, 0, 0, 0] {
            put16(&mut central, field);
        }
        put32(&mut central, MODE);
        put32(&mut central, offset);
        central.extend_from_slice(name);
    }
    let count = u16::try_from(entries.len()).map_err(|_| too_big())?;
    let central_at = u32::try_from(out.len()).map_err(|_| too_big())?;
    let central_len = u32::try_from(central.len()).map_err(|_| too_big())?;
    out.extend_from_slice(&central);
    put32(&mut out, END);
    for field in [0, 0, count, count] {
        put16(&mut out, field);
    }
    put32(&mut out, central_len);
    put32(&mut out, central_at);
    put16(&mut out, 0);
    Ok(out)
}

fn put16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// MS-DOS time and date (two-second steps, from 1980) for `at_ms` UTC.
fn dos_time(at_ms: i64) -> (u16, u16) {
    let secs = at_ms.div_euclid(1000);
    let (days, of_day) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let (year, month, day) = civil_day(days);
    if year < 1980 {
        return (0, (1 << 5) | 1);
    }
    let time = ((of_day / 3600) << 11) | ((of_day % 3600 / 60) << 5) | ((of_day % 60) / 2);
    let date = ((year - 1980).min(127) << 9) | (month << 5) | day;
    (
        u16::try_from(time).unwrap_or(0),
        u16::try_from(date).unwrap_or(0),
    )
}

/// The calendar day of `days` since 1970-01-01 (Howard Hinnant's
/// `civil_from_days`).
pub(crate) const fn civil_day(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

/// Reads the archive back by its central directory, as an unzip tool
/// does: every name, inflated, with its CRC checked.
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::cast_possible_truncation)]
pub(crate) fn read(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    use std::io::Read as _;
    let u16_at = |at: usize| usize::from(u16::from_le_bytes([bytes[at], bytes[at + 1]]));
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let end = bytes.len() - 22;
    assert_eq!(u32_at(end), END);
    let count = u16_at(end + 10);
    let mut at = u32_at(end + 16) as usize;
    let mut files = Vec::new();
    for _ in 0..count {
        assert_eq!(u32_at(at), CENTRAL);
        let crc = u32_at(at + 16);
        let packed = u32_at(at + 20) as usize;
        let name_len = u16_at(at + 28);
        let local = u32_at(at + 42) as usize;
        let name = String::from_utf8(bytes[at + 46..at + 46 + name_len].to_vec()).unwrap();
        assert_eq!(u32_at(local), LOCAL);
        let data_at = local + 30 + u16_at(local + 26) + u16_at(local + 28);
        let mut data = Vec::new();
        flate2::read::DeflateDecoder::new(&bytes[data_at..data_at + packed])
            .read_to_end(&mut data)
            .unwrap();
        let mut sum = flate2::Crc::new();
        sum.update(&data);
        assert_eq!(sum.sum(), crc, "{name}");
        files.push((name, data));
        at += 46 + name_len;
    }
    files
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn what_goes_in_comes_back_out() {
        let big = "a line of a log\n".repeat(10_000);
        let bytes = write(
            &[
                Entry {
                    name: "about.json",
                    data: b"{\"version\":\"0.1.0\"}",
                },
                Entry {
                    name: "logs/supervision.jsonl",
                    data: big.as_bytes(),
                },
                Entry {
                    name: "empty",
                    data: b"",
                },
            ],
            1_792_713_600_000,
        )
        .unwrap();
        assert!(bytes.len() < big.len() / 10, "deflated: {}", bytes.len());
        let files = read(&bytes);
        assert_eq!(
            files[0],
            ("about.json".into(), b"{\"version\":\"0.1.0\"}".to_vec())
        );
        assert_eq!(files[1].1, big.as_bytes());
        assert_eq!(files[2], ("empty".into(), Vec::new()));
    }

    #[test]
    fn the_system_unzip_reads_it_where_there_is_one() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("d.zip");
        std::fs::write(
            &path,
            write(
                &[Entry {
                    name: "a/b.txt",
                    data: b"hello",
                }],
                0,
            )
            .unwrap(),
        )
        .unwrap();
        let Ok(out) = std::process::Command::new("unzip")
            .arg("-t")
            .arg(&path)
            .output()
        else {
            return; // no unzip on this machine (Windows CI)
        };
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stdout)
        );
        assert!(String::from_utf8_lossy(&out.stdout).contains("No errors detected"));
    }

    #[test]
    fn dos_time_reads_the_utc_day() {
        // 2026-10-23 06:40:12 UTC.
        let (time, date) = dos_time(1_792_737_612_000);
        assert_eq!(date >> 9, 2026 - 1980);
        assert_eq!((date >> 5) & 0xf, 10);
        assert_eq!(date & 0x1f, 23);
        assert_eq!(time >> 11, 6);
        assert_eq!((time >> 5) & 0x3f, 40);
        assert_eq!((time & 0x1f) * 2, 12);
        assert_eq!(civil_day(0), (1970, 1, 1));
    }
}
