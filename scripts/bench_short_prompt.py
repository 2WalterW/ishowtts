#!/usr/bin/env python3
import argparse
import base64
import json
import statistics
import sys
import time
import urllib.error
import urllib.request

DEFAULT_TEXT = "My name is Walter Hartwell White."
DEFAULT_VOICE = "walter-index"
DEFAULT_URL = "http://127.0.0.1:27121/api/tts"


def post_tts(url: str, voice: str, text: str) -> dict:
    payload = json.dumps({"text": text, "voice_id": voice}).encode("utf-8")
    req = urllib.request.Request(
        url,
        data=payload,
        method="POST",
        headers={"Content-Type": "application/json"},
    )

    started = time.perf_counter()
    with urllib.request.urlopen(req, timeout=120) as resp:
        body = resp.read()
        status = resp.status
    elapsed = time.perf_counter() - started

    if status != 200:
        raise RuntimeError(f"Request failed ({status}): {body.decode('utf-8', errors='ignore')}")

    doc = json.loads(body)
    audio_bytes = 0
    if isinstance(doc, dict) and isinstance(doc.get("audio_base64"), str):
        try:
            audio_bytes = len(base64.b64decode(doc["audio_base64"], validate=True))
        except (base64.binascii.Error, ValueError):
            audio_bytes = 0
    return {
        "elapsed": elapsed,
        "audio_bytes": audio_bytes,
        "response": doc,
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Benchmark short-prompt TTS latency")
    parser.add_argument("--url", default=DEFAULT_URL, help="TTS endpoint URL")
    parser.add_argument("--voice", default=DEFAULT_VOICE, help="Voice ID to test")
    parser.add_argument("--text", default=DEFAULT_TEXT, help="Prompt text")
    parser.add_argument("--runs", type=int, default=3, help="Number of requests to send")
    parser.add_argument("--warmup", type=int, default=1, help="Warmup requests (not timed)")
    args = parser.parse_args(argv)

    print(f"Benchmarking {args.voice!r} at {args.url} -> runs={args.runs}, warmup={args.warmup}")

    for i in range(args.warmup):
        try:
            post_tts(args.url, args.voice, args.text)
        except Exception as exc:
            print(f"Warmup {i+1} failed: {exc}", file=sys.stderr)
            return 1

    timings = []
    audio_sizes = []
    for idx in range(args.runs):
        try:
            result = post_tts(args.url, args.voice, args.text)
        except (urllib.error.URLError, RuntimeError) as exc:
            print(f"Run {idx+1} error: {exc}", file=sys.stderr)
            return 1

        elapsed_ms = result["elapsed"] * 1000.0
        timings.append(result["elapsed"])
        audio_sizes.append(result["audio_bytes"])
        preview = result["response"].get("text_preview", "") if isinstance(result["response"], dict) else ""
        print(f"Run {idx+1:02d}: {elapsed_ms:8.2f} ms, audio={result['audio_bytes']} bytes, preview={preview!r}")

    if not timings:
        print("No successful runs", file=sys.stderr)
        return 1

    avg = statistics.mean(timings)
    p50 = statistics.median(timings)
    p90 = statistics.quantiles(timings, n=10)[8] if len(timings) > 1 else timings[0]
    best = min(timings)

    print("\nSummary (seconds):")
    print(f"  best={best:.3f}  p50={p50:.3f}  p90={p90:.3f}  mean={avg:.3f}")
    print(f"  audio bytes mean={statistics.mean(audio_sizes):.1f}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
