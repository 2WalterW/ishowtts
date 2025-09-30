#!/usr/bin/env python3
"""Simple standalone test for TensorRT vocoder."""
import sys
import torch
import numpy as np
from pathlib import Path

# Add F5-TTS to path
sys.path.insert(0, str(Path(__file__).parent.parent / "third_party" / "F5-TTS" / "src"))

print("Testing TensorRT vocoder loading...")

# Test basic loading
from f5_tts.infer.tensorrt_vocoder import TensorRTVocoder, TRT_AVAILABLE

print(f"TensorRT available: {TRT_AVAILABLE}")

if not TRT_AVAILABLE:
    print("ERROR: TensorRT not available")
    sys.exit(1)

# Load engine
engine_path = "models/vocos_decoder.engine"
print(f"Loading engine: {engine_path}")

try:
    vocoder = TensorRTVocoder(engine_path, device="cuda")
    print("✅ TensorRT vocoder loaded successfully")

    # Test with dummy input
    print("\nTesting inference...")
    mel = torch.randn(1, 100, 256).cuda()
    print(f"Input shape: {mel.shape}")

    audio = vocoder.decode(mel)
    print(f"Output shape: {audio.shape}")
    print(f"✅ Inference successful!")

except Exception as e:
    print(f"❌ Error: {e}")
    import traceback
    traceback.print_exc()
    sys.exit(1)