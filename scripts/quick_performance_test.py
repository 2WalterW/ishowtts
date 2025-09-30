#!/usr/bin/env python3
"""
Quick performance test to validate optimizations
"""

import sys
import time
from pathlib import Path

# Add F5-TTS to path
sys.path.insert(0, str(Path(__file__).parent.parent / "third_party" / "F5-TTS" / "src"))

try:
    import torch
    from f5_tts.api import F5TTS
except ImportError as e:
    print(f"Error importing dependencies: {e}")
    print("Make sure you have activated the ishowtts environment")
    sys.exit(1)


def quick_test():
    """Run a quick performance test"""

    print("=" * 70)
    print("Quick Performance Test")
    print("=" * 70)

    # System info
    print(f"\nPyTorch: {torch.__version__}")
    print(f"CUDA available: {torch.cuda.is_available()}")
    if torch.cuda.is_available():
        print(f"CUDA device: {torch.cuda.get_device_name(0)}")
        print(f"CUDA version: {torch.version.cuda}")

    # Reference files
    ref_audio = Path("data/voices/walter_reference.wav")
    ref_text = "No, you clearly don't know who you're talking to, so let me clue you in."

    if not ref_audio.exists():
        print(f"\nError: Reference audio not found: {ref_audio}")
        sys.exit(1)

    # Initialize model
    print("\n[Initializing Model]")
    start = time.perf_counter()
    model = F5TTS(model_type="F5-TTS", device="cuda" if torch.cuda.is_available() else "cpu")
    init_time = time.perf_counter() - start
    print(f"Initialization time: {init_time:.2f}s")

    # Test text
    test_text = "Hello world, this is a quick test of the optimized TTS system."

    # Warmup run
    print("\n[Warmup Run]")
    start = time.perf_counter()
    try:
        _ = model.infer(
            ref_file=str(ref_audio),
            ref_text=ref_text,
            gen_text=test_text,
            nfe_step=16,
            show_info=lambda x: None
        )
        warmup_time = time.perf_counter() - start
        print(f"Warmup time: {warmup_time:.2f}s (includes torch.compile overhead)")
    except Exception as e:
        print(f"Warmup failed: {e}")
        import traceback
        traceback.print_exc()
        return

    # Actual test runs
    print("\n[Performance Test - 3 runs]")
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
                nfe_step=16,
                show_info=lambda x: None
            )
        except Exception as e:
            print(f"Run {i+1} failed: {e}")
            continue

        if torch.cuda.is_available():
            torch.cuda.synchronize()

        elapsed = time.perf_counter() - start
        times.append(elapsed)

        # Calculate RTF
        audio_duration = len(wav) / sr
        rtf = elapsed / audio_duration

        print(f"Run {i+1}: {elapsed:.3f}s | Audio: {audio_duration:.3f}s | RTF: {rtf:.3f} | Speedup: {1/rtf:.2f}x")

    if times:
        import numpy as np
        mean_time = np.mean(times)
        mean_rtf = mean_time / audio_duration

        print("\n" + "=" * 70)
        print("RESULTS")
        print("=" * 70)
        print(f"Average synthesis time: {mean_time:.3f}s")
        print(f"Average RTF: {mean_rtf:.3f}")
        print(f"Average speedup: {1/mean_rtf:.2f}x real-time")

        if mean_rtf < 0.3:
            print("\n✅ SUCCESS! Achieved Whisper-level performance (RTF < 0.3)")
        elif mean_rtf < 0.5:
            print("\n✅ GOOD! Suitable for real-time streaming (RTF < 0.5)")
        elif mean_rtf < 1.0:
            print("\n⚠️  ACCEPTABLE but could be better (RTF < 1.0)")
        else:
            print("\n❌ NEEDS OPTIMIZATION (RTF >= 1.0)")

        # Check optimizations
        print("\n[Optimization Status]")
        print(f"torch.compile enabled: {hasattr(torch, 'compile')}")
        print(f"CUDA available: {torch.cuda.is_available()}")
        print(f"FP16 AMP available: {torch.cuda.is_available() and hasattr(torch, 'amp')}")


if __name__ == "__main__":
    quick_test()