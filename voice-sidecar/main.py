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
import io
import logging
import os
import sys
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
HTTP_TIMEOUT = aiohttp.ClientTimeout(total=120)

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
    transcripts: list[str] = []

    async with aiohttp.ClientSession() as http:
        for user_id, audio in sink.audio_data.items():
            try:
                user = await bot.fetch_user(user_id)
                nick = user.display_name
            except Exception:
                nick = str(user_id)

            wav_bytes = audio.file.getvalue()
            if not wav_bytes:
                continue
            try:
                text = await transcribe_wav(http, wav_bytes, language)
            except Exception as e:
                log.exception("whisper failed for user %s: %s", user_id, e)
                continue
            if not text:
                continue
            ts = datetime.now(timezone.utc).strftime("%H:%M:%S")
            transcripts.append(f"[{ts}] @{nick}: {text}")

    if not transcripts:
        transcripts.append("(no speech detected)")

    file_path.write_text("\n".join(transcripts) + "\n", encoding="utf-8")

    duration = (datetime.now(timezone.utc) - started_at).total_seconds()
    try:
        await text_channel.send(
            content=(
                f"Recording stopped. Duration {fmt_duration(duration)}. "
                f"{len([t for t in transcripts if not t.startswith('(no')])} line(s)."
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

    sessions[ctx.guild.id] = {
        "voice_client": voice_client,
        "text_channel": ctx.channel,
        "file_path": file_path,
        "language": language,
        "started_at": started_at,
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
async def on_command_error(ctx, error):
    if isinstance(error, commands.CommandNotFound):
        return
    log.exception("command error: %s", error)


if __name__ == "__main__":
    bot.run(DISCORD_TOKEN)
