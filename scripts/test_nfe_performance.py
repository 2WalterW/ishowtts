#!/usr/bin/env python3
"""
Test performance with different NFE step values
"""

import sys
import time
from pathlib import Path

# Add F5-TTS to path
sys.path.insert(0, str(Path(__file__).parent.parent / "third_party" / "F5-TTS" / "src"))

try:
    import torch
    import numpy as np
    from f5_tts.api import F5TTS
except ImportError as e:
    print(f"Error importing dependencies: {e}")
    sys.exit(1)


def test_nfe_values():
    """Test different NFE values"""

    print("=" * 70)
    print("NFE Performance Comparison")
    print("=" * 70)

    # System info
    print(f"\nPyTorch: {torch.__version__}")
    print(f"CUDA: {torch.cuda.is_available()}")

    # Reference files
    ref_audio = Path("data/voices/walter_reference.wav")
    ref_text = "No, you clearly don't know who you're talking to, so let me clue you in."

    if not ref_audio.exists():
        print(f"\nError: Reference audio not found: {ref_audio}")
        sys.exit(1)

    # Initialize model
    print("\n[Initializing Model]")
    start = time.perf_counter()
    model = F5TTS(model="F5TTS_v1_Base", device="cuda" if torch.cuda.is_available() else "cpu")
    init_time = time.perf_counter() - start
    print(f"Init time: {init_time:.2f}s")

    # Test text
    test_text = "Hello world, this is a test of the TTS system with various NFE configurations."

    # NFE values to test
    nfe_values = [8, 12, 16, 20, 24, 32]

    print("\n[Running Tests]")
    results = []

    for nfe in nfe_values:
        print(f"\n--- Testing NFE={nfe} ---")

        # Warmup run
        try:
            _ = model.infer(
                ref_file=str(ref_audio),
                ref_text=ref_text,
                gen_text=test_text,
                nfe_step=nfe,
                show_info=lambda x: None
            )
        except Exception as e:
            print(f"Warmup failed: {e}")
            continue

        # Test runs
        times = []
        for i in range(3):
            if torch.cuda.is_available():
                torch.cuda.synchronize()

            start = time.perf_counter()

            try:
                wav, sr, _ = model.infer(
                    ref_file=str(ref_audio),
                    ref_text=ref_text,
                    gen_text=test_text,
                    nfe_step=nfe,
                    show_info=lambda x: None
                )
            except Exception as e:
                print(f"Run {i+1} failed: {e}")
                continue

            if torch.cuda.is_available():
                torch.cuda.synchronize()

            elapsed = time.perf_counter() - start
            times.append(elapsed)

        if times:
            audio_duration = len(wav) / sr
            mean_time = np.mean(times)
            rtf = mean_time / audio_duration

            results.append({
                'nfe': nfe,
                'mean_time': mean_time,
                'audio_duration': audio_duration,
                'rtf': rtf,
                'speedup': 1.0 / rtf
            })

            print(f"  Time: {mean_time:.3f}s | RTF: {rtf:.3f} | Speedup: {1/rtf:.2f}x")

    # Summary
    print("\n" + "=" * 70)
    print("SUMMARY")
    print("=" * 70)
    print(f"\n{'NFE':<8} {'Time(s)':<12} {'RTF':<12} {'Speedup':<12} {'Quality':<12}")
    print("-" * 70)

    for r in results:
        quality = ""
        if r['nfe'] >= 32:
            quality = "Excellent"
        elif r['nfe'] >= 16:
            quality = "Good"
        elif r['nfe'] >= 12:
            quality = "Fair"
        else:
            quality = "Lower"

        status = ""
        if r['rtf'] < 0.3:
            status = " ✅ TARGET"
        elif r['rtf'] < 0.5:
            status = " ✓ Good"

        print(f"{r['nfe']:<8} {r['mean_time']:<12.3f} {r['rtf']:<12.3f} "
              f"{r['speedup']:<12.2f} {quality:<12} {status}")

    # Find best NFE for target
    print("\n" + "=" * 70)
    print("RECOMMENDATIONS")
    print("=" * 70)

    target_nfe = None
    for r in results:
        if r['rtf'] < 0.3:
            if target_nfe is None or r['nfe'] > target_nfe['nfe']:
                target_nfe = r

    if target_nfe:
        print(f"\n✅ SUCCESS! NFE={target_nfe['nfe']} achieves RTF < 0.3")
        print(f"   RTF: {target_nfe['rtf']:.3f}")
        print(f"   Speedup: {target_nfe['speedup']:.2f}x")
        print(f"\nRecommendation: Set default_nfe_step = {target_nfe['nfe']} in config")
    else:
        print("\n❌ No NFE value achieved RTF < 0.3")
        print("   Further optimizations needed (TensorRT, INT8, etc.)")


if __name__ == "__main__":
    test_nfe_values()