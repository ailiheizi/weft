#!/usr/bin/env python3
from __future__ import annotations

import argparse
from fractions import Fraction
import signal
import sys
import time
import wave
from pathlib import Path
from threading import Lock

import numpy as np

DEFAULT_SAMPLE_RATE = 16_000
DEFAULT_SEGMENT_SECONDS = 2.0
DEFAULT_PREFIX = "audio-segment"

_running = True


def _stop(*_: object) -> None:
    global _running
    _running = False


signal.signal(signal.SIGTERM, _stop)
signal.signal(signal.SIGINT, _stop)


def write_wav(path: Path, samples: np.ndarray, sample_rate: int) -> None:
    normalized = np.asarray(samples, dtype=np.int16)
    with wave.open(str(path), "wb") as wav_file:
        wav_file.setnchannels(1)
        wav_file.setsampwidth(2)
        wav_file.setframerate(sample_rate)
        wav_file.writeframes(normalized.tobytes())


def write_pcm_segments(
    samples: np.ndarray,
    output_dir: Path,
    sample_rate: int,
    segment_seconds: float,
    prefix: str = DEFAULT_PREFIX,
    start_index: int = 1,
) -> list[Path]:
    output_dir.mkdir(parents=True, exist_ok=True)
    mono = np.asarray(samples, dtype=np.int16).reshape(-1)
    frames_per_segment = max(1, int(round(sample_rate * segment_seconds)))
    written: list[Path] = []
    index = start_index
    for offset in range(0, len(mono), frames_per_segment):
        chunk = mono[offset: offset + frames_per_segment]
        if len(chunk) < frames_per_segment:
            break
        path = output_dir / f"{prefix}-{index:06d}.wav"
        write_wav(path, chunk, sample_rate)
        written.append(path)
        index += 1
    return written


def record_sounddevice_segments(
    output_dir: Path,
    sample_rate: int,
    segment_seconds: float,
    prefix: str = DEFAULT_PREFIX,
    max_segments: int = 0,
) -> list[Path]:
    try:
        import sounddevice as sd
    except ImportError as exc:
        raise RuntimeError("sounddevice is required for live audio capture") from exc

    global _running
    _running = True
    output_dir.mkdir(parents=True, exist_ok=True)

    frames_per_segment = max(1, int(round(sample_rate * segment_seconds)))
    pending = np.empty(0, dtype=np.int16)
    written: list[Path] = []
    next_index = 1
    lock = Lock()

    def callback(indata, frames, time_info, status) -> None:  # type: ignore[no-untyped-def]
        nonlocal pending
        if not _running:
            raise sd.CallbackStop()
        if status:
            print(f"[audio-segment-recorder] callback status: {status}", flush=True)
        mono = np.asarray(indata[:, 0], dtype=np.int16)
        with lock:
            if pending.size == 0:
                pending = mono.copy()
            else:
                pending = np.concatenate([pending, mono])

    def pop_segment() -> np.ndarray | None:
        nonlocal pending
        with lock:
            if pending.size < frames_per_segment:
                return None
            chunk = pending[:frames_per_segment].copy()
            pending = pending[frames_per_segment:]
            return chunk

    def write_available_segments() -> None:
        nonlocal next_index
        while True:
            chunk = pop_segment()
            if chunk is None:
                return
            path = output_dir / f"{prefix}-{next_index:06d}.wav"
            write_wav(path, chunk, sample_rate)
            written.append(path)
            print(f"[audio-segment-recorder] WROTE {path}", flush=True)
            next_index += 1

    def try_stream(device: int | None, channels: int, extra, label: str):
        candidates = [min(channels, 2), 1] if channels > 1 else [1]
        for channel_count in candidates:
            try:
                kwargs = {
                    "samplerate": sample_rate,
                    "channels": channel_count,
                    "dtype": "int16",
                    "device": device,
                    "latency": "high",
                    "callback": callback,
                }
                if extra is not None:
                    kwargs["extra_settings"] = extra
                stream = sd.InputStream(**kwargs)
                return stream, f"{label} (ch={channel_count})"
            except Exception as exc:
                print(f"[audio-segment-recorder] sounddevice candidate failed {label}: {exc}", flush=True)
        return None, ""

    candidates: list[tuple[int | None, int, object | None, str]] = []
    try:
        wasapi_shared = sd.WasapiSettings(exclusive=False)
        keywords = ["stereo mix", "loopback", "立体声混音"]
        for index, device in enumerate(sd.query_devices()):
            if int(device["max_input_channels"]) < 1:
                continue
            name = str(device.get("name") or "")
            lowered = name.lower()
            if any(keyword in lowered for keyword in keywords):
                candidates.append((index, int(device["max_input_channels"]), wasapi_shared, f"StereoMix:{name}"))
    except Exception:
        pass

    try:
        candidates.append((None, 2, sd.WasapiSettings(exclusive=False), "DefaultInput:WASAPI"))
    except Exception:
        pass
    candidates.append((None, 2, None, "DefaultInput:MME"))

    stream = None
    stream_label = ""
    for device, channels, extra, label in candidates:
        stream, stream_label = try_stream(device, channels, extra, label)
        if stream is not None:
            break
    if stream is None:
        raise RuntimeError("sounddevice could not open any usable input device")

    with stream:
        print(
            f"[audio-segment-recorder] START output_dir={output_dir} sample_rate={sample_rate} segment_seconds={segment_seconds} source={stream_label}",
            flush=True,
        )
        while _running:
            write_available_segments()
            if max_segments > 0 and len(written) >= max_segments:
                break
            time.sleep(0.1)

    return written


def record_pyaudiowpatch_segments(
    output_dir: Path,
    sample_rate: int,
    segment_seconds: float,
    prefix: str = DEFAULT_PREFIX,
    max_segments: int = 0,
) -> list[Path]:
    try:
        import pyaudiowpatch as pyaudio
    except ImportError as exc:
        raise RuntimeError("pyaudiowpatch is required for WASAPI loopback capture") from exc

    global _running
    _running = True
    output_dir.mkdir(parents=True, exist_ok=True)

    pa = pyaudio.PyAudio()
    try:
        wasapi_index = None
        for index in range(pa.get_host_api_count()):
            info = pa.get_host_api_info_by_index(index)
            if info.get("type") == pyaudio.paWASAPI:
                wasapi_index = index
                break
        if wasapi_index is None:
            raise RuntimeError("WASAPI host API not found")

        wasapi_info = pa.get_host_api_info_by_index(wasapi_index)
        default_output_index = int(wasapi_info.get("defaultOutputDevice", -1))
        if default_output_index < 0:
            raise RuntimeError("default output device not found")

        default_output = pa.get_device_info_by_index(default_output_index)
        default_output_name = str(default_output.get("name") or "")
        loopback_device = None

        for index in range(pa.get_device_count()):
            device = pa.get_device_info_by_index(index)
            if not device.get("isLoopbackDevice", False):
                continue
            name = str(device.get("name") or "")
            if default_output_name and default_output_name[:10] in name:
                loopback_device = dict(device)
                loopback_device["_index"] = index
                break

        if loopback_device is None:
            for index in range(pa.get_device_count()):
                device = pa.get_device_info_by_index(index)
                if device.get("isLoopbackDevice", False):
                    loopback_device = dict(device)
                    loopback_device["_index"] = index
                    break

        if loopback_device is None:
            raise RuntimeError("no loopback device found")

        device_index = int(loopback_device["_index"])
        device_name = str(loopback_device.get("name") or "")
        device_channels = max(1, min(int(loopback_device.get("maxInputChannels", 2)), 2))
        device_sample_rate = int(loopback_device.get("defaultSampleRate", sample_rate))

        pcm_frames: list[bytes] = []
        pending = np.empty(0, dtype=np.int16)
        written_count = 0
        frames_per_segment = max(1, int(round(sample_rate * segment_seconds)))
        lock = Lock()

        def pa_callback(in_data, frame_count, time_info, status):  # type: ignore[no-untyped-def]
            if _running:
                with lock:
                    pcm_frames.append(in_data)
            return (None, pyaudio.paContinue)

        stream = pa.open(
            format=pyaudio.paInt16,
            channels=device_channels,
            rate=device_sample_rate,
            input=True,
            input_device_index=device_index,
            frames_per_buffer=1024,
            stream_callback=pa_callback,
        )
        stream.start_stream()
        print(
            f"[audio-segment-recorder] START output_dir={output_dir} sample_rate={sample_rate} segment_seconds={segment_seconds} source=Loopback:{device_name}",
            flush=True,
        )
        try:
            while _running:
                with lock:
                    frame_count = len(pcm_frames)
                if frame_count > 0:
                    with lock:
                        raw = b"".join(pcm_frames)
                        pcm_frames.clear()
                    pcm = np.frombuffer(raw, dtype=np.int16)
                    if device_channels > 1:
                        pcm = pcm.reshape(-1, device_channels)[:, 0]
                    if device_sample_rate != sample_rate:
                        ratio = Fraction(sample_rate, device_sample_rate).limit_denominator(100)
                        original_length = len(pcm)
                        new_length = int(original_length * ratio.numerator / ratio.denominator)
                        pcm = np.interp(
                            np.linspace(0, original_length - 1, new_length),
                            np.arange(original_length),
                            pcm.astype(np.float32),
                        ).astype(np.int16)
                    pending = np.concatenate([pending, pcm]) if pending.size else pcm
                    while pending.size >= frames_per_segment:
                        chunk = pending[:frames_per_segment]
                        pending = pending[frames_per_segment:]
                        written_count += 1
                        path = output_dir / f"{prefix}-{written_count:06d}.wav"
                        write_wav(path, chunk, sample_rate)
                        written = [path]
                        for path in written:
                            print(f"[audio-segment-recorder] WROTE {path}", flush=True)
                        if max_segments > 0 and written_count >= max_segments:
                            break
                    if max_segments > 0 and written_count >= max_segments:
                        break
                time.sleep(0.1)
        finally:
            stream.stop_stream()
            stream.close()
    finally:
        pa.terminate()

    return sorted(output_dir.glob(f"{prefix}-*.wav"))


def record_live_segments(
    output_dir: Path,
    sample_rate: int,
    segment_seconds: float,
    prefix: str = DEFAULT_PREFIX,
    max_segments: int = 0,
) -> list[Path]:
    try:
        result = record_pyaudiowpatch_segments(
            output_dir=output_dir,
            sample_rate=sample_rate,
            segment_seconds=segment_seconds,
            prefix=prefix,
            max_segments=max_segments,
        )
        if result:
            return result
    except Exception as exc:
        print(f"[audio-segment-recorder] loopback unavailable: {exc}", flush=True)

    return record_sounddevice_segments(
        output_dir=output_dir,
        sample_rate=sample_rate,
        segment_seconds=segment_seconds,
        prefix=prefix,
        max_segments=max_segments,
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-dir", required=True)
    parser.add_argument("--sample-rate", type=int, default=DEFAULT_SAMPLE_RATE)
    parser.add_argument("--segment-seconds", type=float, default=DEFAULT_SEGMENT_SECONDS)
    parser.add_argument("--prefix", default=DEFAULT_PREFIX)
    parser.add_argument("--max-segments", type=int, default=0)
    args = parser.parse_args()

    result = record_live_segments(
        output_dir=Path(args.output_dir),
        sample_rate=int(args.sample_rate),
        segment_seconds=float(args.segment_seconds),
        prefix=str(args.prefix),
        max_segments=int(args.max_segments),
    )
    print(f"[audio-segment-recorder] DONE segments={len(result)}", flush=True)


if __name__ == "__main__":
    try:
        main()
    except RuntimeError as exc:
        print(f"[audio-segment-recorder] ERROR {exc}", flush=True)
        sys.exit(1)
