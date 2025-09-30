#!/usr/bin/env python3
"""
Comprehensive validation test for NFE=7 configuration.

Runs 10 iterations to establish reliable statistics for production deployment.
"""

import time
import torch
import sys
import os
from pathlib import Path

# Add F5-TTS to Python path
sys.path.insert(0, str(Path(__file__).parent.parent / "third_party" / "F5-TTS" / "src"))

from f5_tts.api import F5TTS

def main():
    print("="*70)
    print("NFE=7 Production Validation Test")
    print("="*70)
    print(f"PyTorch: {torch.__version__}")
    print(f"CUDA: {torch.cuda.is_available()}")
    print()

    # Test configuration
    ref_audio = "/ssd/ishowtts/third_party/F5-TTS/src/f5_tts/infer/examples/basic/basic_ref_en.wav"
    ref_text = "No, you clearly don't know who you're talking to, so let me clue you in."
    test_text = "Hello world, this is a test of the optimized TTS system."
    nfe_step = 7
    num_runs = 10

    print("[Init Model]")
    start = time.time()
    model = F5TTS()
    init_time = time.time() - start
    print(f"Init: {init_time:.2f}s")

    print("\n[Warmup]")
    _ = model.infer(
        ref_file=ref_audio,
        ref_text=ref_text,
        gen_text=test_text,
        show_info=lambda x: None,
        nfe_step=nfe_step,
    )
    print("Warmup complete")

    print(f"\n[Test Runs - NFE={nfe_step}]")
    times = []
    rtfs = []
    audio_duration = 0

    for i in range(num_runs):
        torch.cuda.synchronize()
        start = time.time()

        wav, sr, _ = model.infer(
            ref_file=ref_audio,
            ref_text=ref_text,
            gen_text=test_text,
            show_info=lambda x: None,
            nfe_step=nfe_step,
        )

        torch.cuda.synchronize()
        elapsed = time.time() - start

        audio_duration = len(wav) / sr
        rtf = elapsed / audio_duration
        speedup = 1.0 / rtf

        times.append(elapsed)
        rtfs.append(rtf)

        print(f"Run {i+1:2d}: {elapsed:.3f}s | RTF: {rtf:.3f} | Speedup: {speedup:.2f}x")

    # Calculate statistics
    mean_time = sum(times) / len(times)
    best_time = min(times)
    worst_time = max(times)
    mean_rtf = sum(rtfs) / len(rtfs)
    best_rtf = min(rtfs)
    worst_rtf = max(rtfs)
    variance = (max(rtfs) - min(rtfs)) / mean_rtf * 100

    print("\n" + "="*70)
    print("RESULTS")
    print("="*70)
    print(f"Audio duration: {audio_duration:.3f}s")
    print(f"Number of runs: {num_runs}")
    print()
    print(f"Mean time: {mean_time:.3f}s")
    print(f"Best time: {best_time:.3f}s")
    print(f"Worst time: {worst_time:.3f}s")
    print()
    print(f"Mean RTF: {mean_rtf:.3f}")
    print(f"Best RTF: {best_rtf:.3f}")
    print(f"Worst RTF: {worst_rtf:.3f}")
    print(f"Variance: ±{variance:.1f}%")
    print()
    print(f"Mean speedup: {1.0/mean_rtf:.2f}x")
    print(f"Best speedup: {1.0/best_rtf:.2f}x")

    # Phase 3 target check
    print("\n" + "="*70)
    print("TARGET VALIDATION")
    print("="*70)

    phase1_target = 0.30
    phase3_target = 0.20

    if mean_rtf < phase3_target:
        print(f"✅ Phase 3 target achieved! (Mean RTF {mean_rtf:.3f} < {phase3_target})")
    elif best_rtf < phase3_target:
        print(f"⚠️ Phase 3 target met by best run (Best RTF {best_rtf:.3f} < {phase3_target})")
        print(f"   Mean RTF {mean_rtf:.3f} slightly above target")
    else:
        print(f"❌ Phase 3 target not met (Mean RTF {mean_rtf:.3f} > {phase3_target})")

    if mean_rtf < phase1_target:
        print(f"✅ Phase 1 target maintained (Mean RTF {mean_rtf:.3f} < {phase1_target})")

    # Improvement over baseline NFE=8
    baseline_rtf = 0.243  # From previous NFE=8 testing
    improvement = (baseline_rtf - mean_rtf) / baseline_rtf * 100
    speedup_gain = (1.0/mean_rtf) / (1.0/baseline_rtf)

    print(f"\n🚀 Improvement vs NFE=8 (RTF {baseline_rtf}):")
    print(f"   RTF improvement: {improvement:.1f}% faster")
    print(f"   Speedup gain: {speedup_gain:.2f}x")

    # Overall improvement from original baseline
    original_baseline = 1.32
    total_improvement = original_baseline / mean_rtf
    print(f"\n🎉 Total improvement from original baseline (RTF {original_baseline}):")
    print(f"   {total_improvement:.1f}x speedup")

    print("="*70)

if __name__ == "__main__":
    main()