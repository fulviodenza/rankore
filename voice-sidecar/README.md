# rankore-voice — voice transcription sidecar

A Python (py-cord) Discord bot that handles voice channel join / receive /
transcribe. Runs alongside the main Rust `rankore` bot.

## Why a separate process / bot account

The main Rust bot uses [`songbird`](https://github.com/serenity-rs/songbird)
for voice, which as of 2025 does not implement Discord's DAVE end-to-end
encryption protocol. Voice channels that require DAVE refuse the connection
(WS close code 4017 "E2EE/DAVE protocol required"). DAVE is increasingly
mandatory across Discord.

**Stable py-cord can't do voice receive on DAVE channels either** — it emits a
`RuntimeWarning: Voice reception is currently broken due to Discord's DAVE
(End-to-End Encryption) protocol` and yields no audio. There's an active fix
in [pycord PR #3159](https://github.com/Pycord-Development/pycord/pull/3159)
which we pin to in `requirements.txt`.

Discord requires one gateway connection per bot token, so this sidecar uses a
**second Discord application** (its own bot account). Both bots are invited
to the same server.

## Architecture

```
              ┌──────────────────────────┐
              │ Discord guild            │
              │                          │
  text cmds   │  ┌───────────────────┐   │  voice
  scoring     │  │  Rankore (Rust)   │   │  capture
  (!leader,   │  └───────────────────┘   │  + STT
   !set_*,                               │
   !help, ...)│  ┌───────────────────┐   │  (this service)
              │  │ Rankore Voice (Py)│←──┼──── joins voice
              │  └─────────┬─────────┘   │     channel on
              └────────────┼─────────────┘     !transcribe_join
                           │
                           │ POST WAV
                           ▼
              ┌──────────────────────────┐
              │ whisper (faster-whisper) │
              │   /v1/audio/transcriptions
              └─────────────┬────────────┘
                            │ text
                            ▼
              ┌──────────────────────────┐
              │ transcripts PVC          │
              │  /transcripts/*.txt      │
              └──────────────────────────┘
```

## Build and push

```sh
docker buildx build --platform linux/amd64 \
  -t ghcr.io/fulviodenza/rankore-voice:latest \
  --push voice-sidecar
```

## Configure and deploy

1. Create a second Discord application at https://discord.com/developers/applications
2. → Bot → **Reset Token** → copy
3. → Bot → enable **Message Content Intent**, **Server Members Intent**, **Presence Intent** (only the first two are required; presence is optional)
4. → OAuth2 → URL Generator → scopes `bot`, perms: View Channels, Send Messages, Embed Links, Attach Files, Read Message History, **Connect**, **Speak**
5. Open the URL, invite to your server
6. Put the token in the `rankore-voice-bot` Secret (see `k8s/10-secrets.example.yaml`)
7. `kubectl apply -f k8s/50-voice-sidecar.yaml`

## Local test

```sh
docker build -t rankore-voice voice-sidecar
docker run --rm \
  -e DISCORD_TOKEN=... \
  -e WHISPER_URL=http://host.docker.internal:8000 \
  -e TRANSCRIPTS_DIR=/tmp/transcripts \
  -v /tmp/rankore-transcripts:/tmp/transcripts \
  rankore-voice
```

## Commands

| Command | Action |
|---|---|
| `!transcribe_join` | Join caller's voice channel, auto-detect language |
| `!transcribe_join lang=it` | Join with language pinned to `it` (also `en`/`es`/`fr`) |
| `!transcribe_leave` | Disconnect, transcribe captured audio, reply with the transcript file |
| `!transcribe_status` | Show active session info |

## Limitations

- **Per-user timestamping is whole-session**, not per-utterance. py-cord's `WaveSink` buffers per-user audio without exposing per-segment timestamps. If you need real time-aligned transcripts, swap in a custom Sink that flushes on silence intervals.
- **All transcription happens at `!transcribe_leave`**, not streaming during the call. Switching to streaming requires a VAD-based custom Sink and incremental whisper POSTs.
- **One session per guild.** A second `!transcribe_join` in the same guild rejects.
