#!/usr/bin/env python3
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


def spawn_process(command: list[str]) -> subprocess.Popen[str]:
    return subprocess.Popen(
        command,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
    )


def read_process_output(process: subprocess.Popen[str]) -> tuple[str, str]:
    stdout, stderr = process.communicate()
    return stdout or "", stderr or ""


def build_recorder_command(args: argparse.Namespace) -> list[str]:
    command = [
        str(args.recorder_python),
        str(args.recorder_script),
        "--output-dir",
        str(args.segments_dir),
        "--sample-rate",
        str(args.sample_rate),
        "--segment-seconds",
        str(args.segment_seconds),
        "--prefix",
        str(args.prefix),
        "--max-segments",
        str(args.max_segments),
    ]
    for item in args.recorder_extra_arg:
        command.append(item)
    return command


def build_producer_command(args: argparse.Namespace) -> list[str]:
    command = [
        str(args.producer_python),
        str(args.producer_script),
        "--model",
        str(args.model),
        "--labels",
        str(args.labels),
        "--watch-dir",
        str(args.segments_dir),
        "--poll-seconds",
        str(args.poll_seconds),
        "--max-files",
        str(args.max_segments),
        "--topk",
        str(args.topk),
        "--webhook-url",
        str(args.webhook_url),
        "--session-id",
        str(args.session_id),
    ]
    for item in args.producer_extra_arg:
        command.append(item)
    return command


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--recorder-python", default=sys.executable)
    parser.add_argument("--recorder-script", required=True)
    parser.add_argument("--producer-python", default=sys.executable)
    parser.add_argument("--producer-script", required=True)
    parser.add_argument("--segments-dir", required=True)
    parser.add_argument("--model", required=True)
    parser.add_argument("--labels", required=True)
    parser.add_argument("--webhook-url", required=True)
    parser.add_argument("--session-id", required=True)
    parser.add_argument("--sample-rate", type=int, default=16_000)
    parser.add_argument("--segment-seconds", type=float, default=2.0)
    parser.add_argument("--max-segments", type=int, default=0)
    parser.add_argument("--topk", type=int, default=5)
    parser.add_argument("--poll-seconds", type=float, default=0.2)
    parser.add_argument("--prefix", default="audio-segment")
    parser.add_argument("--recorder-extra-arg", action="append", default=[])
    parser.add_argument("--producer-extra-arg", action="append", default=[])
    args = parser.parse_args()

    Path(args.segments_dir).mkdir(parents=True, exist_ok=True)
    producer = spawn_process(build_producer_command(args))
    recorder = spawn_process(build_recorder_command(args))

    recorder_stdout, recorder_stderr = read_process_output(recorder)
    if recorder.returncode != 0:
        producer.terminate()
        producer_stdout, producer_stderr = read_process_output(producer)
        raise RuntimeError(
            f"recorder failed with code {recorder.returncode}\nstdout:\n{recorder_stdout}\nstderr:\n{recorder_stderr}\n"
            f"producer stdout:\n{producer_stdout}\nproducer stderr:\n{producer_stderr}"
        )

    producer_stdout, producer_stderr = read_process_output(producer)
    if producer.returncode != 0:
        raise RuntimeError(
            f"producer failed with code {producer.returncode}\nstdout:\n{producer_stdout}\nstderr:\n{producer_stderr}"
        )

    if recorder_stdout.strip():
        print(recorder_stdout.strip())
    if producer_stdout.strip():
        print(producer_stdout.strip())


if __name__ == "__main__":
    try:
        main()
    except RuntimeError as exc:
        print(f"[audio-event-live-runner] ERROR {exc}", flush=True)
        sys.exit(1)
