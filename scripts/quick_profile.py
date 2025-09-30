#!/usr/bin/env python3
"""
Quick profiling script to identify bottlenecks in F5-TTS inference.
Measures component-level timing for optimization planning.
"""

import sys
import time
from pathlib import Path

import torch
import numpy as np

# Add F5-TTS to path
f5_tts_path = Path(__file__).parent.parent / "third_party" / "F5-TTS" / "src"
sys.path.insert(0, str(f5_tts_path))

from f5_tts.api import F5TTS


def profile_tts(num_runs=5):
    """Profile TTS inference with component timing."""

    print("="*80)
    print("Quick TTS Profiling")
    print("="*80)
    print()

    # Setup
    ref_audio = "data/voices/walter_reference.wav"
    ref_text = "No, you clearly don't know who you're talking to, so let me clue you in."
    gen_text = "Hello world, this is a test of the optimized TTS system."

    print(f"PyTorch: {torch.__version__}")
    print(f"CUDA: {torch.cuda.is_available()}")
    print(f"Device: {torch.cuda.get_device_name(0) if torch.cuda.is_available() else 'CPU'}")
    print()

    # Initialize model
    print("Initializing F5-TTS...")
    init_start = time.perf_counter()
    model = F5TTS(model="F5TTS_v1_Base", device="cuda")
    init_time = time.perf_counter() - init_start
    print(f"Init time: {init_time:.2f}s")
    print()

    # Warmup
    print("Warming up...")
    for _ in range(2):
        _ = model.infer(ref_file=ref_audio, ref_text=ref_text, gen_text=gen_text)
    print("Warmup complete.")
    print()

    # Profile runs
    print(f"Running {num_runs} inference tests...")
    times = []

    for i in range(num_runs):
        # Clear cache
        if torch.cuda.is_available():
            torch.cuda.synchronize()

        start = time.perf_counter()
        audio, sr, _ = model.infer(ref_file=ref_audio, ref_text=ref_text, gen_text=gen_text)

        if torch.cuda.is_available():
            torch.cuda.synchronize()

        elapsed = time.perf_counter() - start
        times.append(elapsed)

        audio_duration = len(audio) / sr
        rtf = elapsed / audio_duration
        speedup = audio_duration / elapsed

        print(f"Run {i+1}: {elapsed:.3f}s | RTF: {rtf:.3f} | Speedup: {speedup:.2f}x | Audio: {audio_duration:.2f}s")

    print()
    print("="*80)
    print("Results Summary")
    print("="*80)

    mean_time = np.mean(times)
    std_time = np.std(times)
    min_time = np.min(times)
    max_time = np.max(times)

    mean_rtf = mean_time / audio_duration
    min_rtf = min_time / audio_duration

    print(f"Audio duration: {audio_duration:.2f}s")
    print(f"Mean time: {mean_time:.3f}s (±{std_time:.3f}s)")
    print(f"Best time: {min_time:.3f}s")
    print(f"Worst time: {max_time:.3f}s")
    print(f"Mean RTF: {mean_rtf:.3f}")
    print(f"Best RTF: {min_rtf:.3f}")
    print(f"Variance: {100*std_time/mean_time:.1f}%")
    print()

    # GPU utilization
    if torch.cuda.is_available():
        memory_allocated = torch.cuda.max_memory_allocated() / 1024**3
        memory_reserved = torch.cuda.max_memory_reserved() / 1024**3
        print(f"GPU Memory:")
        print(f"  Allocated: {memory_allocated:.2f} GB")
        print(f"  Reserved: {memory_reserved:.2f} GB")
        print()

    # Analysis
    print("="*80)
    print("Analysis & Next Steps")
    print("="*80)
    print()

    if mean_rtf < 0.25:
        print("✅ EXCELLENT performance (RTF < 0.25)")
        print("   Consider Phase 3 optimizations for RTF < 0.20:")
        print("   - INT8 quantization")
        print("   - Model architecture tuning")
        print("   - Streaming inference")
    elif mean_rtf < 0.30:
        print("✅ GOOD performance (RTF < 0.30) - Phase 1 target met")
        print("   Consider Phase 3 optimizations:")
        print("   - INT8 quantization (1.5-2x potential)")
        print("   - Further torch.compile tuning")
    elif mean_rtf < 0.40:
        print("⚠️  ACCEPTABLE performance (RTF < 0.40)")
        print("   Review Phase 1 optimizations:")
        print("   - Verify torch.compile enabled")
        print("   - Check GPU frequency lock")
        print("   - Verify NFE=8 setting")
    else:
        print("❌ POOR performance (RTF >= 0.40)")
        print("   Action required:")
        print("   - Check GPU is being used")
        print("   - Verify all optimizations applied")
        print("   - Check for competing processes")

    print()

    # Bottleneck estimation (rough heuristics from previous profiling)
    estimated_model_time = mean_time * 0.70  # Model typically 70%
    estimated_vocoder_time = mean_time * 0.25  # Vocoder typically 25%
    estimated_other_time = mean_time * 0.05  # Other 5%

    print("Estimated Component Breakdown (rough):")
    print(f"  Model inference: {estimated_model_time:.3f}s ({100*0.70:.0f}%)")
    print(f"  Vocoder:         {estimated_vocoder_time:.3f}s ({100*0.25:.0f}%)")
    print(f"  Other:           {estimated_other_time:.3f}s ({100*0.05:.0f}%)")
    print()
    print("Note: Run full profiler (profile_bottlenecks.py) for accurate breakdown")
    print()

    return {
        'mean_rtf': mean_rtf,
        'best_rtf': min_rtf,
        'variance_pct': 100*std_time/mean_time,
        'audio_duration': audio_duration,
    }


if __name__ == "__main__":
    results = profile_tts(num_runs=5)