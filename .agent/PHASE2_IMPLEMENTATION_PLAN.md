# Phase 2 Implementation Plan - iShowTTS Optimization
**Date**: 2025-09-30
**Current Status**: Phase 1 Complete ✅ (RTF=0.241)
**Next Target**: RTF < 0.20 with TensorRT vocoder

---

## 📊 Current Performance Baseline

| Metric | Value | Status |
|--------|-------|--------|
| **RTF (Mean)** | 0.241 | ✅ Target achieved |
| **RTF (Best)** | 0.239 | ✅ |
| **Speedup** | 4.14x real-time | ✅ |
| **Variance** | <2% | ✅ Excellent |

---

## 🎯 Phase 2 Goals (Priority Order)

### 1. TensorRT Vocoder Integration (HIGHEST PRIORITY)
**Timeline**: 2-3 weeks
**Expected Impact**: RTF 0.241 → 0.15-0.20 (35-40% reduction)
**Effort**: Medium
**Risk**: Medium

### 2. End-to-End & Load Testing
**Timeline**: 1 week
**Expected Impact**: Stability & confidence
**Effort**: Low
**Risk**: Low

### 3. Batch Processing
**Timeline**: 1-2 weeks
**Expected Impact**: 2-3x throughput for queued requests
**Effort**: Medium
**Risk**: Medium

### 4. INT8 Quantization (OPTIONAL - LOW PRIORITY)
**Timeline**: 2-3 weeks
**Expected Impact**: Additional 20-30% if quality maintained
**Effort**: High
**Risk**: High (quality degradation)

---

## 📋 Task 1: TensorRT Vocoder (Detailed Plan)

### Phase 1.1: Research & Preparation (2-3 days)
- [ ] Study Vocos architecture and ONNX export requirements
- [ ] Check TensorRT version on Jetson Orin (`dpkg -l | grep tensorrt`)
- [ ] Review similar TensorRT integrations (BigVGAN, HiFi-GAN)
- [ ] Identify potential export issues (dynamic shapes, custom ops)

### Phase 1.2: ONNX Export (3-5 days)
**Script**: `scripts/export_vocoder_onnx.py`

```python
#!/usr/bin/env python3
"""
Export Vocos vocoder to ONNX format for TensorRT conversion.
"""
import sys
import torch
from pathlib import Path
from vocos import Vocos

def export_vocoder_onnx():
    print("Loading Vocos vocoder...")
    model = Vocos.from_pretrained("charactr/vocos-mel-24khz")
    model.eval()
    model.to("cuda")
    
    # Dummy input - match F5-TTS output shape
    # Shape: (batch=1, mel_channels=100, time_frames=256)
    dummy_input = torch.randn(1, 100, 256).cuda()
    
    print("Testing forward pass...")
    with torch.no_grad():
        output = model.decode(dummy_input)
    print(f"Input shape: {dummy_input.shape}")
    print(f"Output shape: {output.shape}")
    
    print("Exporting to ONNX...")
    torch.onnx.export(
        model,
        dummy_input,
        "models/vocos_vocoder.onnx",
        input_names=["mel_spectrogram"],
        output_names=["audio_waveform"],
        dynamic_axes={
            "mel_spectrogram": {0: "batch", 2: "time"},
            "audio_waveform": {0: "batch", 1: "samples"}
        },
        opset_version=17,
        do_constant_folding=True,
        verbose=True
    )
    
    print("✅ ONNX export complete: models/vocos_vocoder.onnx")
    
    # Verify ONNX model
    import onnx
    onnx_model = onnx.load("models/vocos_vocoder.onnx")
    onnx.checker.check_model(onnx_model)
    print("✅ ONNX model validation passed")

if __name__ == "__main__":
    export_vocoder_onnx()
```

**Tasks**:
- [ ] Create script
- [ ] Test ONNX export with various input sizes
- [ ] Validate ONNX model (onnx.checker)
- [ ] Test ONNX inference with onnxruntime (optional)

**Potential Issues & Solutions**:
- **Issue**: Custom ops not supported by ONNX
  - **Solution**: Replace with standard torch ops or use torch.onnx.register_custom_op_symbolic
- **Issue**: Dynamic shapes causing export errors
  - **Solution**: Use static shapes or torch.jit.trace instead of export

### Phase 1.3: TensorRT Conversion (2-3 days)
**Script**: `scripts/convert_vocoder_tensorrt.sh`

```bash
#!/bin/bash
# Convert ONNX vocoder to TensorRT engine with FP16

set -e

ONNX_PATH="models/vocos_vocoder.onnx"
ENGINE_PATH="models/vocos_vocoder.engine"

echo "Converting ONNX to TensorRT..."

/usr/src/tensorrt/bin/trtexec \
    --onnx="$ONNX_PATH" \
    --saveEngine="$ENGINE_PATH" \
    --fp16 \
    --workspace=4096 \
    --minShapes=mel_spectrogram:1x100x32 \
    --optShapes=mel_spectrogram:1x100x256 \
    --maxShapes=mel_spectrogram:1x100x512 \
    --buildOnly \
    --verbose \
    --dumpProfile \
    --exportProfile=models/vocoder_profile.json

echo "✅ TensorRT conversion complete: $ENGINE_PATH"

# Test inference speed
echo "Testing inference speed..."
/usr/src/tensorrt/bin/trtexec \
    --loadEngine="$ENGINE_PATH" \
    --shapes=mel_spectrogram:1x100x256 \
    --iterations=100 \
    --avgRuns=10

echo "✅ Benchmark complete"
```

**Tasks**:
- [ ] Create conversion script
- [ ] Test with various shape configurations
- [ ] Benchmark TensorRT engine vs PyTorch
- [ ] Verify output correctness (compare with PyTorch)

**Target Performance**:
- Current vocoder: ~30-40% of total time (~0.07-0.10s)
- TensorRT vocoder: <0.03s (2-3x faster)
- Expected total RTF: 0.15-0.20

### Phase 1.4: Python Integration (5-7 days)
**File**: `third_party/F5-TTS/src/f5_tts/infer/tensorrt_vocoder.py`

```python
"""
TensorRT vocoder wrapper for F5-TTS inference.
"""
import numpy as np
import torch
import tensorrt as trt
import pycuda.driver as cuda
import pycuda.autoinit

class TensorRTVocoder:
    """TensorRT-accelerated vocoder for real-time audio synthesis."""
    
    def __init__(self, engine_path: str):
        self.logger = trt.Logger(trt.Logger.WARNING)
        self.runtime = trt.Runtime(self.logger)
        
        # Load engine
        with open(engine_path, 'rb') as f:
            self.engine = self.runtime.deserialize_cuda_engine(f.read())
        
        self.context = self.engine.create_execution_context()
        
        # Allocate buffers
        self.inputs = []
        self.outputs = []
        self.bindings = []
        self.stream = cuda.Stream()
        
        for i in range(self.engine.num_io_tensors):
            tensor_name = self.engine.get_tensor_name(i)
            size = trt.volume(self.engine.get_tensor_shape(tensor_name))
            dtype = trt.nptype(self.engine.get_tensor_dtype(tensor_name))
            
            # Allocate host and device buffers
            host_mem = cuda.pagelocked_empty(size, dtype)
            device_mem = cuda.mem_alloc(host_mem.nbytes)
            
            self.bindings.append(int(device_mem))
            
            if self.engine.get_tensor_mode(tensor_name) == trt.TensorIOMode.INPUT:
                self.inputs.append({'host': host_mem, 'device': device_mem})
            else:
                self.outputs.append({'host': host_mem, 'device': device_mem})
    
    def decode(self, mel_spectrogram: torch.Tensor) -> torch.Tensor:
        """
        Decode mel spectrogram to audio waveform.
        
        Args:
            mel_spectrogram: (batch, mel_channels, time_frames)
        
        Returns:
            audio_waveform: (batch, samples)
        """
        # Convert to numpy and copy to input buffer
        mel_np = mel_spectrogram.cpu().numpy().astype(np.float32)
        np.copyto(self.inputs[0]['host'], mel_np.ravel())
        
        # Transfer input to GPU
        cuda.memcpy_htod_async(
            self.inputs[0]['device'],
            self.inputs[0]['host'],
            self.stream
        )
        
        # Run inference
        self.context.execute_async_v3(stream_handle=self.stream.handle)
        
        # Transfer output to CPU
        cuda.memcpy_dtoh_async(
            self.outputs[0]['host'],
            self.outputs[0]['device'],
            self.stream
        )
        
        # Synchronize
        self.stream.synchronize()
        
        # Convert back to torch tensor
        audio_np = self.outputs[0]['host'].reshape(mel_np.shape[0], -1)
        return torch.from_numpy(audio_np).to(mel_spectrogram.device)
    
    def __del__(self):
        """Cleanup resources."""
        # Free CUDA memory
        for inp in self.inputs + self.outputs:
            inp['device'].free()
```

**Integration into utils_infer.py**:
```python
# In load_vocoder() function
if vocoder_name == "vocos":
    if local_path and local_path.endswith('.engine'):
        # Use TensorRT vocoder
        from f5_tts.infer.tensorrt_vocoder import TensorRTVocoder
        vocoder = TensorRTVocoder(local_path)
    else:
        # Use PyTorch vocoder (existing code)
        vocoder = Vocos.from_hparams(config_path)
        # ... existing code ...
```

**Tasks**:
- [ ] Implement TensorRTVocoder class
- [ ] Add proper error handling
- [ ] Test with various input sizes
- [ ] Integrate into F5-TTS inference pipeline
- [ ] Update configuration to support .engine files

### Phase 1.5: Testing & Validation (3-5 days)
**Script**: `scripts/benchmark_vocoder.py`

```python
#!/usr/bin/env python3
"""
Benchmark PyTorch vs TensorRT vocoder performance.
"""
import time
import torch
import numpy as np
from vocos import Vocos
from f5_tts.infer.tensorrt_vocoder import TensorRTVocoder

def benchmark_vocoder():
    # Test configurations
    batch_sizes = [1]
    time_frames = [64, 128, 256, 512]
    n_runs = 100
    
    # Load models
    pytorch_vocoder = Vocos.from_pretrained("charactr/vocos-mel-24khz").cuda().eval()
    tensorrt_vocoder = TensorRTVocoder("models/vocos_vocoder.engine")
    
    results = []
    
    for batch in batch_sizes:
        for frames in time_frames:
            # Create dummy input
            mel = torch.randn(batch, 100, frames).cuda()
            
            # Warmup
            for _ in range(10):
                _ = pytorch_vocoder.decode(mel)
                _ = tensorrt_vocoder.decode(mel)
            
            # Benchmark PyTorch
            torch.cuda.synchronize()
            start = time.perf_counter()
            for _ in range(n_runs):
                _ = pytorch_vocoder.decode(mel)
            torch.cuda.synchronize()
            pytorch_time = (time.perf_counter() - start) / n_runs
            
            # Benchmark TensorRT
            torch.cuda.synchronize()
            start = time.perf_counter()
            for _ in range(n_runs):
                _ = tensorrt_vocoder.decode(mel)
            torch.cuda.synchronize()
            tensorrt_time = (time.perf_counter() - start) / n_runs
            
            speedup = pytorch_time / tensorrt_time
            
            results.append({
                'batch': batch,
                'frames': frames,
                'pytorch_ms': pytorch_time * 1000,
                'tensorrt_ms': tensorrt_time * 1000,
                'speedup': speedup
            })
            
            print(f"Batch={batch}, Frames={frames}: "
                  f"PyTorch={pytorch_time*1000:.2f}ms, "
                  f"TensorRT={tensorrt_time*1000:.2f}ms, "
                  f"Speedup={speedup:.2f}x")
    
    # Summary
    avg_speedup = np.mean([r['speedup'] for r in results])
    print(f"\nAverage speedup: {avg_speedup:.2f}x")
    
    return results

if __name__ == "__main__":
    benchmark_vocoder()
```

**Tasks**:
- [ ] Create benchmark script
- [ ] Compare PyTorch vs TensorRT performance
- [ ] Verify output quality (MSE, perceptual)
- [ ] Test with real F5-TTS outputs
- [ ] Run E2E performance test with TensorRT

**Success Criteria**:
- TensorRT vocoder ≥2x faster than PyTorch
- Output MSE < 1e-5 vs PyTorch
- E2E RTF < 0.20 achieved
- No quality degradation (subjective listening)

### Phase 1.6: Documentation & Deployment (1-2 days)
- [ ] Update README.md with TensorRT instructions
- [ ] Update config/ishowtts.toml with example
- [ ] Update MAINTENANCE_PLAN with TensorRT notes
- [ ] Add troubleshooting guide for TensorRT issues
- [ ] Create installation script for dependencies

---

## 📋 Task 2: End-to-End Testing (After TensorRT)

### Test Suite Structure
```
tests/
├── test_e2e_basic.py           # Basic E2E flow
├── test_e2e_danmaku.py         # Danmaku integration
├── test_load_concurrent.py     # Concurrent requests
├── test_load_queue.py          # Queue stress test
└── test_performance_regression.py  # Performance monitoring
```

### test_e2e_basic.py
```python
#!/usr/bin/env python3
"""End-to-end test for basic TTS functionality."""
import requests
import time

def test_tts_api():
    """Test /api/tts endpoint."""
    response = requests.post(
        "http://localhost:8080/api/tts",
        json={
            "text": "Hello world",
            "voice_id": "ishow",
            "nfe_step": 8
        }
    )
    assert response.status_code == 200
    assert len(response.content) > 0
    print("✅ TTS API test passed")

def test_performance():
    """Test performance meets target."""
    start = time.time()
    response = requests.post(
        "http://localhost:8080/api/tts",
        json={
            "text": "This is a performance test",
            "voice_id": "ishow",
            "nfe_step": 8
        }
    )
    elapsed = time.time() - start
    
    # Assuming ~5 seconds of audio
    rtf = elapsed / 5.0
    assert rtf < 0.25, f"RTF too high: {rtf}"
    print(f"✅ Performance test passed (RTF={rtf:.3f})")

if __name__ == "__main__":
    test_tts_api()
    test_performance()
```

**Tasks**:
- [ ] Create E2E test suite
- [ ] Test all API endpoints
- [ ] Test error handling
- [ ] Test with various text inputs
- [ ] Add to CI/CD pipeline

---

## 📋 Task 3: Batch Processing (Optional, After Testing)

### Implementation Approach
1. **Queue batching** in Rust backend
2. **Batch inference** in Python
3. **Configure batch size and timeout**

### Key Changes
**File**: `crates/backend/src/tts_service.rs`
- Add batch accumulator (collect requests for 50-100ms)
- Send batch to Python when full or timeout
- Distribute results back to requesters

**File**: `third_party/F5-TTS/src/f5_tts/infer/utils_infer.py`
- Update `infer_batch_process` to handle true batch inference
- Process multiple gen_texts with same reference audio
- Return list of audio outputs

**Expected Impact**:
- Single request: RTF=0.20 (no change)
- 4 concurrent requests: Total time ~0.5s (vs 0.8s sequential)
- Throughput: 2-3x improvement

**Tasks**:
- [ ] Design batch API between Rust and Python
- [ ] Implement batch accumulation in Rust
- [ ] Update Python to support batch inference
- [ ] Test with concurrent requests
- [ ] Measure throughput improvement

---

## 📊 Phase 2 Timeline

| Task | Duration | Dependencies | Status |
|------|----------|-------------|---------|
| TensorRT Research | 2-3 days | None | ⏳ Pending |
| ONNX Export | 3-5 days | Research | ⏳ Pending |
| TensorRT Conversion | 2-3 days | ONNX Export | ⏳ Pending |
| Python Integration | 5-7 days | Conversion | ⏳ Pending |
| Testing & Validation | 3-5 days | Integration | ⏳ Pending |
| Documentation | 1-2 days | Validation | ⏳ Pending |
| E2E Testing | 5-7 days | TensorRT Complete | ⏳ Pending |
| Batch Processing | 7-10 days | Testing | ⏳ Pending |

**Total Estimated Time**: 4-6 weeks for TensorRT + Testing

---

## 🎯 Success Metrics

### Phase 2.1 (TensorRT Vocoder)
- ✅ RTF < 0.20 achieved
- ✅ No quality degradation (subjective)
- ✅ Output MSE < 1e-5 vs PyTorch
- ✅ ≥2x vocoder speedup
- ✅ Stable over 100+ runs

### Phase 2.2 (Testing)
- ✅ E2E tests passing
- ✅ Load tests passing (10+ concurrent)
- ✅ No performance regression detected
- ✅ CI/CD pipeline setup

### Phase 2.3 (Batch Processing)
- ✅ 2-3x throughput improvement
- ✅ Single request latency unchanged
- ✅ Stable under stress test

---

## 📚 Resources & References

### TensorRT Documentation
- https://docs.nvidia.com/deeplearning/tensorrt/developer-guide/
- https://docs.nvidia.com/deeplearning/tensorrt/quick-start-guide/

### Similar Implementations
- BigVGAN TensorRT: https://github.com/NVIDIA/BigVGAN
- HiFi-GAN TensorRT: https://github.com/jik876/hifi-gan

### Tools
- ONNX Runtime: https://onnxruntime.ai/
- Polygraphy: https://github.com/NVIDIA/TensorRT/tree/main/tools/Polygraphy

---

**Last Updated**: 2025-09-30 11:25 (UTC+8)
**Next Review**: After TensorRT implementation
