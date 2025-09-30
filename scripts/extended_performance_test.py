#!/usr/bin/env python3
"""
Extended Performance Test for iShowTTS
Tests with 20+ runs to measure variance and stability
"""

import sys
import time
import numpy as np
import statistics
sys.path.insert(0, '../third_party/F5-TTS/src')

try:
    import torch
    import torchaudio
    from f5_tts.api import F5TTS
except ImportError as e:
    print(f"Error importing dependencies: {e}")
    print("Make sure you have activated the ishowtts environment")
    sys.exit(1)

# Configuration
REF_AUDIO = "data/voices/walter_reference.wav"
REF_TEXT = "No, you clearly don't know who you're talking to, so let me clue you in."
GEN_TEXT = "Hello world, this is a quick test of the optimized TTS system with additional text to match the longer audio duration used in the quick performance test for more accurate RTF measurements."
NUM_RUNS = 20
WARMUP_RUNS = 3
NFE_STEP = 7  # Current configuration

def print_separator():
    print("=" * 70)

def main():
    print_separator()
    print("Extended Performance Test (20+ runs)")
    print_separator()
    print()

    # System info
    print(f"PyTorch: {torch.__version__}")
    print(f"CUDA available: {torch.cuda.is_available()}")
    if torch.cuda.is_available():
        print(f"CUDA device: {torch.cuda.get_device_name(0)}")
        print(f"CUDA version: {torch.version.cuda}")
    print()

    # Initialize model
    print("[Initializing Model]")
    init_start = time.time()
    f5tts = F5TTS(
        model="F5TTS_v1_Base",
        device="cuda" if torch.cuda.is_available() else "cpu",
        hf_cache_dir="data/cache/huggingface"
    )
    init_time = time.time() - init_start
    print(f"Initialization time: {init_time:.2f}s")
    print()

    # Warmup runs
    print(f"[Warmup - {WARMUP_RUNS} runs]")
    for i in range(WARMUP_RUNS):
        start = time.time()
        wav, sr, _ = f5tts.infer(
            ref_file=REF_AUDIO,
            ref_text=REF_TEXT,
            gen_text=GEN_TEXT,
            show_info=lambda x: None,  # Suppress output
            nfe_step=NFE_STEP,
        )
        elapsed = time.time() - start
        audio_duration = len(wav) / sr
        print(f"Warmup {i+1}: {elapsed:.3f}s | Audio: {audio_duration:.3f}s")
    print()

    # Performance test
    print(f"[Performance Test - {NUM_RUNS} runs]")
    times = []
    audio_durations = []
    rtfs = []
    speedups = []

    for i in range(NUM_RUNS):
        start = time.time()
        wav, sr, _ = f5tts.infer(
            ref_file=REF_AUDIO,
            ref_text=REF_TEXT,
            gen_text=GEN_TEXT,
            show_info=lambda x: None,
            nfe_step=NFE_STEP,
        )
        elapsed = time.time() - start
        audio_duration = len(wav) / sr
        rtf = elapsed / audio_duration
        speedup = audio_duration / elapsed

        times.append(elapsed)
        audio_durations.append(audio_duration)
        rtfs.append(rtf)
        speedups.append(speedup)

        print(f"Run {i+1:2d}: {elapsed:.3f}s | Audio: {audio_duration:.3f}s | RTF: {rtf:.3f} | Speedup: {speedup:.2f}x")

    print()
    print_separator()
    print("STATISTICS")
    print_separator()

    # Calculate statistics
    mean_time = statistics.mean(times)
    median_time = statistics.median(times)
    min_time = min(times)
    max_time = max(times)
    std_time = statistics.stdev(times) if len(times) > 1 else 0
    cv_time = (std_time / mean_time * 100) if mean_time > 0 else 0

    mean_rtf = statistics.mean(rtfs)
    median_rtf = statistics.median(rtfs)
    min_rtf = min(rtfs)
    max_rtf = max(rtfs)
    std_rtf = statistics.stdev(rtfs) if len(rtfs) > 1 else 0
    cv_rtf = (std_rtf / mean_rtf * 100) if mean_rtf > 0 else 0

    mean_speedup = statistics.mean(speedups)
    max_speedup = max(speedups)
    min_speedup = min(speedups)

    mean_audio_duration = statistics.mean(audio_durations)

    print(f"Audio Duration: {mean_audio_duration:.3f}s (mean)")
    print()
    print(f"Synthesis Time:")
    print(f"  Mean:   {mean_time:.3f}s")
    print(f"  Median: {median_time:.3f}s")
    print(f"  Min:    {min_time:.3f}s")
    print(f"  Max:    {max_time:.3f}s")
    print(f"  StdDev: {std_time:.3f}s")
    print(f"  CV:     {cv_time:.1f}%")
    print()
    print(f"Real-Time Factor (RTF):")
    print(f"  Mean:   {mean_rtf:.3f}")
    print(f"  Median: {median_rtf:.3f}")
    print(f"  Min:    {min_rtf:.3f} (best)")
    print(f"  Max:    {max_rtf:.3f} (worst)")
    print(f"  StdDev: {std_rtf:.3f}")
    print(f"  CV:     {cv_rtf:.1f}%")
    print()
    print(f"Speedup:")
    print(f"  Mean:   {mean_speedup:.2f}x")
    print(f"  Max:    {max_speedup:.2f}x (best)")
    print(f"  Min:    {min_speedup:.2f}x (worst)")
    print()

    # Performance evaluation
    target_rtf = 0.3
    phase3_target = 0.2

    print_separator()
    print("EVALUATION")
    print_separator()

    if mean_rtf < target_rtf:
        print(f"✅ Phase 1 Target (RTF < {target_rtf}): ACHIEVED")
    else:
        print(f"❌ Phase 1 Target (RTF < {target_rtf}): NOT MET")

    if mean_rtf < phase3_target:
        print(f"✅ Phase 3 Target (RTF < {phase3_target}): ACHIEVED")
    else:
        print(f"⚠️  Phase 3 Target (RTF < {phase3_target}): NOT MET (current: {mean_rtf:.3f})")

    if cv_rtf < 10:
        print(f"✅ Variance (CV < 10%): EXCELLENT ({cv_rtf:.1f}%)")
    elif cv_rtf < 20:
        print(f"⚠️  Variance (CV < 20%): ACCEPTABLE ({cv_rtf:.1f}%)")
    else:
        print(f"❌ Variance (CV < 20%): POOR ({cv_rtf:.1f}%)")

    print()

    # Recommendations
    print_separator()
    print("RECOMMENDATIONS")
    print_separator()

    if mean_rtf >= phase3_target:
        improvement_needed = ((mean_rtf - phase3_target) / mean_rtf) * 100
        print(f"- Need {improvement_needed:.1f}% improvement to reach Phase 3 target (RTF < {phase3_target})")
        print(f"- Current: RTF={mean_rtf:.3f}, Target: RTF={phase3_target:.3f}")
        print()
        print("Suggested optimizations:")
        print("  1. Test NFE=6 (expected 14% speedup)")
        print("  2. Enable IndexTTS FP16 for faster fallback")
        print("  3. Implement batch processing for multiple requests")
        print("  4. Profile to find remaining bottlenecks")
    else:
        print(f"✅ All performance targets achieved!")
        print(f"✅ RTF={mean_rtf:.3f} (target: <{phase3_target})")
        print(f"✅ Variance={cv_rtf:.1f}% (target: <10%)")

    if cv_rtf > 10:
        print()
        print("Variance reduction suggestions:")
        print("  1. Verify GPU frequency is locked (jetson_clocks)")
        print("  2. Reduce background processes")
        print("  3. Test with more warmup iterations")
        print("  4. Check thermal throttling")

    print()
    print_separator()

    # Save results to file
    results_file = ".agent/performance_results_extended.txt"
    with open(results_file, "w") as f:
        f.write(f"Extended Performance Test Results\n")
        f.write(f"Date: {time.strftime('%Y-%m-%d %H:%M:%S')}\n")
        f.write(f"Runs: {NUM_RUNS}\n")
        f.write(f"NFE: {NFE_STEP}\n")
        f.write(f"\n")
        f.write(f"Mean RTF: {mean_rtf:.3f}\n")
        f.write(f"Median RTF: {median_rtf:.3f}\n")
        f.write(f"Min RTF: {min_rtf:.3f}\n")
        f.write(f"Max RTF: {max_rtf:.3f}\n")
        f.write(f"StdDev: {std_rtf:.3f}\n")
        f.write(f"CV: {cv_rtf:.1f}%\n")
        f.write(f"\n")
        f.write(f"Mean Speedup: {mean_speedup:.2f}x\n")
        f.write(f"Max Speedup: {max_speedup:.2f}x\n")
        f.write(f"Min Speedup: {min_speedup:.2f}x\n")
        f.write(f"\n")
        f.write(f"Phase 1 Target (RTF < {target_rtf}): {'ACHIEVED' if mean_rtf < target_rtf else 'NOT MET'}\n")
        f.write(f"Phase 3 Target (RTF < {phase3_target}): {'ACHIEVED' if mean_rtf < phase3_target else 'NOT MET'}\n")
        f.write(f"Variance Target (CV < 10%): {'ACHIEVED' if cv_rtf < 10 else 'NOT MET'}\n")

    print(f"Results saved to: {results_file}")

if __name__ == "__main__":
    main()