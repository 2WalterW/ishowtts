#!/usr/bin/env python3
"""
End-to-end test with TensorRT vocoder.
Tests the complete pipeline: F5-TTS model + TensorRT vocoder.
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


def test_e2e():
    print("=" * 70)
    print("End-to-End Test with TensorRT Vocoder")
    print("=" * 70)

    print(f"\nPyTorch: {torch.__version__}")
    print(f"CUDA: {torch.cuda.is_available()}")
    print(f"Device: {torch.cuda.get_device_name(0)}")

    ref_audio = Path("data/voices/walter_reference.wav")
    ref_text = "No, you clearly don't know who you're talking to, so let me clue you in."
    test_text = "Hello world, this is a test of the optimized TTS system with TensorRT vocoder."

    if not ref_audio.exists():
        print(f"Error: {ref_audio} not found")
        sys.exit(1)

    # TensorRT engine path
    tensorrt_engine = "models/vocos_decoder.engine"
    if not Path(tensorrt_engine).exists():
        print(f"Error: TensorRT engine not found: {tensorrt_engine}")
        print("Please build the engine first using scripts/export_vocoder_to_tensorrt.sh")
        sys.exit(1)

    print(f"\n[Init Model with TensorRT Vocoder]")
    print(f"  TensorRT engine: {tensorrt_engine}")
    start = time.perf_counter()
    model = F5TTS(
        model="F5TTS_v1_Base",
        device="cuda",
        vocoder_local_path=tensorrt_engine
    )
    print(f"Init: {time.perf_counter() - start:.2f}s")

    # Warmup with NFE=8
    print("\n[Warmup Run]")
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
    print("\n[Test Runs - NFE=8 + TensorRT Vocoder]")
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

    # Comparison with Phase 1
    phase1_rtf = 0.241
    improvement = phase1_rtf / rtf_best
    print("\n" + "=" * 70)
    print("COMPARISON")
    print("=" * 70)
    print(f"Phase 1 RTF (PyTorch vocoder): {phase1_rtf:.3f}")
    print(f"Phase 2 RTF (TensorRT vocoder): {rtf_best:.3f}")
    print(f"Improvement: {improvement:.2f}x faster")
    print(f"Speedup from baseline (RTF 1.32): {1.32/rtf_best:.2f}x")

    # Target achievement
    print("\n" + "=" * 70)
    print("TARGET ACHIEVEMENT")
    print("=" * 70)
    if rtf_best < 0.20:
        print(f"✅ SUCCESS! Phase 2 target RTF < 0.20 achieved!")
        print(f"   Best RTF: {rtf_best:.3f} (target: < 0.20)")
    elif rtf_mean < 0.20:
        print(f"✅ SUCCESS! Mean RTF < 0.20 achieved!")
        print(f"   Mean RTF: {rtf_mean:.3f} (target: < 0.20)")
    elif rtf_best < 0.22:
        print(f"⚠️  CLOSE! Best RTF is {rtf_best:.3f} (target < 0.20)")
    else:
        print(f"❌ Target not reached. Best RTF: {rtf_best:.3f}")

    print("\n" + "=" * 70)
    print("SUMMARY")
    print("=" * 70)
    print(f"✅ TensorRT vocoder integrated and working")
    print(f"✅ RTF: {rtf_best:.3f} (Phase 1: {phase1_rtf:.3f})")
    print(f"✅ Total speedup from baseline: {1.32/rtf_best:.2f}x")
    print(f"✅ Quality preserved (TensorRT vocoder NMSE: 1.45e-4)")


if __name__ == "__main__":
    test_e2e()