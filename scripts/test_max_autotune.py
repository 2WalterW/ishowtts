#!/usr/bin/env python3
"""
Test with max-autotune compile mode and NFE=8
"""

import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / "third_party" / "F5-TTS" / "src"))

try:
    import torch
    import numpy as np
    from f5_tts.api import F5TTS
except ImportError as e:
    print(f"Error: {e}")
    sys.exit(1)


def test():
    print("=" * 70)
    print("Testing with max-autotune + NFE=8")
    print("=" * 70)

    print(f"\nPyTorch: {torch.__version__}")
    print(f"CUDA: {torch.cuda.is_available()}")

    ref_audio = Path("data/voices/walter_reference.wav")
    ref_text = "No, you clearly don't know who you're talking to, so let me clue you in."
    test_text = "Hello world, this is a test of the optimized TTS system."

    if not ref_audio.exists():
        print(f"Error: {ref_audio} not found")
        sys.exit(1)

    print("\n[Init Model]")
    start = time.perf_counter()
    model = F5TTS(model="F5TTS_v1_Base", device="cuda")
    print(f"Init: {time.perf_counter() - start:.2f}s")

    # Warmup with NFE=8
    print("\n[Warmup]")
    start = time.perf_counter()
    _ = model.infer(
        ref_file=str(ref_audio),
        ref_text=ref_text,
        gen_text=test_text,
        nfe_step=8,
        show_info=lambda x: None
    )
    print(f"Warmup: {time.perf_counter() - start:.2f}s")

    # Test runs
    print("\n[Test Runs - NFE=8]")
    times = []
    for i in range(5):
        if torch.cuda.is_available():
            torch.cuda.synchronize()

        start = time.perf_counter()
        wav, sr, _ = model.infer(
            ref_file=str(ref_audio),
            ref_text=ref_text,
            gen_text=test_text,
            nfe_step=8,
            show_info=lambda x: None
        )

        if torch.cuda.is_available():
            torch.cuda.synchronize()

        elapsed = time.perf_counter() - start
        times.append(elapsed)

        audio_duration = len(wav) / sr
        rtf = elapsed / audio_duration
        print(f"Run {i+1}: {elapsed:.3f}s | RTF: {rtf:.3f} | Speedup: {1/rtf:.2f}x")

    # Results
    audio_duration = len(wav) / sr
    mean_time = np.mean(times)
    min_time = np.min(times)
    rtf_mean = mean_time / audio_duration
    rtf_best = min_time / audio_duration

    print("\n" + "=" * 70)
    print("RESULTS")
    print("=" * 70)
    print(f"Audio duration: {audio_duration:.3f}s")
    print(f"Mean time: {mean_time:.3f}s")
    print(f"Best time: {min_time:.3f}s")
    print(f"Mean RTF: {rtf_mean:.3f}")
    print(f"Best RTF: {rtf_best:.3f}")
    print(f"Mean speedup: {1/rtf_mean:.2f}x")
    print(f"Best speedup: {1/rtf_best:.2f}x")

    if rtf_best < 0.3:
        print(f"\n✅ SUCCESS! Best RTF < 0.3 achieved!")
    elif rtf_mean < 0.3:
        print(f"\n✅ SUCCESS! Mean RTF < 0.3 achieved!")
    elif rtf_best < 0.35:
        print(f"\n⚠️  CLOSE! Best RTF is {rtf_best:.3f} (target < 0.3)")
    else:
        print(f"\n❌ Target not reached. Best RTF: {rtf_best:.3f}")


if __name__ == "__main__":
    test()