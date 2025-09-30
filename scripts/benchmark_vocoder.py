#!/usr/bin/env python3
"""
Benchmark TensorRT vocoder vs PyTorch vocoder.

This script compares the inference speed and accuracy of TensorRT vs PyTorch Vocos vocoder.
"""
import sys
import time
import torch
import numpy as np
from pathlib import Path

# Add F5-TTS to path
sys.path.insert(0, str(Path(__file__).parent.parent / "third_party" / "F5-TTS" / "src"))

from vocos import Vocos
from f5_tts.infer.tensorrt_vocoder import TensorRTVocoder, TRT_AVAILABLE


def benchmark_pytorch_vocoder(vocoder, mel_inputs, warmup=2, runs=10):
    """Benchmark PyTorch vocoder."""
    print("\n" + "=" * 70)
    print("PyTorch Vocoder Benchmark")
    print("=" * 70)

    times = []

    # Warmup
    print(f"\nWarmup: {warmup} runs...")
    for i in range(warmup):
        with torch.no_grad():
            _ = vocoder.decode(mel_inputs[0])
        print(f"  Warmup {i+1}/{warmup} done")

    # Benchmark
    print(f"\nBenchmark: {runs} runs...")
    for i, mel in enumerate(mel_inputs[:runs]):
        torch.cuda.synchronize()
        start = time.time()

        with torch.no_grad():
            audio = vocoder.decode(mel)

        torch.cuda.synchronize()
        elapsed = time.time() - start
        times.append(elapsed * 1000)  # Convert to ms

        print(f"  Run {i+1}/{runs}: {elapsed*1000:.2f} ms | Audio: {tuple(audio.shape)}")

    mean_time = np.mean(times)
    std_time = np.std(times)
    min_time = np.min(times)

    print(f"\n📊 PyTorch Results:")
    print(f"  Mean: {mean_time:.2f} ms")
    print(f"  Std:  {std_time:.2f} ms")
    print(f"  Min:  {min_time:.2f} ms")
    print(f"  Max:  {np.max(times):.2f} ms")

    return audio, mean_time, std_time


def benchmark_tensorrt_vocoder(vocoder, mel_inputs, warmup=2, runs=10):
    """Benchmark TensorRT vocoder."""
    print("\n" + "=" * 70)
    print("TensorRT Vocoder Benchmark")
    print("=" * 70)

    times = []

    # Warmup
    print(f"\nWarmup: {warmup} runs...")
    for i in range(warmup):
        _ = vocoder.decode(mel_inputs[0])
        print(f"  Warmup {i+1}/{warmup} done")

    # Benchmark
    print(f"\nBenchmark: {runs} runs...")
    for i, mel in enumerate(mel_inputs[:runs]):
        torch.cuda.synchronize()
        start = time.time()

        audio = vocoder.decode(mel)

        torch.cuda.synchronize()
        elapsed = time.time() - start
        times.append(elapsed * 1000)  # Convert to ms

        print(f"  Run {i+1}/{runs}: {elapsed*1000:.2f} ms | Audio: {tuple(audio.shape)}")

    mean_time = np.mean(times)
    std_time = np.std(times)
    min_time = np.min(times)

    print(f"\n📊 TensorRT Results:")
    print(f"  Mean: {mean_time:.2f} ms")
    print(f"  Std:  {std_time:.2f} ms")
    print(f"  Min:  {min_time:.2f} ms")
    print(f"  Max:  {np.max(times):.2f} ms")

    return audio, mean_time, std_time


def compare_accuracy(pytorch_audio, tensorrt_audio):
    """Compare audio outputs for accuracy."""
    print("\n" + "=" * 70)
    print("Accuracy Comparison")
    print("=" * 70)

    # Ensure same shape
    if pytorch_audio.shape != tensorrt_audio.shape:
        print(f"⚠️  Shape mismatch:")
        print(f"  PyTorch:   {tuple(pytorch_audio.shape)}")
        print(f"  TensorRT:  {tuple(tensorrt_audio.shape)}")
        # Trim to same length
        min_len = min(pytorch_audio.shape[-1], tensorrt_audio.shape[-1])
        pytorch_audio = pytorch_audio[..., :min_len]
        tensorrt_audio = tensorrt_audio[..., :min_len]
        print(f"  Trimmed to: {tuple(pytorch_audio.shape)}")

    # Convert to numpy
    pt_np = pytorch_audio.cpu().numpy()
    trt_np = tensorrt_audio.cpu().numpy()

    # Calculate metrics
    mse = np.mean((pt_np - trt_np) ** 2)
    mae = np.mean(np.abs(pt_np - trt_np))
    max_diff = np.max(np.abs(pt_np - trt_np))

    # Normalized metrics
    pt_rms = np.sqrt(np.mean(pt_np ** 2))
    normalized_mse = mse / (pt_rms ** 2 + 1e-8)

    print(f"\n📏 Accuracy Metrics:")
    print(f"  MSE:            {mse:.2e}")
    print(f"  MAE:            {mae:.2e}")
    print(f"  Max Diff:       {max_diff:.2e}")
    print(f"  Normalized MSE: {normalized_mse:.2e}")

    # Quality assessment
    if normalized_mse < 1e-4:
        print(f"\n✅ Excellent match (NMSE < 1e-4)")
    elif normalized_mse < 1e-3:
        print(f"\n✅ Good match (NMSE < 1e-3)")
    elif normalized_mse < 1e-2:
        print(f"\n⚠️  Fair match (NMSE < 1e-2)")
    else:
        print(f"\n❌ Poor match (NMSE >= 1e-2)")

    return mse, normalized_mse


def main():
    import argparse
    parser = argparse.ArgumentParser(description="Benchmark TensorRT vs PyTorch vocoder")
    parser.add_argument(
        "--engine",
        type=str,
        default="models/vocos_decoder.engine",
        help="Path to TensorRT engine (default: models/vocos_decoder.engine)"
    )
    parser.add_argument(
        "--frames",
        type=int,
        default=256,
        help="Number of time frames for test input (default: 256)"
    )
    parser.add_argument(
        "--warmup",
        type=int,
        default=3,
        help="Number of warmup runs (default: 3)"
    )
    parser.add_argument(
        "--runs",
        type=int,
        default=20,
        help="Number of benchmark runs (default: 20)"
    )
    parser.add_argument(
        "--pytorch-only",
        action="store_true",
        help="Only benchmark PyTorch (skip TensorRT)"
    )

    args = parser.parse_args()

    print("=" * 70)
    print("Vocoder Performance Benchmark")
    print("=" * 70)
    print(f"\nConfiguration:")
    print(f"  Engine:  {args.engine}")
    print(f"  Frames:  {args.frames}")
    print(f"  Warmup:  {args.warmup}")
    print(f"  Runs:    {args.runs}")
    print(f"  Device:  {torch.cuda.get_device_name(0)}")

    # Generate test inputs
    print(f"\nGenerating test inputs...")
    torch.manual_seed(42)
    mel_inputs = [torch.randn(1, 100, args.frames).cuda() for _ in range(args.runs + args.warmup)]
    print(f"  Generated {len(mel_inputs)} inputs of shape (1, 100, {args.frames})")

    # Load PyTorch vocoder
    print(f"\nLoading PyTorch vocoder...")
    pytorch_vocoder = Vocos.from_pretrained("charactr/vocos-mel-24khz")
    pytorch_vocoder.eval()
    pytorch_vocoder.to("cuda")
    print(f"✅ PyTorch vocoder loaded")

    # Benchmark PyTorch
    pytorch_audio, pytorch_mean, pytorch_std = benchmark_pytorch_vocoder(
        pytorch_vocoder, mel_inputs, warmup=args.warmup, runs=args.runs
    )

    if args.pytorch_only:
        print("\n" + "=" * 70)
        print("BENCHMARK COMPLETE (PyTorch only)")
        print("=" * 70)
        return

    # Check TensorRT availability
    if not TRT_AVAILABLE:
        print("\n❌ TensorRT not available. Install with:")
        print("   pip install pycuda")
        print("   (TensorRT should be available from system packages)")
        return

    # Load TensorRT vocoder
    print(f"\nLoading TensorRT vocoder...")
    try:
        tensorrt_vocoder = TensorRTVocoder(args.engine, device="cuda")
        print(f"✅ TensorRT vocoder loaded")
    except Exception as e:
        print(f"❌ Failed to load TensorRT vocoder: {e}")
        return

    # Benchmark TensorRT
    tensorrt_audio, tensorrt_mean, tensorrt_std = benchmark_tensorrt_vocoder(
        tensorrt_vocoder, mel_inputs, warmup=args.warmup, runs=args.runs
    )

    # Compare accuracy
    mse, nmse = compare_accuracy(pytorch_audio, tensorrt_audio)

    # Summary
    print("\n" + "=" * 70)
    print("SUMMARY")
    print("=" * 70)

    speedup = pytorch_mean / tensorrt_mean

    print(f"\n⏱️  Performance:")
    print(f"  PyTorch:   {pytorch_mean:.2f} ± {pytorch_std:.2f} ms")
    print(f"  TensorRT:  {tensorrt_mean:.2f} ± {tensorrt_std:.2f} ms")
    print(f"  Speedup:   {speedup:.2f}x faster")

    print(f"\n📏 Accuracy:")
    print(f"  MSE:  {mse:.2e}")
    print(f"  NMSE: {nmse:.2e}")

    # Expected impact on end-to-end performance
    # Assume vocoder is ~40% of total time
    print(f"\n🚀 Expected End-to-End Impact:")
    baseline_rtf = 0.241  # Current RTF from STATUS.md
    vocoder_fraction = 0.40  # Estimate: vocoder is 40% of total time
    vocoder_speedup_fraction = (1.0 - 1.0/speedup) * vocoder_fraction
    new_rtf = baseline_rtf * (1.0 - vocoder_speedup_fraction)

    print(f"  Current RTF:  {baseline_rtf:.3f}")
    print(f"  Vocoder fraction: {vocoder_fraction*100:.0f}%")
    print(f"  Vocoder speedup: {speedup:.1f}x")
    print(f"  Expected new RTF: {new_rtf:.3f}")
    print(f"  Total improvement: {baseline_rtf/new_rtf:.2f}x faster")

    if new_rtf < 0.20:
        print(f"\n🎯 ✅ TARGET ACHIEVED: RTF < 0.20")
    else:
        print(f"\n⚠️  Target not reached (RTF >= 0.20)")

    print("\n" + "=" * 70)
    print("BENCHMARK COMPLETE")
    print("=" * 70)


if __name__ == "__main__":
    main()