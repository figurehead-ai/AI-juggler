//! The puzzle the service may ask for under load (spec §5.5): a nonce
//! such that `SHA-256("ajstats-pow-v1:" ‖ report_id ‖ ":" ‖ week ‖ ":" ‖
//! nonce)` starts with `bits` zero bits. Bound to the report and week, so
//! a nonce can't be reused.

use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

/// Above this the service is under heavy attack: postpone instead of
/// spending battery.
pub const MAX_BITS: u32 = 28;
/// Give up after this long and try again next tick.
pub const BUDGET: Duration = Duration::from_secs(60);

/// Leading zero bits of a digest.
#[must_use]
pub fn leading_zero_bits(digest: &[u8]) -> u32 {
    let mut bits = 0;
    for byte in digest {
        if *byte == 0 {
            bits += 8;
        } else {
            bits += byte.leading_zeros();
            break;
        }
    }
    bits
}

/// The digest for one nonce.
#[must_use]
pub fn digest(report_id: &str, week: &str, nonce: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"ajstats-pow-v1:");
    hasher.update(report_id.as_bytes());
    hasher.update(b":");
    hasher.update(week.as_bytes());
    hasher.update(b":");
    hasher.update(nonce.as_bytes());
    hasher.finalize().into()
}

/// `nonce` solves the puzzle at `bits`.
#[must_use]
pub fn check(report_id: &str, week: &str, nonce: &str, bits: u32) -> bool {
    !nonce.is_empty()
        && nonce.len() <= 32
        && nonce
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        && leading_zero_bits(&digest(report_id, week, nonce)) >= bits
}

/// Counts nonces up from 0 (lowercase hex) until one solves the puzzle,
/// or `budget` runs out.
#[must_use]
pub fn solve(report_id: &str, week: &str, bits: u32, budget: Duration) -> Option<String> {
    let started = Instant::now();
    let mut prefix = Sha256::new();
    prefix.update(b"ajstats-pow-v1:");
    prefix.update(report_id.as_bytes());
    prefix.update(b":");
    prefix.update(week.as_bytes());
    prefix.update(b":");
    let mut buffer = [0u8; 16];
    for n in 0u64.. {
        if n % 65_536 == 0 && started.elapsed() > budget {
            return None;
        }
        let nonce = hex_of(n, &mut buffer);
        let mut hasher = prefix.clone();
        hasher.update(nonce);
        if leading_zero_bits(&hasher.finalize()) >= bits {
            return Some(String::from_utf8_lossy(nonce).into_owned());
        }
    }
    None
}

/// `n` in lowercase hex without leading zeros, written into `buffer`.
fn hex_of(n: u64, buffer: &mut [u8; 16]) -> &[u8] {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut at = buffer.len();
    let mut rest = n;
    loop {
        at -= 1;
        buffer[at] = DIGITS[usize::try_from(rest & 0xf).unwrap_or(0)];
        rest >>= 4;
        if rest == 0 {
            return &buffer[at..];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_and_checks() {
        let id = "0b6f2d4e-6a3c-4f0e-9d7b-3c1e5a9f8b21";
        let nonce = solve(id, "2026-W42", 12, BUDGET).unwrap();
        assert!(check(id, "2026-W42", &nonce, 12));
        // Bound to the report and the week.
        assert_ne!(
            digest(id, "2026-W42", &nonce),
            digest(id, "2026-W43", &nonce)
        );
        assert_ne!(
            digest(id, "2026-W42", &nonce),
            digest("another", "2026-W42", &nonce)
        );
        assert!(!check(id, "2026-W42", "XYZ", 0));
        assert!(!check(id, "2026-W42", &"0".repeat(33), 0));
        assert_eq!(leading_zero_bits(&[0, 0b0001_0000]), 11);
        assert_eq!(leading_zero_bits(&[0xff]), 0);
    }

    /// Nonces found by counting up from 0, checked on 2026-10-18 with the
    /// service's own verifier run locally (AJ-E1108 evidence).
    const VECTORS: &[(&str, &str, u32, &str)] = &[
        ("0b6f2d4e-6a3c-4f0e-9d7b-3c1e5a9f8b21", "2026-W42", 8, "ab"),
        (
            "0b6f2d4e-6a3c-4f0e-9d7b-3c1e5a9f8b21",
            "2026-W42",
            12,
            "2a07",
        ),
        (
            "0b6f2d4e-6a3c-4f0e-9d7b-3c1e5a9f8b21",
            "2026-W42",
            16,
            "7c10",
        ),
        (
            "0b6f2d4e-6a3c-4f0e-9d7b-3c1e5a9f8b21",
            "2026-W42",
            20,
            "7c10",
        ),
        ("1d4f3c3e-2f4e-4a8a-9d3e-5b8f7a6c4e21", "2026-W43", 8, "22"),
        (
            "1d4f3c3e-2f4e-4a8a-9d3e-5b8f7a6c4e21",
            "2026-W43",
            12,
            "28ee",
        ),
        (
            "1d4f3c3e-2f4e-4a8a-9d3e-5b8f7a6c4e21",
            "2026-W43",
            16,
            "17d0d",
        ),
        (
            "1d4f3c3e-2f4e-4a8a-9d3e-5b8f7a6c4e21",
            "2026-W43",
            20,
            "444cb",
        ),
    ];

    #[test]
    fn the_solver_reproduces_the_checked_vectors() {
        for (id, week, bits, nonce) in VECTORS {
            assert!(check(id, week, nonce, *bits), "{id} {week} {bits}");
            assert_eq!(
                solve(id, week, *bits, BUDGET).as_deref(),
                Some(*nonce),
                "{id} {week} {bits}"
            );
            assert!(!check(id, "2026-W01", nonce, *bits));
        }
    }

    #[test]
    fn twenty_bits_is_quick() {
        let started = Instant::now();
        let nonce = solve(
            "1d4f3c3e-2f4e-4a8a-9d3e-5b8f7a6c4e21",
            "2026-W42",
            20,
            BUDGET,
        )
        .unwrap();
        let took = started.elapsed();
        println!("pow: 20 bits in {} ms (nonce {nonce})", took.as_millis());
        assert!(check(
            "1d4f3c3e-2f4e-4a8a-9d3e-5b8f7a6c4e21",
            "2026-W42",
            &nonce,
            20
        ));
    }

    #[test]
    fn hex_matches_format() {
        let mut buffer = [0u8; 16];
        for n in [0, 1, 15, 16, 255, 0x444cb, u64::MAX] {
            assert_eq!(hex_of(n, &mut buffer), format!("{n:x}").as_bytes());
        }
    }

    #[test]
    fn a_budget_stops_the_search() {
        assert_eq!(solve("x", "2026-W42", 200, Duration::ZERO), None);
    }
}
