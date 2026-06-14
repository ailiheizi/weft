#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import json
import time
import urllib.error
import urllib.request
import wave
from pathlib import Path

import numpy as np
import sherpa_onnx
import soundfile as sf


def load_labels(labels_path: Path) -> dict[int, str]:
    labels: dict[int, str] = {}
    with labels_path.open("r", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            labels[int(row["index"])] = row["display_name"]
    return labels


def read_wave_samples(wav_path: Path) -> tuple[np.ndarray, int]:
    data, sample_rate = sf.read(
        str(wav_path),
        always_2d=True,
        dtype="float32",
    )
    return np.ascontiguousarray(data[:, 0]), sample_rate


def wav_duration_seconds(wav_path: Path) -> float:
    with wave.open(str(wav_path), "rb") as wav_file:
        return wav_file.getnframes() / wav_file.getframerate()


def build_tagger(model_path: Path, labels_path: Path, topk: int) -> sherpa_onnx.AudioTagging:
    model_config = sherpa_onnx.AudioTaggingModelConfig(
        zipformer=sherpa_onnx.OfflineZipformerAudioTaggingModelConfig(
            model=str(model_path),
        ),
        num_threads=1,
        debug=False,
        provider="cpu",
    )
    config = sherpa_onnx.AudioTaggingConfig(model_config, str(labels_path), topk)
    if not config.validate():
        raise ValueError(f"Invalid config: {config}")
    return sherpa_onnx.AudioTagging(config)


def classify_audio(
    tagger: sherpa_onnx.AudioTagging,
    labels: dict[int, str],
    wav_path: Path,
) -> list[dict[str, float | int | str]]:
    samples, sample_rate = read_wave_samples(wav_path)
    stream = tagger.create_stream()
    stream.accept_waveform(sample_rate=sample_rate, waveform=samples)
    result = tagger.compute(stream)
    return [
        {
            "label": labels.get(int(item.index), f"UNKNOWN_{int(item.index)}"),
            "index": int(item.index),
            "score": float(item.prob),
        }
        for item in result
    ]


def build_sound_event_body(
    model_path: Path,
    wav_path: Path,
    top: list[dict[str, float | int | str]],
    session_id: str,
) -> dict[str, object]:
    if not top:
        raise RuntimeError("audio tagging returned no labels")

    primary = top[0]
    payload = {
        "primary_label": str(primary["label"]),
        "primary_score": float(primary["score"]),
        "top": top,
        "source": "zipformer-audio-tagging",
        "audio_file": wav_path.name,
        "audio_seconds": wav_duration_seconds(wav_path),
        "model": model_path.name,
    }
    return {
        "action": "ingest_external_event",
        "data": {
            "event_type": "sound_event_detected",
            "payload": payload,
            "session_id": session_id,
        },
    }


def post_webhook(webhook_url: str, body: dict[str, object]) -> dict[str, object]:
    request = urllib.request.Request(
        webhook_url,
        data=json.dumps(body, ensure_ascii=False).encode("utf-8"),
        headers={"Content-Type": "application/json; charset=utf-8"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            raw = response.read().decode("utf-8")
    except urllib.error.HTTPError as exc:
        raise RuntimeError(f"webhook failed: HTTP {exc.code} {exc.read().decode('utf-8', errors='replace')}") from exc
    except urllib.error.URLError as exc:
        raise RuntimeError(f"webhook failed: {exc}") from exc
    parsed = json.loads(raw) if raw.strip() else {}
    if isinstance(parsed, dict) and parsed.get("status") == "error":
        raise RuntimeError(str(parsed.get("error") or "webhook returned error"))
    return parsed if isinstance(parsed, dict) else {}


def emit_wav_event(
    tagger: sherpa_onnx.AudioTagging,
    labels: dict[int, str],
    model_path: Path,
    wav_path: Path,
    webhook_url: str,
    session_id: str,
) -> dict[str, object]:
    top = classify_audio(tagger, labels, wav_path)
    body = build_sound_event_body(model_path, wav_path, top, session_id)
    response = post_webhook(webhook_url, body)
    data = body["data"]
    if not isinstance(data, dict):
        raise RuntimeError("invalid sound event body")
    payload = data["payload"]
    if not isinstance(payload, dict):
        raise RuntimeError("invalid sound event payload")
    return {
        "event_type": "sound_event_detected",
        "payload": payload,
        "webhook_status": response.get("status", "ok") if isinstance(response, dict) else "ok",
    }


def iter_wav_segments(watch_dir: Path) -> list[Path]:
    return sorted(
        path
        for path in watch_dir.glob("*.wav")
        if path.is_file()
    )


def run_watch_dir(
    tagger: sherpa_onnx.AudioTagging,
    labels: dict[int, str],
    model_path: Path,
    watch_dir: Path,
    webhook_url: str,
    session_id: str,
    poll_seconds: float,
    max_files: int,
) -> None:
    processed: set[Path] = set()
    emitted = 0
    while True:
        for wav_path in iter_wav_segments(watch_dir):
            resolved = wav_path.resolve()
            if resolved in processed:
                continue
            result = emit_wav_event(
                tagger=tagger,
                labels=labels,
                model_path=model_path,
                wav_path=wav_path,
                webhook_url=webhook_url,
                session_id=session_id,
            )
            processed.add(resolved)
            emitted += 1
            print(json.dumps(result, ensure_ascii=False), flush=True)
            if max_files > 0 and emitted >= max_files:
                return
        time.sleep(max(0.01, poll_seconds))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", required=True)
    parser.add_argument("--labels", required=True)
    input_group = parser.add_mutually_exclusive_group(required=True)
    input_group.add_argument("--wav")
    input_group.add_argument("--watch-dir")
    parser.add_argument("--webhook-url", required=True)
    parser.add_argument("--session-id", default="")
    parser.add_argument("--topk", type=int, default=5)
    parser.add_argument("--poll-seconds", type=float, default=1.0)
    parser.add_argument("--max-files", type=int, default=0)
    args = parser.parse_args()

    model_path = Path(args.model)
    labels_path = Path(args.labels)
    session_id = str(args.session_id or "").strip()

    labels = load_labels(labels_path)
    tagger = build_tagger(model_path, labels_path, args.topk)
    if args.watch_dir:
        run_watch_dir(
            tagger=tagger,
            labels=labels,
            model_path=model_path,
            watch_dir=Path(args.watch_dir),
            webhook_url=str(args.webhook_url),
            session_id=session_id,
            poll_seconds=float(args.poll_seconds),
            max_files=int(args.max_files),
        )
        return

    result = emit_wav_event(
        tagger=tagger,
        labels=labels,
        model_path=model_path,
        wav_path=Path(args.wav),
        webhook_url=str(args.webhook_url),
        session_id=session_id,
    )
    print(json.dumps(result, ensure_ascii=False))


if __name__ == "__main__":
    main()
