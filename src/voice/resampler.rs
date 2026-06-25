//! 48 kHz stereo s16 PCM -> 16 kHz mono s16 PCM.
//!
//! Songbird's `VoiceTick` already gives us decoded PCM at 48 kHz interleaved
//! stereo. Whisper wants 16 kHz mono. We average L+R, then decimate by 3.
//! This is intentionally simple (no FIR anti-aliasing) — speech in the 8 kHz
//! band has plenty of margin and the model is robust to mild aliasing.
//! If needed later, swap in `rubato` for a proper sinc resampler.

use super::{SAMPLE_RATE_IN, SAMPLE_RATE_OUT};

const DECIM: usize = (SAMPLE_RATE_IN / SAMPLE_RATE_OUT) as usize; // 3

/// `interleaved` is L,R,L,R,... at 48 kHz. Returns 16 kHz mono.
pub fn stereo48k_to_mono16k(interleaved: &[i16]) -> Vec<i16> {
    if interleaved.is_empty() {
        return Vec::new();
    }
    let frames = interleaved.len() / 2;
    let out_len = frames / DECIM;
    let mut out = Vec::with_capacity(out_len);
    let mut i = 0;
    while i + DECIM * 2 <= interleaved.len() {
        // Average across DECIM frames * 2 channels — gives a cheap box-filter.
        let mut acc: i32 = 0;
        for f in 0..DECIM {
            let l = interleaved[i + f * 2] as i32;
            let r = interleaved[i + f * 2 + 1] as i32;
            acc += (l + r) / 2;
        }
        let sample = (acc / DECIM as i32).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        out.push(sample);
        i += DECIM * 2;
    }
    out
}
