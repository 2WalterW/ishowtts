#!/bin/bash
# Convert Vocos ONNX vocoder to TensorRT engine with FP16 optimization
# This script converts the ONNX model exported by export_vocoder_onnx.py to a TensorRT engine

set -e

# Configuration
ONNX_PATH="${ONNX_PATH:-models/vocos_decoder.onnx}"
ENGINE_PATH="${ENGINE_PATH:-models/vocos_decoder.engine}"
WORKSPACE_MB="${WORKSPACE_MB:-4096}"

# TensorRT executable
TRTEXEC="/usr/src/tensorrt/bin/trtexec"

# Check if ONNX model exists
if [ ! -f "$ONNX_PATH" ]; then
    echo "❌ ONNX model not found: $ONNX_PATH"
    echo "Please run: python scripts/export_vocoder_onnx.py"
    exit 1
fi

echo "======================================================================"
echo "TensorRT Vocoder Conversion"
echo "======================================================================"
echo ""
echo "Input ONNX:  $ONNX_PATH"
echo "Output Engine: $ENGINE_PATH"
echo "Workspace:   ${WORKSPACE_MB} MB"
echo ""

# Print ONNX model info
echo "[1/3] ONNX Model Information"
echo "----------------------------------------------------------------------"
ls -lh "$ONNX_PATH"
echo ""

# Define dynamic shapes for TTS workload
# Typical F5-TTS output: 64-512 time frames
# - minShapes: Shortest possible audio (~2.5s)
# - optShapes: Typical audio length (~10s)
# - maxShapes: Longest audio we support (~20s)
MIN_FRAMES=64
OPT_FRAMES=256
MAX_FRAMES=512

echo "[2/3] Converting ONNX to TensorRT Engine"
echo "----------------------------------------------------------------------"
echo "Building TensorRT engine with FP16 optimization..."
echo "This may take 5-10 minutes..."
echo ""

# Build TensorRT engine
# TensorRT 10.3 uses --memPoolSize instead of --workspace
# and doesn't have --buildOnly option
"$TRTEXEC" \
    --onnx="$ONNX_PATH" \
    --saveEngine="$ENGINE_PATH" \
    --fp16 \
    --memPoolSize=workspace:${WORKSPACE_MB}M \
    --minShapes=mel_spectrogram:1x100x${MIN_FRAMES} \
    --optShapes=mel_spectrogram:1x100x${OPT_FRAMES} \
    --maxShapes=mel_spectrogram:1x100x${MAX_FRAMES} \
    --verbose \
    --dumpLayerInfo \
    --exportLayerInfo=models/vocoder_layer_info.json \
    --dumpProfile \
    --exportProfile=models/vocoder_profile.json \
    --noDataTransfers \
    --warmUp=0 \
    --iterations=0 \
    2>&1 | tee models/vocoder_build.log

echo ""
echo "✅ TensorRT engine built successfully"
echo ""

# Print engine info
echo "[3/3] TensorRT Engine Information"
echo "----------------------------------------------------------------------"
if [ -f "$ENGINE_PATH" ]; then
    ls -lh "$ENGINE_PATH"
    echo ""
    echo "✅ Engine file created: $ENGINE_PATH"
else
    echo "❌ Engine file not found after build"
    exit 1
fi

echo ""
echo "======================================================================"
echo "CONVERSION COMPLETE"
echo "======================================================================"
echo ""
echo "Build log saved to: models/vocoder_build.log"
echo "Layer info saved to: models/vocoder_layer_info.json"
echo "Profile saved to: models/vocoder_profile.json"
echo ""
echo "Next steps:"
echo "1. Benchmark TensorRT engine:"
echo "   bash scripts/benchmark_vocoder_trt.sh"
echo "2. Test end-to-end inference:"
echo "   python scripts/test_trt_vocoder.py"
echo "3. Integrate into F5-TTS pipeline:"
echo "   Update config/ishowtts.toml to use TensorRT vocoder"
echo ""