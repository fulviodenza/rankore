//! HTTP client for the in-cluster whisper service.
//!
//! Speaks the OpenAI-compatible `/v1/audio/transcriptions` API exposed by
//! faster-whisper-server. Sends a WAV blob, gets back JSON `{ "text": "..." }`.

use std::io::Cursor;

use reqwest::multipart::{Form, Part};
use serde::Deserialize;

use super::SAMPLE_RATE_OUT;

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: String,
}

pub struct WhisperClient {
    http: reqwest::Client,
    base_url: String,
}

impl WhisperClient {
    pub fn new(http: reqwest::Client, base_url: String) -> Self {
        Self { http, base_url }
    }

    /// `pcm16k_mono` is 16 kHz mono i16 PCM. `language` is an ISO-639-1 code
    /// or None for auto-detect.
    pub async fn transcribe(
        &self,
        pcm16k_mono: &[i16],
        language: Option<&str>,
    ) -> Result<String, reqwest::Error> {
        let wav_bytes = pcm_to_wav(pcm16k_mono);

        let part = Part::bytes(wav_bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .expect("static mime type is valid");
        let mut form = Form::new()
            .part("file", part)
            .text("model", "Systran/faster-whisper-small")
            .text("response_format", "json");
        if let Some(lang) = language {
            form = form.text("language", lang.to_string());
        }

        let url = format!("{}/v1/audio/transcriptions", self.base_url);
        let resp: TranscriptionResponse = self
            .http
            .post(&url)
            .multipart(form)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp.text.trim().to_string())
    }
}

fn pcm_to_wav(pcm: &[i16]) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::with_capacity(pcm.len() * 2 + 44);
    let cursor = Cursor::new(&mut buf);
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE_OUT,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::new(cursor, spec).expect("wav writer");
    for &s in pcm {
        writer.write_sample(s).expect("wav write");
    }
    writer.finalize().expect("wav finalize");
    buf
}
