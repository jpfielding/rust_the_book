//! Port of the SIMD example from Mitchell Hashimoto's "Everyone should know SIMD"
//! (https://mitchellh.com/writing/everyone-should-know-simd).
//!
//! Problem: given decoded codepoints, find the end of the next run of printable
//! characters, i.e. the index of the first value <= 0xF (a C0 control character).
//! See TUTORIAL.md for the walkthrough.

use std::hint::black_box;
use std::time::Instant;

const THRESHOLD: u32 = 0xF;

/// Step 0: the scalar reference. Every SIMD version must agree with this.
pub fn printable_end_scalar(cps: &[u32], start: usize) -> usize {
    let mut end = start;
    while end < cps.len() && cps[end] > THRESHOLD {
        end += 1;
    }
    end
}

/// aarch64 NEON: 4 x u32 per vector. NEON is mandatory on aarch64, so no runtime check.
#[cfg(target_arch = "aarch64")]
pub fn printable_end_neon(cps: &[u32], start: usize) -> usize {
    use std::arch::aarch64::*;
    const LANES: usize = 4;
    let mut end = start;

    // SAFETY: NEON is always available on aarch64; every load reads LANES
    // elements starting at `end`, guarded by `end + LANES <= cps.len()`.
    unsafe {
        let threshold = vdupq_n_u32(THRESHOLD); // broadcast ("splat")
        while end + LANES <= cps.len() {
            let values = vld1q_u32(cps.as_ptr().add(end));
            let gt = vcgtq_u32(values, threshold); // each lane: all-ones or zero
            if vminvq_u32(gt) == u32::MAX {
                end += LANES; // every lane passed
                continue;
            }
            // NEON has no movemask. Narrow 4 x u32 -> 4 x u16 (64 bits total),
            // so each lane is a 16-bit group of 1s or 0s in one u64.
            let narrowed = vmovn_u32(gt);
            let bits = vget_lane_u64::<0>(vreinterpret_u64_u16(narrowed));
            end += (!bits).trailing_zeros() as usize / 16; // first failing lane
            return end;
        }
    }

    printable_end_scalar(cps, end)
}

/// x86_64 AVX2: 8 x u32 per vector. Caller must have verified AVX2 support.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn printable_end_avx2(cps: &[u32], start: usize) -> usize {
    use std::arch::x86_64::*;
    const LANES: usize = 8;
    let mut end = start;

    // SAFETY: loads are unaligned and bounded by `end + LANES <= cps.len()`.
    unsafe {
        // AVX2 only has a *signed* 32-bit compare, which is wrong for values
        // >= 2^31. Instead test `v >= 0x10` (same as `v > 0xF`) as
        // `max_epu32(v, 0x10) == v`.
        let min_printable = _mm256_set1_epi32(THRESHOLD as i32 + 1);
        while end + LANES <= cps.len() {
            let values = _mm256_loadu_si256(cps.as_ptr().add(end) as *const __m256i);
            let ge = _mm256_cmpeq_epi32(_mm256_max_epu32(values, min_printable), values);
            // movemask_ps packs the top bit of each 32-bit lane into 8 bits.
            let mask = _mm256_movemask_ps(_mm256_castsi256_ps(ge)) as u32;
            if mask == 0xFF {
                end += LANES;
                continue;
            }
            end += (!mask).trailing_zeros() as usize; // first failing lane
            return end;
        }
    }

    printable_end_scalar(cps, end)
}

/// Public entry point: pick the best implementation for this CPU at runtime.
pub fn printable_end(cps: &[u32], start: usize) -> usize {
    #[cfg(target_arch = "aarch64")]
    {
        return printable_end_neon(cps, start);
    }
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            // SAFETY: AVX2 support was just checked.
            return unsafe { printable_end_avx2(cps, start) };
        }
    }
    #[allow(unreachable_code)]
    printable_end_scalar(cps, start)
}

fn main() {
    // The article's trace: lane 3 is 0x0A, so the run ends at index 3.
    let sample = [0x41, 0x42, 0x43, 0x0A, 0x44, 0x45, 0x46, 0x47];
    println!("sample run ends at {}", printable_end(&sample, 0));

    // One long printable run, then a control character at the very end.
    let n = 1 << 20;
    let mut data = vec![0x41u32; n];
    data[n - 1] = 0x0A;

    bench("scalar", &data, printable_end_scalar);
    bench("simd  ", &data, printable_end);
}

fn bench(name: &str, data: &[u32], f: fn(&[u32], usize) -> usize) {
    const ITERS: u32 = 2000;
    let t = Instant::now();
    for _ in 0..ITERS {
        black_box(f(black_box(data), 0));
    }
    let per = t.elapsed() / ITERS;
    let gb_s = (data.len() * 4) as f64 / per.as_secs_f64() / 1e9;
    println!("{name}: {per:?} per scan ({gb_s:.1} GB/s)");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn impls() -> Vec<(&'static str, Box<dyn Fn(&[u32], usize) -> usize>)> {
        let mut v: Vec<(&'static str, Box<dyn Fn(&[u32], usize) -> usize>)> =
            vec![("dispatch", Box::new(printable_end))];
        #[cfg(target_arch = "aarch64")]
        v.push(("neon", Box::new(printable_end_neon)));
        #[cfg(target_arch = "x86_64")]
        if is_x86_feature_detected!("avx2") {
            v.push(("avx2", Box::new(|c, s| unsafe { printable_end_avx2(c, s) })));
        }
        v
    }

    #[test]
    fn article_trace() {
        let s = [0x41, 0x42, 0x43, 0x0A, 0x44, 0x45, 0x46, 0x47];
        for (name, f) in impls() {
            assert_eq!(f(&s, 0), 3, "{name}");
        }
    }

    #[test]
    fn matches_scalar_for_every_length_and_position() {
        // Control char at every position, for every length crossing the
        // vector-width and tail boundaries; also the no-control-char case.
        for len in 0..40 {
            for ctl in 0..=len {
                let mut d = vec![0x1F600u32; len]; // big values: unsigned-compare check
                if ctl < len {
                    d[ctl] = 0x0;
                }
                for (name, f) in impls() {
                    assert_eq!(
                        f(&d, 0),
                        printable_end_scalar(&d, 0),
                        "{name} len={len} ctl={ctl}"
                    );
                }
            }
        }
    }

    #[test]
    fn threshold_boundary_and_high_bit() {
        let d = [0x10, 0xF, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10];
        let hi = [u32::MAX, 0x8000_0000, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10];
        for (name, f) in impls() {
            assert_eq!(f(&d, 0), 1, "{name}");
            assert_eq!(f(&hi, 0), 8, "{name}");
        }
    }

    #[test]
    fn nonzero_start() {
        let d = [0x0, 0x41, 0x41, 0x41, 0x41, 0x41, 0x0, 0x41];
        for (name, f) in impls() {
            assert_eq!(f(&d, 1), 6, "{name}");
        }
    }
}
