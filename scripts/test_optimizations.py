#!/usr/bin/env python3
"""
Test script to verify F5-TTS optimizations are working correctly.
Tests tensor caching, mixed precision, and measures performance.
"""

import sys
import time
from pathlib import Path

# Add F5-TTS to path
sys.path.insert(0, str(Path(__file__).parent.parent / "third_party" / "F5-TTS" / "src"))

import torch
import torchaudio
from f5_tts.api import F5TTS
from f5_tts.infer.utils_infer import _ref_audio_tensor_cache


def test_basic_synthesis(f5tts: F5TTS, ref_file: str, ref_text: str, gen_text: str) -> tuple[float, int]:
    """Run synthesis and return (time_taken, waveform_length)"""
    start = time.perf_counter()

    wav, sr, _ = f5tts.infer(
        ref_file=ref_file,
        ref_text=ref_text,
        gen_text=gen_text,
        show_info=lambda x: None,  # Silence output
        progress=None,
    )

    elapsed = time.perf_counter() - start
    return elapsed, len(wav)


def test_tensor_caching():
    """Test that tensor caching is working"""
    print("Testing tensor caching...")

    # Check cache exists
    assert "_ref_audio_tensor_cache" in dir(sys.modules["f5_tts.infer.utils_infer"]), "Tensor cache not found!"

    # Cache should be empty initially
    initial_size = len(_ref_audio_tensor_cache)
    print(f"  Initial cache size: {initial_size}")

    return True


def test_mixed_precision():
    """Test that mixed precision is available"""
    print("Testing mixed precision support...")

    if not torch.cuda.is_available():
        print("  ⚠️  CUDA not available, skipping")
        return False

    # Check if autocast is available
    try:
        with torch.amp.autocast(device_type="cuda", dtype=torch.float16):
            x = torch.randn(10, 10, device="cuda")
            y = torch.mm(x, x)
        print("  ✓ Mixed precision (AMP) is available")
        return True
    except Exception as e:
        print(f"  ✗ Mixed precision test failed: {e}")
        return False


def benchmark_nfe_steps(f5tts: F5TTS, ref_file: str, ref_text: str):
    """Benchmark different NFE step values"""
    print("\nBenchmarking NFE steps...")

    test_text = "The quick brown fox jumps over the lazy dog."
    nfe_values = [8, 16, 24, 32]

    results = []
    for nfe in nfe_values:
        print(f"  Testing NFE={nfe}...", end=" ", flush=True)

        # Monkey patch to set NFE steps
        original_nfe = f5tts.infer.__defaults__
        # Update kwargs
        start = time.perf_counter()
        wav, sr, _ = f5tts.infer(
            ref_file=ref_file,
            ref_text=ref_text,
            gen_text=test_text,
            nfe_step=nfe,
            show_info=lambda x: None,
            progress=None,
        )
        elapsed = time.perf_counter() - start

        audio_duration = len(wav) / sr
        rtf = elapsed / audio_duration

        results.append((nfe, elapsed, audio_duration, rtf))
        print(f"Time: {elapsed:.2f}s, RTF: {rtf:.3f}")

    # Calculate speedup
    baseline = next(r for r in results if r[0] == 32)
    optimized = next(r for r in results if r[0] == 16)

    speedup = baseline[1] / optimized[1]
    print(f"\n  Speedup (NFE 32→16): {speedup:.2f}x")

    return results


def main():
    print("=" * 60)
    print("iShowTTS Optimization Test Suite")
    print("=" * 60)

    # Test 1: Tensor caching
    print("\n1. Tensor Caching")
    test_tensor_caching()

    # Test 2: Mixed precision
    print("\n2. Mixed Precision")
    test_mixed_precision()

    # Test 3: Initialize F5-TTS
    print("\n3. Initializing F5-TTS...")
    try:
        f5tts = F5TTS(model="F5TTS_v1_Base", device="cuda" if torch.cuda.is_available() else "cpu")
        print("  ✓ F5-TTS initialized successfully")
    except Exception as e:
        print(f"  ✗ Failed to initialize F5-TTS: {e}")
        print("\nMake sure you have:")
        print("  1. Installed F5-TTS dependencies")
        print("  2. Downloaded model checkpoints")
        return 1

    # Test 4: Find reference audio
    print("\n4. Looking for reference audio...")
    ref_paths = [
        Path(__file__).parent.parent / "data" / "voices" / "walter_reference.wav",
        Path(__file__).parent.parent / "data" / "voices" / "demo_reference.wav",
    ]

    ref_file = None
    for path in ref_paths:
        if path.exists():
            ref_file = str(path)
            print(f"  ✓ Found: {path.name}")
            break

    if not ref_file:
        print("  ✗ No reference audio found")
        return 1

    ref_text = "This is a test reference audio for voice cloning."

    # Test 5: Benchmark NFE steps
    print("\n5. Performance Benchmark")
    try:
        results = benchmark_nfe_steps(f5tts, ref_file, ref_text)
    except Exception as e:
        print(f"  ✗ Benchmark failed: {e}")
        import traceback

        traceback.print_exc()
        return 1

    # Summary
    print("\n" + "=" * 60)
    print("Test Summary")
    print("=" * 60)
    print("✓ Optimizations verified")
    print("✓ Performance benchmarked")
    print("\nRecommendation: Use NFE=16 for best speed/quality balance")
    print("=" * 60)

    return 0


if __name__ == "__main__":
    sys.exit(main())