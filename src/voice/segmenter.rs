//! Energy-threshold VAD + utterance segmenter.
//!
//! One instance per (user, session). Feed it 16 kHz mono frames; it returns
//! complete utterances ready to send to whisper. Not as good as silero, but
//! has zero deps and is good enough as a starting point.

use super::SAMPLE_RATE_OUT;

const FRAME_MS: u32 = 20;
const FRAME_SAMPLES: usize = (SAMPLE_RATE_OUT * FRAME_MS / 1000) as usize; // 320

/// Default RMS threshold for "speech vs silence" on i16-normalized signal.
const RMS_THRESHOLD: f32 = 300.0;
/// Trailing silence required to close an utterance.
const TRAILING_SILENCE_MS: u32 = 700;
/// Maximum utterance length — force flush past this to keep STT latency bounded.
const MAX_UTTERANCE_MS: u32 = 12_000;
/// Drop utterances shorter than this (almost always noise).
const MIN_UTTERANCE_MS: u32 = 300;

pub struct Segmenter {
    pending: Vec<i16>,    // partial frame buffer
    utterance: Vec<i16>,  // accumulated speech
    in_speech: bool,
    trailing_silence_samples: usize,
}

impl Default for Segmenter {
    fn default() -> Self {
        Self::new()
    }
}

impl Segmenter {
    pub fn new() -> Self {
        Self {
            pending: Vec::with_capacity(FRAME_SAMPLES * 4),
            utterance: Vec::new(),
            in_speech: false,
            trailing_silence_samples: 0,
        }
    }

    /// Push a chunk of 16 kHz mono PCM. Returns 0..N complete utterances.
    pub fn push(&mut self, pcm: &[i16]) -> Vec<Vec<i16>> {
        self.pending.extend_from_slice(pcm);
        let mut out = Vec::new();

        let trailing_silence_limit =
            (SAMPLE_RATE_OUT as usize * TRAILING_SILENCE_MS as usize) / 1000;
        let max_utterance_samples =
            (SAMPLE_RATE_OUT as usize * MAX_UTTERANCE_MS as usize) / 1000;
        let min_utterance_samples =
            (SAMPLE_RATE_OUT as usize * MIN_UTTERANCE_MS as usize) / 1000;

        while self.pending.len() >= FRAME_SAMPLES {
            let frame: Vec<i16> = self.pending.drain(..FRAME_SAMPLES).collect();
            let rms = rms_i16(&frame);
            let is_speech = rms > RMS_THRESHOLD;

            if is_speech {
                if !self.in_speech {
                    self.in_speech = true;
                }
                self.utterance.extend_from_slice(&frame);
                self.trailing_silence_samples = 0;
            } else if self.in_speech {
                // Capture trailing silence as part of the utterance so words
                // don't get cut off, but track how much we've added.
                self.utterance.extend_from_slice(&frame);
                self.trailing_silence_samples += FRAME_SAMPLES;
                if self.trailing_silence_samples >= trailing_silence_limit {
                    if self.utterance.len() >= min_utterance_samples {
                        out.push(std::mem::take(&mut self.utterance));
                    } else {
                        self.utterance.clear();
                    }
                    self.in_speech = false;
                    self.trailing_silence_samples = 0;
                }
            }

            // Bounded buffer — never let an utterance grow past MAX.
            if self.utterance.len() >= max_utterance_samples {
                out.push(std::mem::take(&mut self.utterance));
                self.in_speech = false;
                self.trailing_silence_samples = 0;
            }
        }

        out
    }

    /// Flush any pending speech as a final utterance (used on session end).
    pub fn flush(&mut self) -> Option<Vec<i16>> {
        let min_utterance_samples =
            (SAMPLE_RATE_OUT as usize * MIN_UTTERANCE_MS as usize) / 1000;
        let final_buf = std::mem::take(&mut self.utterance);
        self.in_speech = false;
        self.trailing_silence_samples = 0;
        if final_buf.len() >= min_utterance_samples {
            Some(final_buf)
        } else {
            None
        }
    }
}

fn rms_i16(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    ((sum / samples.len() as f64).sqrt()) as f32
}
