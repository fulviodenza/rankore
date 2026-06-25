# Voice transcription — implementation status

Companion to [`transcription-plan.md`](./transcription-plan.md). The plan was
the original design; this doc tracks the real architecture after we discovered
that Rust's `songbird` cannot connect to DAVE-required voice channels.

## TL;DR

- **Text + scoring side**: Rust (`rankore` bot, this repo) — done, deployed,
  working.
- **Voice + transcription side**: Python `voice-sidecar/` running py-cord —
  done, deployed alongside as a **separate Discord bot account**.
- **STT backend**: `whisper` (faster-whisper-server, `small` multilingual,
  EN/IT/ES/FR) — done, deployed.
- **Shared state**: both bots mount the same `rankore-transcripts` PVC; users
  download transcript files attached to bot replies.

## Why two bots, not one

We initially built voice into the Rust bot using `songbird 0.4`. It connected
fine until Discord rolled out the DAVE end-to-end encryption protocol to the
guild's voice channels. Songbird does not implement DAVE; Discord closes the
voice WebSocket immediately with:

```
WsClosed(Some(CloseFrame { code: Library(4017), reason: "E2EE/DAVE protocol required" }))
```

Upstream tracking issue: <https://github.com/serenity-rs/songbird/issues> (DAVE
support is in progress but unreleased).

py-cord supports voice on DAVE channels today, so we shell voice out to it.
Discord allows only **one gateway connection per bot token**, so the voice
sidecar uses its own bot application/token. Both bots are invited to the same
guild.

This is documented in [`voice-sidecar/README.md`](../voice-sidecar/README.md).

## Architecture

```
┌─────────────────────┐    ┌──────────────────────┐    ┌─────────────────────┐
│ Rankore (Rust)      │    │ Rankore Voice (Py)   │    │ whisper (HTTP)      │
│  serenity 0.12      │    │  py-cord, DAVE-OK    │───▶│  faster-whisper-    │
│  poise 0.6          │    │  joins voice         │    │   server, small     │
│  text commands +    │    │  records to WaveSink │    │   multilingual      │
│  scoring            │    │  POSTs WAV per user  │    └─────────────────────┘
└──────────┬──────────┘    └──────────┬───────────┘
           │                          │
           │                          ▼
           │                ┌──────────────────────┐
           │                │ transcripts PVC      │
           └───────read─────│  /transcripts/*.txt  │
                            └──────────────────────┘
```

## What's in the repo

| Component | Path | Purpose |
|---|---|---|
| Rust bot | `src/` | text commands, scoring, DB (Postgres) |
| Voice sidecar | `voice-sidecar/` | `!transcribe_*` commands, voice receive |
| Whisper Deployment | `k8s/40-whisper.yaml` | OpenAI-compatible STT, multilingual `small` |
| Voice Deployment | `k8s/50-voice-sidecar.yaml` | Python sidecar, mounts shared transcripts PVC |
| Voice secret slot | `k8s/10-secrets.example.yaml` | `rankore-voice-bot` Secret for the second token |

## Setup checklist for a fresh deploy

1. **Build & push the Rust bot image** (no longer carries songbird/audiopus):
   ```sh
   docker buildx build --platform linux/amd64 \
     -t ghcr.io/fulviodenza/rankore:latest --push .
   ```
2. **Build & push the voice sidecar image**:
   ```sh
   docker buildx build --platform linux/amd64 \
     -t ghcr.io/fulviodenza/rankore-voice:latest --push voice-sidecar
   ```
3. **Create a second Discord application** (Developer Portal → New App).
   - Bot tab → Reset Token → copy
   - Enable Message Content Intent + Server Members Intent
   - OAuth2 → URL Generator → scopes `bot`, perms: Connect, Speak, Send
     Messages, Embed Links, Attach Files, Read Message History
   - Open URL, invite to your guild
4. **Fill the Secret** for `rankore-voice-bot` in `k8s/10-secrets.yaml`,
   then `kubectl apply -f k8s/10-secrets.yaml`.
5. **Apply** `k8s/50-voice-sidecar.yaml`.
6. Test with `!transcribe_join` → speak → `!transcribe_leave` in Discord.

## Known limitations

- **Per-utterance timestamps**: py-cord's `WaveSink` doesn't expose them;
  all transcript lines are stamped with the flush time. Acceptable for short
  sessions; for accurate time alignment, write a custom Sink that flushes on
  silence intervals.
- **All STT runs at `!transcribe_leave`**, not streaming during the call.
  Switching to streaming requires a VAD-based custom Sink and incremental
  whisper POSTs.
- **One concurrent session per guild.** A second `!transcribe_join` while
  one is active rejects.
- **Two bot accounts in your member list.** Unavoidable consequence of
  Discord's one-gateway-per-token rule. You can name them however you like.

## Future work

- Streaming captions to a text channel during the call (incremental STT).
- Per-user opt-out command (the bot already supports dropping audio for
  specific user IDs; just needs the command surface).
- Auto-rotate transcripts older than N days (CronJob).
- Replace WaveSink with a VAD-based custom Sink for tighter timestamps.
- When songbird ships DAVE support, optionally fold voice back into the
  Rust bot and retire the sidecar — though the two-process split is also a
  reasonable long-term architecture.
