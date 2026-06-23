# Voice-to-text transcription — design and plan

Goal: while the bot is in a Discord voice channel, capture each user's audio,
transcribe it, and append the result to a per-session text file the server can
download via a chat command.

This is a non-trivial feature: Discord's voice receive API is undocumented and
unstable, the audio pipeline is real-time, and there are consent/legal
considerations distinct from the rest of the bot. Below is the design and a
phased plan, not an implementation.

---

## 1. Constraints and scope

- **Discord voice ingest is unofficial.** Discord supports voice send through
  documented APIs but voice *receive* is not officially supported. The standard
  workaround is the [Songbird](https://github.com/serenity-rs/songbird) crate,
  which speaks the same gateway as Discord clients and decodes the per-speaker
  Opus streams. Songbird supports voice receive but the underlying packet
  format has changed before and may again.
- **The bot has to *join* the channel.** It cannot eavesdrop on a channel it
  isn't in. Joining is a visible action; users will see "Rankore joined voice."
- **Consent.** The bot must announce that recording is active when it joins
  and ideally support per-user opt-out before any transcript is persisted.
  Some jurisdictions (and Discord's own terms) require all-party consent for
  voice recording.
- **Songbird ↔ serenity version coupling.** Songbird `0.3.x` pairs with
  serenity `0.11.x` (what we're on); newer songbird requires newer serenity.
  Upgrading serenity is its own task and bundles unrelated API churn.

## 2. Architecture

```
        ┌──────────────────────────────────────────────────────────┐
        │ Discord voice gateway (UDP)                              │
        │   per-speaker Opus packets, 48kHz/20ms frames            │
        └─────────────────────┬────────────────────────────────────┘
                              │
                  ┌───────────▼──────────┐
                  │ Songbird driver      │  bot process (Rust)
                  │ — VoiceReceiveEvent  │
                  └───────────┬──────────┘
                              │ per-(guild, channel, user) Opus stream
                              │
                  ┌───────────▼──────────┐
                  │ Decoder + resampler  │  opus -> PCM s16 48k stereo
                  │ (opus, rubato)       │  -> PCM s16 16k mono
                  └───────────┬──────────┘
                              │ 16k mono PCM
                              │
                  ┌───────────▼──────────┐
                  │ Voice-activity       │  silero-vad-rs or webrtc-vad
                  │ detector / segmenter │  -> utterance buffers (~0.5–10s)
                  └───────────┬──────────┘
                              │ utterance WAV
                              │  (HTTP POST)
                  ┌───────────▼──────────┐
                  │ whisper service      │  separate k8s Deployment,
                  │ (whisper.cpp HTTP)   │  CPU or GPU node, model on PVC
                  └───────────┬──────────┘
                              │ text + start/end timestamps
                              │
                  ┌───────────▼──────────┐
                  │ Transcript writer    │  appends to per-session file on PVC
                  └──────────────────────┘
```

### Why a separate STT service

Keeping Whisper out of the bot process has three benefits:
1. **Decoupled scaling.** STT is compute-heavy; the bot is event-driven. Sizing
   them together is wasteful.
2. **Restart safety.** A whisper.cpp crash doesn't take the bot down.
3. **Model swap.** Tiny/base/medium/large are a config change, not a redeploy.

The bot talks HTTP to it (e.g., `POST /transcribe` with `audio/wav` body).

## 3. Engine choice — sized for low-cost hardware

Target: a single k3s node with 2–4 CPU cores and a few GB of free RAM, no GPU.
This rules out anything from `medium` upward; even `small` is borderline once
the bot, Postgres, and other workloads are on the same node.

### Model shortlist

| Model | Params | RAM (Q5_0) | Disk (Q5_0) | RTF on 4×x86_64 CPU | Languages | Quality |
|---|---|---|---|---|---|---|
| whisper.cpp `tiny.en`   | 39M  | ~75 MB  | ~30 MB  | ~0.10× | English only | usable for simple speech |
| whisper.cpp `base.en`   | 74M  | ~140 MB | ~60 MB  | ~0.20× | English only | clearly better; recommended |
| whisper.cpp `tiny`      | 39M  | ~75 MB  | ~30 MB  | ~0.10× | multilingual | mediocre |
| whisper.cpp `base`      | 74M  | ~140 MB | ~60 MB  | ~0.20× | multilingual | acceptable |
| whisper.cpp `small.en`  | 244M | ~500 MB | ~190 MB | ~0.50× | English only | great if you have headroom |
| Vosk `small-en-us`      | —    | ~80 MB  | ~40 MB  | ~0.05× | English only | worse than whisper but very efficient |

RTF = real-time factor; RTF < 1.0 means faster than real time, so a single
4-core box can keep up with one speaker.

### Recommendation

- **English-only servers:** `whisper.cpp base.en` with Q5_0 quantization.
  ~140 MB RAM, ~60 MB on disk, comfortably real-time on 2 CPU cores. Quality
  is solidly better than `tiny.en` for not much more cost.
- **Multilingual servers:** `whisper.cpp base` Q5_0. Same footprint.
- **Sub-Raspberry-Pi-class hardware:** `tiny.en` Q5_0. Still useful.
- **Lots of CPU headroom and English-only:** jump to `small.en` Q5_0 for
  noticeably better accuracy. Skip anything larger; `medium` needs ~2 GB RAM
  and an RTF around 1.0 on 4 cores, i.e. exactly real-time with no margin.

Quantized GGML models are at https://huggingface.co/ggerganov/whisper.cpp.
Pull the `*-q5_0.bin` variant.

### Engines considered and rejected

| Option | Why not |
|---|---|
| OpenAI Whisper API ($0.006/min) | Audio leaves the cluster; defeats the homelab/privacy point. |
| Deepgram / AssemblyAI streaming | External, paid, lowest latency — keep as a fallback option, not the default. |
| faster-whisper (CTranslate2) | Faster per watt on CPU, but adds a Python sidecar. Only worth it if `base.en` can't keep up; revisit then. |
| `small` / `medium` / `large` whisper | RAM and CPU cost outside the "low-cost hardware" envelope. |

### Settling latency vs accuracy at runtime

Make the model a config knob on the whisper Deployment (`WHISPER_MODEL=base.en`),
not a build-time decision. The model file lives on a PVC so swapping it is a
restart, not a rebuild.

## 4. Audio plumbing details

- Discord sends each speaker as a separate SSRC. Songbird's
  `EventContext::VoiceTick` (newer) or `EventContext::SpeakingUpdate` /
  `VoicePacket` (older) maps SSRC ↔ user_id, with a small race window at the
  start of a user speaking.
- Each user gets a ring buffer (e.g. 60s capacity). VAD slices speech segments.
- Speech segments shorter than ~300ms get dropped; longer than ~10s get
  force-flushed to keep latency bounded.
- Resampling: 48k stereo → 16k mono via `rubato` (sinc) or `dasp`. Channel
  downmix is `(L+R)/2`.

## 5. Discord-side commands and UX

- `!transcribe_join` — bot joins the caller's current voice channel and starts
  capturing. Replies with a consent banner naming the active session.
- `!transcribe_leave` — bot leaves and finalizes the transcript. Replies with
  a summary and the transcript file (using the existing `send_titled_files`
  path).
- `!transcribe_status` — current session info (channel, duration, opted-out
  users).
- `!transcribe_optout` / `!transcribe_optin` — per-user toggle. Opt-out users'
  audio is dropped at the SSRC mapping step before it reaches any disk.

Sessions are keyed by `(guild_id, channel_id, started_at)`.

## 6. Storage

- Transcripts: append-only `.txt` per session, on a PVC mounted into the bot.
  Format: `[HH:MM:SS] @nick: utterance text`.
- Retention: configurable, default 30 days; cleanup CronJob in the namespace.
- Raw audio: **not retained** unless an explicit debug flag is set. Discarded
  after the utterance is transcribed.

## 7. Kubernetes shape

Two new workloads in the `rankore` namespace:

- `whisper` Deployment + Service
  - image: `ghcr.io/ggerganov/whisper.cpp:server` (or a self-built thin image)
  - PVC for the model file (~60 MB for `base.en` Q5_0) so the model isn't
    re-downloaded on restart
  - resources sized for `base.en` Q5_0:
    - requests: 500m CPU / 256Mi RAM
    - limits:   2 CPU / 512Mi RAM
  - If you later swap to `small.en`, bump the limits to 4 CPU / 1Gi RAM.
- The existing `rankore` Deployment gets a transcripts PVC mount and a
  `WHISPER_URL=http://whisper:9000` env var.

Optional: a `transcript-pruner` CronJob that removes files older than the
retention window.

## 8. Dependencies (Rust side)

```toml
songbird = { version = "0.3", default-features = false, features = [
    "serenity", "rustls", "driver", "gateway",
] }
audiopus = "0.3"          # Opus decode
rubato = "0.14"           # resampling
hound = "3.5"             # WAV write for HTTP body
silero-vad = "0.x"        # or webrtc-vad = "0.4"
reqwest = { version = "0.11", features = ["multipart", "stream"] }
```

`audiopus` builds against `libopus`; the runtime image needs it (`apt-get
install libopus0`).

## 9. Phased rollout

Each phase is independently shippable.

1. **Songbird wiring + raw audio dump.** Add commands `!transcribe_join` /
   `!transcribe_leave`. Bot joins voice and writes per-user raw PCM to a tmp
   file. No STT yet. Validates that voice receive works on this serenity
   version, that SSRC ↔ user mapping is reliable, and that the consent UX
   feels right.
2. **whisper.cpp service in-cluster.** Stand up the whisper Deployment +
   Service + model PVC. Smoke test with `curl` from a debug pod.
3. **STT integration.** Bot resamples + segments PCM, posts utterances to the
   whisper service, appends transcript lines to a session file. Single user
   first, then multi-speaker.
4. **VAD-driven segmenter.** Replace the naive fixed-window segmenter with VAD
   to cut silence and improve word boundaries.
5. **Opt-out + consent UX polish.** Per-user opt-out, in-channel banner on
   join, transcript header naming participants who consented.
6. **Retention CronJob.** Configurable TTL.

Estimated effort: phases 1–3 are the bulk (~2–3 weeks of focused work);
phases 4–6 are smaller polish (~1 week combined).

## 10. Risks

- **Songbird voice-receive API drift.** Mitigation: pin the songbird version,
  keep the receive code in one module behind a small trait so swapping it is
  isolated.
- **Latency under load.** Multiple concurrent speakers × CPU whisper may
  fall behind real time. Mitigation: per-user back-pressure (drop segments
  rather than queue unboundedly); the segmenter already bounds buffer length.
- **Discord ToS / privacy law.** Mitigation: explicit consent banner,
  per-user opt-out, no raw audio retention by default, clear documentation
  of retention.
- **Memory.** Recommended `base.en` Q5_0: ~140 MB resident. `small.en` Q5_0:
  ~500 MB. `medium` and up are intentionally out of scope for the homelab
  hardware budget.
- **serenity 0.11 EOL.** Newer songbird requires serenity 0.12+. If we have
  to upgrade serenity, the bulk of the bot code stays the same but the
  builder APIs and some event signatures change. Worth scoping that upgrade
  separately before phase 1 if we hit blockers.

## 11. Out of scope (for now)

- Real-time captions posted into a text channel during the call.
- Speaker diarization beyond what Discord gives us (Discord already gives us
  per-speaker streams, so this is mostly free).
- Translation.
- Multi-language detection per utterance (rely on whisper's `--language`
  per session, set when joining).
