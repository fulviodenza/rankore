pub mod receiver;
pub mod resampler;
pub mod segmenter;
pub mod session;
pub mod stt;
pub mod transcript;

pub const SAMPLE_RATE_IN: u32 = 48_000;
pub const SAMPLE_RATE_OUT: u32 = 16_000;
