# Voice transcription — implementation status

Companion to [`transcription-plan.md`](./transcription-plan.md). The plan was
the design; this doc tracks what is **actually implemented** on the
`transcription` branch and what still needs runtime validation against a real
Discord guild.

## What changed

### Framework upgrade
- **serenity 0.11 → 0.12**, replacing `StandardFramework` with **poise 0.6**.
- All 10 existing commands rewritten as `#[poise::command(prefix_command, ...)]`
  with typed arguments. Behavior preserved.
- Event handlers consolidated into a single `event_handler` fn on poise's
  `FrameworkOptions`.
- `Data` struct replaces the old `GlobalStateInner` + `TypeMap` indirection.

### Dependency conflict resolved
- Songbird 0.3 was incompatible with modern sqlx via a transitive
  `xsalsa20poly1305` / `zeroize` constraint chain. Upgrading serenity unlocked
  songbird 0.4 which uses modern crypto.

### Voice pipeline (new `src/voice/` module)
- `session.rs` — per-guild `TranscriptSession` (channel, language, file path,
  opted-out users, writer handle).
- `transcript.rs` — append-only text file writer, format
  `[HH:MM:SS] @nick: text`.
- `resampler.rs` — 48 kHz stereo s16 → 16 kHz mono s16, simple box-filter
  decimation by 3.
- `segmenter.rs` — RMS-threshold VAD with 20 ms frames, 700 ms trailing
  silence to close an utterance, 12 s force-flush ceiling, 300 ms minimum to
  drop noise.
- `stt.rs` — `WhisperClient` posting WAV to faster-whisper-server's
  OpenAI-compatible `/v1/audio/transcriptions`.
- `receiver.rs` — songbird `EventHandler` hooking `VoiceTick` (per-speaker
  decoded PCM) and `SpeakingStateUpdate` (SSRC ↔ user_id mapping). Opted-out
  users drop at SSRC mapping before audio buffers; utterances dispatch to
  STT on background tasks and append to the session file.

### New commands (`src/commands/transcribe.rs`)
- `transcribe_join` — joins the caller's current voice channel.
  Accepts `lang=en|it|es|fr`; default is auto-detect. Whitelist enforced.
- `transcribe_leave` — disconnects, sends the transcript file.
- `transcribe_status` — shows current session info and opt-out count.

### Kubernetes manifests
- `k8s/40-whisper.yaml` — faster-whisper-server Deployment + headless Service
  + 2 Gi model cache PVC. Sized for `small` multilingual model (1 CPU req /
  4 CPU lim, 512Mi req / 1Gi lim).
- `k8s/30-bot.yaml` — bot Deployment updated with `WHISPER_URL`,
  `TRANSCRIPTS_DIR`, and a 5 Gi transcripts PVC.

### Dockerfile
- Adds `libopus-dev` / `cmake` / `build-essential` to the builder stage and
  `libopus0` to the runtime stage (needed by songbird's `audiopus`).

## What has been verified

- All k8s manifests pass `kubectl --dry-run=client`.
- Dependency resolution (`cargo update`) succeeds with the new tree.
- Code is structured against the **actual** songbird 0.4 / serenity 0.12 /
  poise 0.6 sources (I read the crate sources on disk to confirm
  `VoiceTick.speaking: HashMap<u32, VoiceData>`, `Speaking.user_id`,
  `EventContext` variants, `PartialContext` shape for dynamic_prefix,
  `FullEvent` variant names, `SerenityInit::register_songbird`,
  `manager.join` signature, etc.).

## What has NOT been verified

- **`cargo check` did not run locally.** The local macOS toolchain failed
  building `audiopus_sys` from source (Xcode 26 + bundled CMakeLists.txt
  declares `cmake_minimum_required(VERSION 2.6)` which modern cmake refuses).
  This is a dev-environment issue, not a project issue — Linux Docker has
  no such problem.
- **Docker build did not run** during this session (local Docker daemon was
  unavailable mid-session). The Dockerfile is updated with the right system
  packages but a Linux build is the first real type-check.
- **Nothing has been tested against Discord.** Voice receive in particular
  is the kind of integration that always needs real-bot iteration: SSRC
  timing edge cases, opus decoder hiccups, network jitter behavior of the
  segmenter.

## Validation checklist (for the first deploy)

1. `docker build -t ghcr.io/fulviodenza/rankore:transcribe .` — first
   compile is the smoke test. Expect 1–2 type errors to fix on the songbird
   handler glue; iterate.
2. Push image, `kubectl apply -f k8s/40-whisper.yaml`, wait for the model
   to download (~190 MB on first request, cached after).
3. `kubectl apply -f k8s/30-bot.yaml`, watch `kubectl logs -f`.
4. In Discord: join a voice channel, then run `!transcribe_join`. Speak.
   Run `!transcribe_leave` and check the attached transcript.
5. If transcription quality on Italian / Spanish / French is poor, try
   `lang=it` (etc.) instead of auto-detect.

## Known follow-ups

- **Opt-out commands** (`transcribe_optout` / `transcribe_optin`) are
  declared in the plan but not yet implemented. The receiver already
  honors the per-session `opted_out: HashSet<UserId>`, so adding the
  commands is mechanical: insert/remove from `session.opted_out`.
- **Graceful shutdown** of segmenters on `transcribe_leave` doesn't flush
  any in-flight partial utterance. Wire `Segmenter::flush()` into the
  leave path before discarding the receiver.
- **Memory** — per-SSRC segmenters are never GC'd within a session. Add a
  reaper that drops segmenters whose `tick.silent` flag has held for >60s.
- **VAD quality** — the RMS-threshold segmenter is fine for clear voice
  but will mis-segment in a noisy channel. Swap in `silero-vad` (ONNX
  Runtime) or `webrtc-vad` (C bindings) if needed.
- **Real-time captions** — currently the transcript is written to a file
  and delivered at session end. Posting per-utterance lines into a text
  channel during the call is a small change (call `channel_id.say` in
  `dispatch_utterance`).
