"""Rankore voice sidecar — handles Discord voice receive + transcription.

Runs as a SEPARATE Discord bot account from the main Rust bot. py-cord
supports DAVE, which the songbird-based Rust bot does not. Both bots are
in the guild; the Rust bot owns text commands + scoring, this one owns
the `!transcribe_*` commands and voice receive.

Audio path:
  Discord voice (DAVE) -> py-cord VoiceClient -> WaveSink (per-user WAV)
    -> on !transcribe_leave, POST each user's WAV to whisper
    -> append transcript lines to a session text file on the shared PVC
    -> reply to the invocation channel with the file attached.
"""
import asyncio
import audioop
import io
import logging
import os
import re
import sys
import wave
from datetime import datetime, timezone
from pathlib import Path

import aiohttp
import discord
from discord.ext import commands
from discord.sinks import WaveSink

# ---------------------------------------------------------------- config / env
DISCORD_TOKEN = os.environ["DISCORD_TOKEN"]
WHISPER_URL = os.environ.get("WHISPER_URL", "http://whisper:8000")
WHISPER_MODEL = os.environ.get("WHISPER_MODEL", "Systran/faster-whisper-small")
TRANSCRIPTS_DIR = Path(os.environ.get("TRANSCRIPTS_DIR", "/transcripts"))
TRANSCRIPTS_DIR.mkdir(parents=True, exist_ok=True)

ALLOWED_LANGS = {"en", "it", "es", "fr"}
# A single user's WAV from a long call can be many minutes. Whisper-small on CPU
# transcribes roughly 0.5–1x realtime, so a 10-min chunk can take ~10 min. We
# disable the overall deadline and only guard against a stuck socket.
HTTP_TIMEOUT = aiohttp.ClientTimeout(total=None, sock_read=600)

# RMS-based VAD config. Split each user's WAV on silence rather than at fixed
# offsets — pure-silence stretches never reach whisper (no hallucinated 'no, no,
# no…' loops), and chunk boundaries fall between utterances instead of mid-word.
VAD_WINDOW_MS = 20
VAD_RMS_THRESHOLD = int(os.environ.get("VAD_RMS_THRESHOLD", "500"))
VAD_MIN_SILENCE_MS = int(os.environ.get("VAD_MIN_SILENCE_MS", "700"))
VAD_MAX_CHUNK_S = int(os.environ.get("VAD_MAX_CHUNK_S", "30"))
VAD_MIN_CHUNK_MS = int(os.environ.get("VAD_MIN_CHUNK_MS", "300"))

logging.basicConfig(
    level=os.environ.get("LOG_LEVEL", "INFO"),
    format="%(asctime)s %(levelname)s %(name)s %(message)s",
    stream=sys.stdout,
)
log = logging.getLogger("rankore-voice")

# ---------------------------------------------------------------- bot setup
intents = discord.Intents.default()
intents.message_content = True
intents.voice_states = True
intents.guilds = True
intents.members = True

bot = commands.Bot(command_prefix="!", intents=intents)

# guild_id -> session dict
# {voice_client, text_channel, file_path, language, started_at}
sessions: dict[int, dict] = {}


# ---------------------------------------------------------------- helpers
def parse_lang(args: str) -> str | None:
    """Return language code from 'lang=xx' or None. Raises ValueError on bad lang."""
    raw = (args or "").strip()
    if not raw:
        return None
    if raw.startswith(("lang=", "language=")):
        code = raw.split("=", 1)[1].strip().lower()
    else:
        code = raw.strip().lower()
    if code not in ALLOWED_LANGS:
        raise ValueError(f"language must be one of {sorted(ALLOWED_LANGS)}, got {code!r}")
    return code


async def transcribe_wav(
    http: aiohttp.ClientSession, wav_bytes: bytes, language: str | None
) -> str:
    """POST a WAV blob to whisper, return the transcript text (may be empty)."""
    form = aiohttp.FormData()
    form.add_field(
        "file", wav_bytes, filename="audio.wav", content_type="audio/wav"
    )
    form.add_field("model", WHISPER_MODEL)
    form.add_field("response_format", "json")
    if language:
        form.add_field("language", language)
    url = f"{WHISPER_URL}/v1/audio/transcriptions"
    async with http.post(url, data=form, timeout=HTTP_TIMEOUT) as resp:
        resp.raise_for_status()
        payload = await resp.json()
    return (payload.get("text") or "").strip()


def chunk_wav_vad(wav_bytes: bytes) -> list[tuple[float, bytes]]:
    """Split a WAV into per-utterance chunks using RMS-based voice activity detection.

    Walks the PCM in fixed-size windows, classifies each window as speech vs
    silence by RMS threshold, and emits a chunk for every run of speech
    separated from the next by ≥ VAD_MIN_SILENCE_MS of silence. Long utterances
    are force-split at VAD_MAX_CHUNK_S to keep each whisper request bounded.

    Returns (offset_seconds, wav_bytes) where offset is elapsed time within the
    source WAV. WaveSink concatenates voice packets without silence, so this
    is elapsed speech time for this user, not wall-clock from session start.
    """
    chunks: list[tuple[float, bytes]] = []
    with wave.open(io.BytesIO(wav_bytes), "rb") as src:
        framerate = src.getframerate()
        nchannels = src.getnchannels()
        sampwidth = src.getsampwidth()
        total_frames = src.getnframes()
        if total_frames == 0:
            return chunks
        pcm = src.readframes(total_frames)

    bytes_per_frame = nchannels * sampwidth
    window_frames = max(1, framerate * VAD_WINDOW_MS // 1000)
    window_bytes = window_frames * bytes_per_frame
    if window_bytes == 0 or len(pcm) < window_bytes:
        return chunks
    n_windows = len(pcm) // window_bytes
    silence_windows = max(1, VAD_MIN_SILENCE_MS // VAD_WINDOW_MS)
    max_chunk_windows = max(1, VAD_MAX_CHUNK_S * 1000 // VAD_WINDOW_MS)
    min_chunk_windows = max(1, VAD_MIN_CHUNK_MS // VAD_WINDOW_MS)

    is_speech = [False] * n_windows
    for i in range(n_windows):
        win = pcm[i * window_bytes:(i + 1) * window_bytes]
        is_speech[i] = audioop.rms(win, sampwidth) >= VAD_RMS_THRESHOLD

    def emit(start_w: int, end_w: int):
        if end_w - start_w < min_chunk_windows:
            return
        data = pcm[start_w * window_bytes:end_w * window_bytes]
        buf = io.BytesIO()
        with wave.open(buf, "wb") as out:
            out.setnchannels(nchannels)
            out.setsampwidth(sampwidth)
            out.setframerate(framerate)
            out.writeframes(data)
        chunks.append((start_w * window_frames / framerate, buf.getvalue()))

    speech_start: int | None = None
    silent_run = 0
    for i in range(n_windows):
        if is_speech[i]:
            if speech_start is None:
                speech_start = i
            silent_run = 0
            if i - speech_start >= max_chunk_windows:
                emit(speech_start, i)
                speech_start = i
        elif speech_start is not None:
            silent_run += 1
            if silent_run >= silence_windows:
                emit(speech_start, i - silent_run + 1)
                speech_start = None
                silent_run = 0
    if speech_start is not None:
        emit(speech_start, n_windows)
    return chunks


# Collapse whisper-style hallucination loops like "no, no, no, no, no…".
# A short phrase (1–4 words) repeated ≥ 4 more times is replaced with one copy.
_REPEAT_RE = re.compile(
    r"(\b\w+(?:[\W_]+\w+){0,3})(?:[\W_]+\1){4,}",
    re.IGNORECASE,
)


def clean_repetition(text: str) -> str:
    cleaned = _REPEAT_RE.sub(r"\1", text)
    # Repeat once in case the substitution exposed another loop with different
    # punctuation at the seam.
    cleaned = _REPEAT_RE.sub(r"\1", cleaned)
    return cleaned.strip()


def fmt_duration(seconds: float) -> str:
    secs = max(0, int(seconds))
    h, rem = divmod(secs, 3600)
    m, s = divmod(rem, 60)
    return f"{h}h{m:02d}m{s:02d}s" if h else f"{m}m{s:02d}s"


# ---------------------------------------------------------------- callbacks
async def recording_finished(
    sink: WaveSink, text_channel: discord.TextChannel, guild_id: int
):
    """Called by py-cord after voice_client.stop_recording().

    Pulls per-user audio out of the sink, sends each through whisper, writes
    one transcript line per user (timestamped at flush time — py-cord WaveSink
    doesn't expose per-utterance offsets without more work), then replies in
    the invocation channel with the file attached.
    """
    session = sessions.pop(guild_id, None)
    if session is None:
        log.warning("recording_finished: no session for guild %s", guild_id)
        return

    file_path: Path = session["file_path"]
    language: str | None = session["language"]
    started_at: datetime = session["started_at"]
    voice_client = session["voice_client"]
    members_seen: set[int] = session.get("members_seen", set())
    # (user_id, offset_seconds, formatted_line) — sorted at the end so each
    # user's chunks stay chronological within their own section.
    entries: list[tuple[int, float, str]] = []

    # py-cord's WaveSink may key audio_data by Member or by int depending on
    # version — normalize to (int_id, audio) so set ops and sorted() work.
    def _key_to_id(k) -> int:
        return k.id if hasattr(k, "id") else int(k)

    captured = [(_key_to_id(k), v) for k, v in sink.audio_data.items()]
    captured_ids = {uid for uid, _ in captured}
    log.info(
        "sink captured %d user(s): %s; members observed during session: %s",
        len(captured_ids), sorted(captured_ids), sorted(members_seen),
    )
    missing_audio = members_seen - captured_ids - {bot.user.id if bot.user else 0}
    if missing_audio:
        log.warning(
            "members present but produced no audio (muted, PTT-silent, or "
            "SSRC/DAVE decrypt failure): %s",
            sorted(missing_audio),
        )

    async with aiohttp.ClientSession() as http:
        for user_id, audio in captured:
            try:
                user = await bot.fetch_user(user_id)
                nick = user.display_name
            except Exception:
                nick = str(user_id)

            wav_bytes = audio.file.getvalue()
            log.info(
                "user %s (%s): %d bytes of raw PCM in sink", nick, user_id, len(wav_bytes),
            )
            if not wav_bytes:
                continue
            try:
                chunks = chunk_wav_vad(wav_bytes)
            except Exception:
                log.exception("VAD chunking failed for user %s", user_id)
                continue
            log.info(
                "user %s (%s): %d utterance chunk(s) after VAD",
                nick, user_id, len(chunks),
            )
            for chunk_offset, chunk_bytes in chunks:
                try:
                    text = await transcribe_wav(http, chunk_bytes, language)
                except Exception:
                    log.exception(
                        "whisper failed for user %s at +%.1fs", user_id, chunk_offset
                    )
                    continue
                text = clean_repetition(text)
                if not text:
                    log.info(
                        "user %s: empty transcript for chunk at +%.1fs",
                        nick, chunk_offset,
                    )
                    continue
                mm, ss = divmod(int(chunk_offset), 60)
                entries.append(
                    (user_id, chunk_offset, f"[+{mm:02d}:{ss:02d}] @{nick}: {text}")
                )

    entries.sort(key=lambda e: (e[0], e[1]))
    transcripts = [line for _, _, line in entries]
    if not transcripts:
        transcripts.append("(no speech detected)")

    file_path.write_text("\n".join(transcripts) + "\n", encoding="utf-8")

    duration = (datetime.now(timezone.utc) - started_at).total_seconds()
    try:
        await text_channel.send(
            content=(
                f"Recording stopped. Duration {fmt_duration(duration)}. "
                f"{len(entries)} line(s)."
            ),
            file=discord.File(str(file_path)),
        )
    except Exception:
        log.exception("failed to send transcript reply")


# ---------------------------------------------------------------- commands
@bot.command(name="transcribe_join")
async def transcribe_join(ctx: commands.Context, *, args: str = ""):
    """Join the caller's voice channel and start transcribing.

    Usage:
      !transcribe_join
      !transcribe_join lang=it
    """
    if ctx.guild is None:
        return
    if ctx.author.voice is None or ctx.author.voice.channel is None:
        await ctx.reply("Join a voice channel first, then run this command again.")
        return

    try:
        language = parse_lang(args)
    except ValueError as e:
        await ctx.reply(str(e))
        return

    if ctx.guild.id in sessions:
        await ctx.reply(
            "A transcription session is already active here. "
            "Run `!transcribe_leave` first."
        )
        return

    channel = ctx.author.voice.channel
    try:
        voice_client = await channel.connect()
    except Exception as e:
        log.exception("connect failed")
        await ctx.reply(f"Failed to join voice channel: {e}")
        return

    started_at = datetime.now(timezone.utc)
    file_name = (
        f"transcript-{ctx.guild.id}-{channel.id}-"
        f"{started_at.strftime('%Y%m%dT%H%M%S')}.txt"
    )
    file_path = TRANSCRIPTS_DIR / file_name

    members_seen = {m.id for m in channel.members if not m.bot}
    log.info(
        "transcribe_join: guild=%s channel=%s members at start: %s",
        ctx.guild.id, channel.id, sorted(members_seen),
    )

    sessions[ctx.guild.id] = {
        "voice_client": voice_client,
        "text_channel": ctx.channel,
        "file_path": file_path,
        "language": language,
        "started_at": started_at,
        "channel_id": channel.id,
        "members_seen": members_seen,
    }

    sink = WaveSink()
    voice_client.start_recording(sink, recording_finished, ctx.channel, ctx.guild.id)

    lang_label = language or "auto-detect"
    await ctx.reply(
        f"Recording started in <#{channel.id}>. Language: **{lang_label}**. "
        f"Run `!transcribe_leave` to stop."
    )


@bot.command(name="transcribe_leave")
async def transcribe_leave(ctx: commands.Context):
    """Stop transcribing, disconnect, and reply with the transcript file."""
    if ctx.guild is None:
        return
    session = sessions.get(ctx.guild.id)
    if session is None:
        await ctx.reply("No transcription session active in this server.")
        return
    voice_client = session["voice_client"]
    try:
        voice_client.stop_recording()  # triggers recording_finished
    except Exception:
        log.exception("stop_recording failed")
    try:
        await voice_client.disconnect(force=False)
    except Exception:
        log.exception("disconnect failed")
    # recording_finished sends the file; nothing to reply here directly.


@bot.command(name="transcribe_status")
async def transcribe_status(ctx: commands.Context):
    if ctx.guild is None:
        return
    session = sessions.get(ctx.guild.id)
    if session is None:
        await ctx.reply("No transcription session active in this server.")
        return
    duration = (datetime.now(timezone.utc) - session["started_at"]).total_seconds()
    lang = session["language"] or "auto-detect"
    channel_id = session["voice_client"].channel.id
    await ctx.reply(
        f"Active session in <#{channel_id}>. "
        f"Language: **{lang}**. Duration: {fmt_duration(duration)}."
    )


# ---------------------------------------------------------------- lifecycle
@bot.event
async def on_ready():
    log.info("voice bot ready as %s (id=%s)", bot.user, bot.user.id)


@bot.event
async def on_voice_state_update(
    member: discord.Member,
    before: discord.VoiceState,
    after: discord.VoiceState,
):
    """Track who is in the recorded channel so we can diagnose missing audio.

    If someone joins after start_recording, py-cord may or may not pick up
    their SSRC depending on the DAVE key exchange. Logging join/leave here
    plus the final sink.audio_data keys tells us who was present but produced
    no audio (muted, push-to-talk silent, or decrypt failure).
    """
    if member.bot:
        return
    session = sessions.get(member.guild.id)
    if session is None:
        return
    target_channel_id = session.get("channel_id")
    before_id = before.channel.id if before.channel else None
    after_id = after.channel.id if after.channel else None
    if after_id == target_channel_id and before_id != target_channel_id:
        session["members_seen"].add(member.id)
        log.info(
            "voice state: %s (%s) joined recorded channel %s",
            member.display_name, member.id, target_channel_id,
        )
    elif before_id == target_channel_id and after_id != target_channel_id:
        log.info(
            "voice state: %s (%s) left recorded channel %s",
            member.display_name, member.id, target_channel_id,
        )


@bot.event
async def on_command_error(ctx, error):
    if isinstance(error, commands.CommandNotFound):
        return
    log.exception("command error: %s", error)


if __name__ == "__main__":
    bot.run(DISCORD_TOKEN)
