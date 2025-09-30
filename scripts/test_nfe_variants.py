#!/usr/bin/env python3
"""
Test different NFE step counts to find the optimal speed/quality trade-off.

Tests NFE = [6, 7, 8, 9, 10] to find the sweet spot.
"""

import time
import torch
import sys
import os
from pathlib import Path

# Add F5-TTS to Python path
sys.path.insert(0, str(Path(__file__).parent.parent / "third_party" / "F5-TTS" / "src"))

from f5_tts.api import F5TTS

def test_nfe_variant(model, nfe_step, test_text, ref_audio, ref_text, num_runs=3):
    """Test a specific NFE step count."""
    print(f"\n{'='*70}")
    print(f"Testing NFE={nfe_step}")
    print(f"{'='*70}")

    times = []
    rtfs = []

    # Warmup run (don't count)
    _ = model.infer(
        ref_file=ref_audio,
        ref_text=ref_text,
        gen_text=test_text,
        show_info=lambda x: None,
        nfe_step=nfe_step,
    )

    # Actual test runs
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

        print(f"Run {i+1}: {elapsed:.3f}s | RTF: {rtf:.3f} | Speedup: {speedup:.2f}x | Audio: {audio_duration:.2f}s")

    mean_time = sum(times) / len(times)
    mean_rtf = sum(rtfs) / len(rtfs)
    best_rtf = min(rtfs)

    print(f"\nNFE={nfe_step} Summary:")
    print(f"  Mean RTF: {mean_rtf:.3f}")
    print(f"  Best RTF: {best_rtf:.3f}")
    print(f"  Mean time: {mean_time:.3f}s")

    return {
        'nfe': nfe_step,
        'mean_rtf': mean_rtf,
        'best_rtf': best_rtf,
        'mean_time': mean_time,
        'speedup': 1.0 / mean_rtf,
    }

def main():
    print("="*70)
    print("NFE Variants Performance Test")
    print("="*70)
    print(f"PyTorch: {torch.__version__}")
    print(f"CUDA: {torch.cuda.is_available()}")
    print()

    # Test configuration
    ref_audio = "/ssd/ishowtts/third_party/F5-TTS/src/f5_tts/infer/examples/basic/basic_ref_en.wav"
    ref_text = "No, you clearly don't know who you're talking to, so let me clue you in."
    test_text = "Hello world, this is a test of the optimized TTS system."

    # NFE values to test
    nfe_values = [6, 7, 8, 9, 10]

    print("[Init Model]")
    start = time.time()
    model = F5TTS()
    init_time = time.time() - start
    print(f"Init: {init_time:.2f}s")

    # Run tests for each NFE value
    results = []
    for nfe in nfe_values:
        result = test_nfe_variant(model, nfe, test_text, ref_audio, ref_text, num_runs=3)
        results.append(result)

    # Summary comparison
    print("\n" + "="*70)
    print("SUMMARY - All NFE Variants")
    print("="*70)
    print(f"{'NFE':<6} {'Mean RTF':<12} {'Best RTF':<12} {'Speedup':<10} {'vs NFE=8':<12}")
    print("-" * 70)

    baseline_idx = next(i for i, r in enumerate(results) if r['nfe'] == 8)
    baseline_rtf = results[baseline_idx]['mean_rtf']

    for result in results:
        improvement = (baseline_rtf / result['mean_rtf'] - 1) * 100
        marker = " ✅" if result['mean_rtf'] < 0.20 else " ⚠️" if result['mean_rtf'] < 0.25 else ""

        print(f"{result['nfe']:<6} {result['mean_rtf']:<12.3f} {result['best_rtf']:<12.3f} "
              f"{result['speedup']:<10.2f}x {improvement:+.1f}%{marker}")

    # Find best result
    best_result = min(results, key=lambda r: r['mean_rtf'])
    print("\n" + "="*70)
    print(f"🏆 Best Configuration: NFE={best_result['nfe']}")
    print(f"   Mean RTF: {best_result['mean_rtf']:.3f}")
    print(f"   Best RTF: {best_result['best_rtf']:.3f}")
    print(f"   Speedup: {best_result['speedup']:.2f}x")

    if best_result['mean_rtf'] < 0.20:
        print(f"\n✅ Phase 3 target achieved! (RTF < 0.20)")
    else:
        gap = best_result['mean_rtf'] - 0.20
        print(f"\n⚠️ Phase 3 target not yet met. Gap: {gap:.3f}")

    print("="*70)

if __name__ == "__main__":
    main()