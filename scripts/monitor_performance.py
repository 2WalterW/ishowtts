#!/usr/bin/env python3
"""
Performance Monitoring Script for iShowTTS
Tracks RTF, latency, and quality metrics over time
Detects performance regressions automatically
"""

import os
import sys
import json
import time
from datetime import datetime
from pathlib import Path

# Add F5-TTS to path
script_dir = Path(__file__).parent
repo_root = script_dir.parent
sys.path.insert(0, str(repo_root / "third_party" / "F5-TTS" / "src"))

from f5_tts.api import F5TTS

# Configuration
REFERENCE_AUDIO = repo_root / "data" / "voices" / "walter_reference.wav"
REFERENCE_TEXT = (
    "No, you clearly don't know who you're talking to, so let me clue you in. "
    "I am not in danger, Skyler. I am the danger. A guy opens his door and gets shot, "
    "and you think that of me? No, I am."
)

TEST_SENTENCES = [
    "Hello world!",
    "The quick brown fox jumps over the lazy dog.",
    "Welcome to the livestream, thanks for joining us today!",
    "In the field of artificial intelligence and machine learning, we are witnessing rapid advancements.",
]

LOG_DIR = repo_root / "logs" / "performance"
LOG_FILE = LOG_DIR / "performance_log.jsonl"

# Performance thresholds (for regression detection)
THRESHOLDS = {
    "mean_rtf": 0.25,  # Alert if mean RTF > 0.25
    "max_rtf": 0.30,   # Alert if any RTF > 0.30
    "variance": 0.10,  # Alert if variance > 10%
}


def ensure_log_dir():
    """Create log directory if it doesn't exist"""
    LOG_DIR.mkdir(parents=True, exist_ok=True)


def load_historical_data():
    """Load historical performance data"""
    if not LOG_FILE.exists():
        return []

    data = []
    with open(LOG_FILE, 'r') as f:
        for line in f:
            if line.strip():
                data.append(json.loads(line))
    return data


def save_performance_data(data):
    """Append performance data to log file"""
    ensure_log_dir()
    with open(LOG_FILE, 'a') as f:
        f.write(json.dumps(data) + '\n')


def run_performance_test(nfe_steps=None):
    """Run performance test and return metrics"""
    print("Initializing F5-TTS...")
    f5tts = F5TTS(
        model_type="F5-TTS",
        ckpt_file="",
        vocab_file="",
        device="cuda"
    )

    # Get NFE from config if not specified
    if nfe_steps is None:
        import toml
        config_file = repo_root / "config" / "ishowtts.toml"
        config = toml.load(config_file)
        nfe_steps = config.get("f5", {}).get("default_nfe_step", 8)

    print(f"Running tests with NFE={nfe_steps}...")

    results = []
    for i, text in enumerate(TEST_SENTENCES, 1):
        print(f"  Test {i}/{len(TEST_SENTENCES)}: {text[:50]}...")

        start_time = time.time()
        wav, sample_rate, infer_stats = f5tts.infer(
            ref_file=str(REFERENCE_AUDIO),
            ref_text=REFERENCE_TEXT,
            gen_text=text,
            nfe_step=nfe_steps,
            show_info=print
        )
        end_time = time.time()

        synthesis_time = end_time - start_time
        audio_duration = len(wav) / sample_rate
        rtf = synthesis_time / audio_duration if audio_duration > 0 else float('inf')

        results.append({
            "text": text,
            "text_length": len(text),
            "synthesis_time": synthesis_time,
            "audio_duration": audio_duration,
            "rtf": rtf,
            "speedup": 1 / rtf if rtf > 0 else 0,
        })

        print(f"    RTF: {rtf:.3f} | Time: {synthesis_time:.2f}s | Audio: {audio_duration:.2f}s")

    return results


def analyze_results(results):
    """Analyze test results and compute statistics"""
    rtfs = [r["rtf"] for r in results]
    synthesis_times = [r["synthesis_time"] for r in results]

    mean_rtf = sum(rtfs) / len(rtfs)
    min_rtf = min(rtfs)
    max_rtf = max(rtfs)

    mean_speedup = 1 / mean_rtf if mean_rtf > 0 else 0

    # Calculate variance
    variance = max(
        abs(rtf - mean_rtf) / mean_rtf for rtf in rtfs
    ) if mean_rtf > 0 else 0

    return {
        "mean_rtf": mean_rtf,
        "min_rtf": min_rtf,
        "max_rtf": max_rtf,
        "mean_speedup": mean_speedup,
        "variance": variance,
        "mean_synthesis_time": sum(synthesis_times) / len(synthesis_times),
    }


def check_regressions(stats, historical_data):
    """Check for performance regressions"""
    regressions = []

    # Check against thresholds
    if stats["mean_rtf"] > THRESHOLDS["mean_rtf"]:
        regressions.append(
            f"⚠️  Mean RTF {stats['mean_rtf']:.3f} exceeds threshold {THRESHOLDS['mean_rtf']}"
        )

    if stats["max_rtf"] > THRESHOLDS["max_rtf"]:
        regressions.append(
            f"⚠️  Max RTF {stats['max_rtf']:.3f} exceeds threshold {THRESHOLDS['max_rtf']}"
        )

    if stats["variance"] > THRESHOLDS["variance"]:
        regressions.append(
            f"⚠️  Variance {stats['variance']*100:.1f}% exceeds threshold {THRESHOLDS['variance']*100:.1f}%"
        )

    # Check against historical baseline (last 10 runs)
    if len(historical_data) >= 10:
        recent = historical_data[-10:]
        baseline_rtf = sum(d["stats"]["mean_rtf"] for d in recent) / len(recent)

        # Alert if 10% slower than recent baseline
        if stats["mean_rtf"] > baseline_rtf * 1.10:
            regressions.append(
                f"⚠️  Performance regression: {stats['mean_rtf']:.3f} vs baseline {baseline_rtf:.3f} "
                f"({(stats['mean_rtf']/baseline_rtf - 1)*100:.1f}% slower)"
            )

    return regressions


def print_summary(stats, regressions):
    """Print performance summary"""
    print("\n" + "="*70)
    print("PERFORMANCE SUMMARY")
    print("="*70)
    print(f"Mean RTF:        {stats['mean_rtf']:.3f} ({'✅' if stats['mean_rtf'] < 0.25 else '⚠️ '})")
    print(f"Best RTF:        {stats['min_rtf']:.3f}")
    print(f"Worst RTF:       {stats['max_rtf']:.3f}")
    print(f"Mean Speedup:    {stats['mean_speedup']:.2f}x")
    print(f"Variance:        ±{stats['variance']*100:.1f}%")
    print(f"Mean Synth Time: {stats['mean_synthesis_time']:.2f}s")
    print("="*70)

    if regressions:
        print("\n🚨 REGRESSIONS DETECTED:")
        for reg in regressions:
            print(f"  {reg}")
        print()
    else:
        print("\n✅ No regressions detected. Performance is healthy!")
    print()


def print_historical_comparison(historical_data):
    """Print comparison with historical data"""
    if len(historical_data) < 2:
        return

    print("\nHISTORICAL COMPARISON (Last 5 runs):")
    print("-" * 70)
    print(f"{'Date':<20} {'Mean RTF':<12} {'Speedup':<10} {'Variance':<10}")
    print("-" * 70)

    for entry in historical_data[-5:]:
        date = entry['timestamp'][:19]  # YYYY-MM-DD HH:MM:SS
        stats = entry['stats']
        print(
            f"{date:<20} {stats['mean_rtf']:<12.3f} "
            f"{stats['mean_speedup']:<10.2f}x ±{stats['variance']*100:<9.1f}%"
        )
    print()


def main():
    """Main entry point"""
    import argparse

    parser = argparse.ArgumentParser(description="Monitor iShowTTS performance")
    parser.add_argument("--nfe", type=int, help="NFE steps to test (default: from config)")
    parser.add_argument("--no-save", action="store_true", help="Don't save results to log")
    parser.add_argument("--show-history", action="store_true", help="Show historical data only")

    args = parser.parse_args()

    # Load historical data
    historical_data = load_historical_data()

    if args.show_history:
        if not historical_data:
            print("No historical data available.")
            return
        print_historical_comparison(historical_data)
        return

    # Run performance test
    print(f"iShowTTS Performance Monitor")
    print(f"Timestamp: {datetime.now().isoformat()}")
    print("-" * 70)

    results = run_performance_test(args.nfe)
    stats = analyze_results(results)

    # Check for regressions
    regressions = check_regressions(stats, historical_data)

    # Print summary
    print_summary(stats, regressions)

    # Show historical comparison
    print_historical_comparison(historical_data)

    # Save results
    if not args.no_save:
        entry = {
            "timestamp": datetime.now().isoformat(),
            "nfe": args.nfe,
            "stats": stats,
            "results": results,
            "regressions": regressions,
        }
        save_performance_data(entry)
        print(f"✅ Results saved to {LOG_FILE}")

    # Exit with error code if regressions detected
    if regressions:
        sys.exit(1)

    sys.exit(0)


if __name__ == "__main__":
    main()