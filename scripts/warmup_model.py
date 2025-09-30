#!/usr/bin/env python3
"""
Model Warmup Script
Pre-compiles F5-TTS model using torch.compile() to avoid first-run latency
"""

import sys
import time
import argparse
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


def warmup_model(ref_audio: str, ref_text: str, nfe_steps: int = 16):
    """
    Warmup the F5-TTS model by running inference once.
    This triggers torch.compile() JIT compilation if enabled.
    """

    print("=" * 70)
    print("F5-TTS Model Warmup")
    print("=" * 70)

    # System info
    print("\n[System Info]")
    print(f"PyTorch: {torch.__version__}")
    print(f"CUDA available: {torch.cuda.is_available()}")
    if torch.cuda.is_available():
        print(f"CUDA device: {torch.cuda.get_device_name(0)}")
        print(f"CUDA version: {torch.version.cuda}")
    print(f"torch.compile available: {hasattr(torch, 'compile')}")

    # Initialize model
    print("\n[Step 1/3] Initializing Model...")
    start = time.perf_counter()
    model = F5TTS(model_type="F5-TTS")
    init_time = time.perf_counter() - start
    print(f"✓ Model loaded in {init_time:.2f}s")

    # First warmup inference (triggers compilation)
    print("\n[Step 2/3] First Warmup Inference (JIT compilation)...")
    print("⚠ This will take 30-60 seconds due to torch.compile() overhead")
    start = time.perf_counter()

    try:
        wav, sr, _ = model.infer(
            ref_file=ref_audio,
            ref_text=ref_text,
            gen_text="你好，这是模型预热测试。",
            nfe_step=nfe_steps,
            show_info=print
        )
        first_time = time.perf_counter() - start
        audio_duration = len(wav) / sr
        rtf_first = first_time / audio_duration

        print(f"✓ First inference completed in {first_time:.2f}s")
        print(f"  Audio duration: {audio_duration:.2f}s")
        print(f"  RTF: {rtf_first:.3f}")

    except Exception as e:
        print(f"✗ First inference failed: {e}")
        return False

    # Second warmup inference (should be much faster)
    print("\n[Step 3/3] Second Warmup Inference (using compiled model)...")
    start = time.perf_counter()

    try:
        wav, sr, _ = model.infer(
            ref_file=ref_audio,
            ref_text=ref_text,
            gen_text="测试编译后的模型性能。",
            nfe_step=nfe_steps,
            show_info=print
        )
        second_time = time.perf_counter() - start
        audio_duration = len(wav) / sr
        rtf_second = second_time / audio_duration

        print(f"✓ Second inference completed in {second_time:.2f}s")
        print(f"  Audio duration: {audio_duration:.2f}s")
        print(f"  RTF: {rtf_second:.3f}")

    except Exception as e:
        print(f"✗ Second inference failed: {e}")
        return False

    # Print summary
    print("\n" + "=" * 70)
    print("WARMUP SUMMARY")
    print("=" * 70)

    speedup = first_time / second_time if second_time > 0 else 0
    print(f"\n  Model initialization: {init_time:.2f}s")
    print(f"  First inference (with compilation): {first_time:.2f}s (RTF: {rtf_first:.3f})")
    print(f"  Second inference (compiled): {second_time:.2f}s (RTF: {rtf_second:.3f})")
    print(f"  Speedup after compilation: {speedup:.2f}x")

    if rtf_second < 0.3:
        print(f"\n  ✓ Excellent! RTF < 0.3 (target achieved)")
    elif rtf_second < 0.5:
        print(f"\n  ✓ Good! RTF < 0.5 (suitable for streaming)")
    elif rtf_second < 1.0:
        print(f"\n  ○ Acceptable. RTF < 1.0")
    else:
        print(f"\n  ✗ Warning: RTF > 1.0 (slower than real-time)")

    # GPU memory info
    if torch.cuda.is_available():
        print(f"\n[GPU Memory]")
        print(f"  Allocated: {torch.cuda.memory_allocated() / 1024**3:.2f} GB")
        print(f"  Reserved: {torch.cuda.memory_reserved() / 1024**3:.2f} GB")

    print("\n✓ Model warmup completed successfully!")
    print("The model is now ready for fast inference.")

    return True


def main():
    parser = argparse.ArgumentParser(description="Warmup F5-TTS model for fast inference")
    parser.add_argument("--ref-audio", required=True, help="Reference audio file path")
    parser.add_argument("--ref-text", required=True, help="Reference audio transcript")
    parser.add_argument("--nfe-steps", type=int, default=16, help="NFE steps (default: 16)")

    args = parser.parse_args()

    # Validate inputs
    if not Path(args.ref_audio).exists():
        print(f"Error: Reference audio not found: {args.ref_audio}")
        sys.exit(1)

    # Run warmup
    success = warmup_model(args.ref_audio, args.ref_text, args.nfe_steps)
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()