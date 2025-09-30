#!/usr/bin/env python3
"""
Export Vocos vocoder to ONNX format for TensorRT conversion.

This script exports only the decoder portion (backbone + head) of the Vocos model,
which is what's used during F5-TTS inference.
"""
import sys
import torch
from pathlib import Path
from vocos import Vocos
import argparse

def export_vocoder_onnx(output_dir: str = "models", test_shapes: bool = True):
    """Export Vocos vocoder decoder to ONNX format.

    Args:
        output_dir: Directory to save the ONNX model
        test_shapes: Whether to test various input shapes
    """
    output_dir = Path(output_dir)
    output_dir.mkdir(exist_ok=True, parents=True)

    print("=" * 70)
    print("Vocos Vocoder ONNX Export")
    print("=" * 70)

    print("\n[1/5] Loading Vocos vocoder...")
    model = Vocos.from_pretrained("charactr/vocos-mel-24khz")
    model.eval()
    model.to("cuda")
    print(f"✅ Model loaded: {model.__class__.__name__}")
    print(f"   - Feature extractor: {model.feature_extractor.__class__.__name__}")
    print(f"   - Backbone: {model.backbone.__class__.__name__}")
    print(f"   - Head: {model.head.__class__.__name__}")

    # Create a wrapper that exports decoder UP TO STFT coefficients
    # ONNX doesn't support complex numbers, so we export real/imaginary separately
    # and do ISTFT in Python wrapper
    class VocosDecoderONNX(torch.nn.Module):
        def __init__(self, backbone, head):
            super().__init__()
            self.backbone = backbone
            self.head_linear = head.out

        def forward(self, mel_spectrogram):
            """
            Args:
                mel_spectrogram: (batch, mel_channels=100, time_frames)
            Returns:
                stft_real: (batch, freq_bins, time_frames) - Real part of STFT
                stft_imag: (batch, freq_bins, time_frames) - Imaginary part of STFT
            """
            # Backbone
            x = self.backbone(mel_spectrogram)

            # Head linear layer (up to STFT computation)
            x = self.head_linear(x).transpose(1, 2)
            mag, p = x.chunk(2, dim=1)
            mag = torch.exp(mag)
            mag = torch.clip(mag, max=1e2)

            # Compute real and imaginary parts of STFT
            # S = mag * exp(j*p) = mag * (cos(p) + j*sin(p))
            stft_real = mag * torch.cos(p)
            stft_imag = mag * torch.sin(p)

            return stft_real, stft_imag

    decoder = VocosDecoderONNX(model.backbone, model.head)
    decoder.eval()
    decoder.to("cuda")

    print("\n[2/5] Testing decoder forward pass...")
    # Test with typical F5-TTS output shape
    # F5-TTS generates mel spectrograms with 100 mel channels
    test_frames = [64, 128, 256, 512] if test_shapes else [256]

    for frames in test_frames:
        mel_input = torch.randn(1, 100, frames).cuda()
        with torch.no_grad():
            stft_real, stft_imag = decoder(mel_input)

        print(f"   Input: {tuple(mel_input.shape)}")
        print(f"   STFT Real: {tuple(stft_real.shape)}, Imag: {tuple(stft_imag.shape)}")

    # Test full pipeline (ONNX decoder + Python ISTFT)
    print("\n   Testing full pipeline (decoder + ISTFT)...")
    mel_input = torch.randn(1, 100, 256).cuda()
    with torch.no_grad():
        stft_real, stft_imag = decoder(mel_input)
        # Reconstruct complex STFT and apply ISTFT
        S = torch.complex(stft_real, stft_imag)
        audio_output = model.head.istft(S)
        print(f"   Final audio shape: {tuple(audio_output.shape)}")

    print("✅ Forward pass successful")

    print("\n[3/5] Exporting to ONNX...")
    # Use a typical size for ONNX export
    dummy_input = torch.randn(1, 100, 256).cuda()
    onnx_path = output_dir / "vocos_decoder.onnx"

    # Export with dynamic axes to support variable-length inputs
    # Output is STFT coefficients (real and imaginary parts separately)
    torch.onnx.export(
        decoder,
        dummy_input,
        str(onnx_path),
        input_names=["mel_spectrogram"],
        output_names=["stft_real", "stft_imag"],
        dynamic_axes={
            "mel_spectrogram": {0: "batch", 2: "time_frames"},
            "stft_real": {0: "batch", 2: "time_frames"},
            "stft_imag": {0: "batch", 2: "time_frames"}
        },
        opset_version=17,
        do_constant_folding=True,
        export_params=True,
        verbose=False
    )

    print(f"✅ ONNX export complete: {onnx_path}")
    print(f"   File size: {onnx_path.stat().st_size / 1024 / 1024:.2f} MB")

    print("\n[4/5] Validating ONNX model...")
    try:
        import onnx
        onnx_model = onnx.load(str(onnx_path))
        onnx.checker.check_model(onnx_model)
        print("✅ ONNX model validation passed")

        # Print model info
        print(f"   Inputs: {[i.name for i in onnx_model.graph.input]}")
        print(f"   Outputs: {[o.name for o in onnx_model.graph.output]}")
        print(f"   Nodes: {len(onnx_model.graph.node)}")
    except ImportError:
        print("⚠️  onnx package not installed, skipping validation")
    except Exception as e:
        print(f"❌ ONNX validation failed: {e}")
        return False

    print("\n[5/5] Testing ONNX inference (optional)...")
    try:
        import onnxruntime as ort

        # Test with CPU provider first
        providers = ['CPUExecutionProvider']
        sess = ort.InferenceSession(str(onnx_path), providers=providers)

        # Test inference
        test_input = torch.randn(1, 100, 256).cpu().numpy()
        onnx_outputs = sess.run(None, {"mel_spectrogram": test_input})
        onnx_real, onnx_imag = onnx_outputs[0], onnx_outputs[1]

        # Compare with PyTorch output
        with torch.no_grad():
            torch_real, torch_imag = decoder(torch.from_numpy(test_input).cuda())
            torch_real = torch_real.cpu().numpy()
            torch_imag = torch_imag.cpu().numpy()

        # Calculate difference for real and imaginary parts
        mse_real = ((onnx_real - torch_real) ** 2).mean()
        mse_imag = ((onnx_imag - torch_imag) ** 2).mean()
        mse_total = (mse_real + mse_imag) / 2

        print(f"✅ ONNX inference test passed")
        print(f"   MSE (Real): {mse_real:.2e}")
        print(f"   MSE (Imag): {mse_imag:.2e}")
        print(f"   MSE (Total): {mse_total:.2e}")

        if mse_total < 1e-4:
            print(f"   ✅ Output matches PyTorch (MSE < 1e-4)")
        else:
            print(f"   ⚠️  Output differs from PyTorch (MSE = {mse_total:.2e})")

    except ImportError:
        print("⚠️  onnxruntime not installed, skipping ONNX inference test")
    except Exception as e:
        print(f"⚠️  ONNX inference test failed: {e}")

    print("\n" + "=" * 70)
    print("EXPORT COMPLETE")
    print("=" * 70)
    print(f"\nNext steps:")
    print(f"1. Convert to TensorRT:")
    print(f"   bash scripts/convert_vocoder_tensorrt.sh")
    print(f"2. Benchmark performance:")
    print(f"   python scripts/benchmark_vocoder.py")

    return True

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Export Vocos vocoder to ONNX")
    parser.add_argument(
        "--output-dir",
        type=str,
        default="models",
        help="Directory to save ONNX model (default: models)"
    )
    parser.add_argument(
        "--no-test-shapes",
        action="store_true",
        help="Skip testing multiple input shapes"
    )

    args = parser.parse_args()

    success = export_vocoder_onnx(
        output_dir=args.output_dir,
        test_shapes=not args.no_test_shapes
    )

    sys.exit(0 if success else 1)