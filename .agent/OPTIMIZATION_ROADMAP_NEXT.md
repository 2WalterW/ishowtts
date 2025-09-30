# iShowTTS Optimization Roadmap - Next Phase
**Date**: 2025-09-30
**Current Status**: RTF=0.278 (mean), target achieved ✅

---

## 🎯 Current State Summary

### Achievements ✅
- **Performance**: RTF < 0.3 achieved (mean: 0.278, best: 0.274)
- **Consistency**: ±1.5% variance with GPU locked
- **Speedup**: 3.59x real-time (3.65x best)
- **All optimizations applied and validated**

### Critical Configuration ⚠️
```bash
# Must run after reboot for consistent performance
sudo jetson_clocks
sudo nvpmodel -m 0
```

---

## 🚀 Phase 2: Advanced Optimizations

### Priority 1: TensorRT Vocoder (Highest Impact)

**Expected Speedup**: 2-3x on vocoder (vocoder is ~30-40% of total time)
**Total RTF Target**: 0.15-0.20 (from current 0.278)

#### Implementation Steps

1. **Export Vocos Vocoder to ONNX**
   ```python
   # Script: scripts/export_vocoder_onnx.py
   import torch
   from vocos import Vocos

   model = Vocos.from_pretrained("charactr/vocos-mel-24khz")
   model.eval()

   # Dummy input: (batch, channels, time)
   dummy_input = torch.randn(1, 100, 256)

   torch.onnx.export(
       model,
       dummy_input,
       "vocos_vocoder.onnx",
       input_names=["mel_spectrogram"],
       output_names=["audio"],
       dynamic_axes={
           "mel_spectrogram": {0: "batch", 2: "time"},
           "audio": {0: "batch", 1: "samples"}
       },
       opset_version=17
   )
   ```

2. **Convert ONNX to TensorRT**
   ```bash
   # On Jetson Orin
   /usr/src/tensorrt/bin/trtexec \
       --onnx=vocos_vocoder.onnx \
       --saveEngine=vocos_vocoder.engine \
       --fp16 \
       --workspace=4096 \
       --minShapes=mel_spectrogram:1x100x32 \
       --optShapes=mel_spectrogram:1x100x256 \
       --maxShapes=mel_spectrogram:1x100x512 \
       --verbose
   ```

3. **Update Configuration**
   ```toml
   [f5]
   vocoder_local_path = "/opt/models/vocos_vocoder.engine"
   ```

4. **Python Integration**
   ```python
   # In f5_tts/infer/utils_infer.py
   import tensorrt as trt
   import pycuda.driver as cuda
   import pycuda.autoinit

   class TensorRTVocoder:
       def __init__(self, engine_path):
           # Load TensorRT engine
           # Allocate buffers
           # Create execution context
           pass

       def decode(self, mel):
           # Run inference
           # Return audio
           pass
   ```

**Files to Create**:
- `scripts/export_vocoder_onnx.py` - Export script
- `scripts/test_tensorrt_vocoder.py` - Validation script
- `scripts/benchmark_vocoder.py` - Compare PyTorch vs TensorRT

**Risks**:
- ONNX export compatibility issues
- TensorRT version compatibility (Jetson vs desktop)
- Dynamic shape handling in TensorRT

**Testing**:
```bash
# 1. Export and convert
python scripts/export_vocoder_onnx.py
/usr/src/tensorrt/bin/trtexec --onnx=vocos_vocoder.onnx --saveEngine=vocos_vocoder.engine --fp16

# 2. Benchmark
python scripts/benchmark_vocoder.py

# 3. E2E test
python scripts/test_max_autotune.py  # Should show RTF ~0.15-0.20
```

---

### Priority 2: INT8 Quantization (Medium Impact)

**Expected Speedup**: 1.5-2x
**Total RTF Target**: 0.18-0.23 (combined with TensorRT vocoder: 0.10-0.15)

#### Implementation Steps

1. **Collect Calibration Dataset**
   ```python
   # Script: scripts/collect_calibration_data.py
   # Generate 100-500 diverse samples for calibration
   ```

2. **Apply Post-Training Quantization**
   ```python
   # Script: scripts/quantize_model.py
   import torch
   from torch.ao.quantization import quantize_dynamic

   # Quantize model to INT8
   quantized_model = quantize_dynamic(
       model,
       {torch.nn.Linear, torch.nn.Conv1d},
       dtype=torch.qint8
   )
   ```

3. **Validation**
   - Quality checks (MOS, similarity)
   - Performance benchmarks
   - A/B testing

**Risks**:
- Quality degradation
- torch.compile compatibility with quantized models
- May need quantization-aware training (QAT) for better results

---

### Priority 3: Batch Processing (Throughput Optimization)

**Expected**: 2-3x throughput for multiple requests
**Use Case**: Queue processing, multiple danmaku messages

#### Implementation Steps

1. **Python Batch Inference**
   ```python
   # In f5_tts/infer/utils_infer.py
   def infer_batch(
       ref_audios: List[Tensor],
       ref_texts: List[str],
       gen_texts: List[str],
       ...
   ):
       # Process multiple requests in single forward pass
       # Pad to same length
       # Return list of audio
       pass
   ```

2. **Rust Queue Batching**
   ```rust
   // In crates/tts-engine/src/lib.rs
   // Collect requests for 10-50ms, then batch
   struct BatchQueue {
       pending: Vec<TtsRequest>,
       deadline: Instant,
   }
   ```

3. **Configuration**
   ```toml
   [f5]
   batch_size = 4  # Process up to 4 requests together
   batch_timeout_ms = 50  # Wait up to 50ms to collect batch
   ```

**Benefits**:
- Better GPU utilization
- Higher overall throughput
- Amortized model overhead

**Risks**:
- Increased latency for early requests in batch
- Complexity in request management
- Need padding/masking for variable lengths

---

### Priority 4: CUDA Graphs (Advanced)

**Expected Speedup**: 10-20% (small but consistent)
**Requirement**: Static input shapes

#### Implementation Steps

1. **Capture CUDA Graph**
   ```python
   # For fixed-length inputs only
   static_input = torch.randn(1, 256, 100).cuda()

   # Warmup
   for _ in range(3):
       output = model(static_input)

   # Capture
   graph = torch.cuda.CUDAGraph()
   with torch.cuda.graph(graph):
       output = model(static_input)

   # Replay (much faster)
   graph.replay()
   ```

2. **Integration Challenges**
   - Need bucketing for different lengths
   - Complex with dynamic shapes
   - torch.compile may already optimize this

**Recommendation**: Defer until after TensorRT vocoder

---

### Priority 5: NFE Reduction Experiment

**Current**: NFE=8
**Experiment**: NFE=6 or NFE=4

**Expected**: RTF ~0.20-0.25 (NFE=6) or ~0.15-0.20 (NFE=4)
**Risk**: Quality degradation

#### Testing Protocol

```bash
# Test NFE=6
echo "default_nfe_step = 6" >> config/ishowtts.toml
python scripts/test_max_autotune.py

# Test NFE=4
echo "default_nfe_step = 4" >> config/ishowtts.toml
python scripts/test_max_autotune.py

# Quality evaluation
# Listen to samples, run objective metrics (MOS, similarity)
```

**Caution**: May significantly degrade quality. Only worth it if TensorRT is not feasible.

---

## 🔧 Infrastructure Improvements

### Testing Infrastructure

1. **E2E Integration Tests**
   ```bash
   # Create: tests/test_e2e.py
   # - Test full pipeline: request → synthesis → response
   # - Test error handling
   # - Test voice override functionality
   ```

2. **Load Testing**
   ```bash
   # Create: tests/test_load.py
   # - Simulate multiple concurrent requests
   # - Measure throughput, latency distribution
   # - Identify bottlenecks
   ```

3. **Performance Regression Tests**
   ```bash
   # Create: tests/test_performance_regression.py
   # - Run on CI/CD
   # - Alert if RTF > 0.35
   # - Track performance over time
   ```

### Monitoring & Profiling

1. **Add Metrics Endpoint**
   ```rust
   // In backend
   GET /api/metrics
   {
     "rtf_mean": 0.278,
     "rtf_p95": 0.285,
     "requests_total": 1234,
     "errors_total": 5
   }
   ```

2. **Continuous Profiling**
   ```bash
   # Add to systemd service or startup script
   nsys profile --sample=cpu --trace=cuda,nvtx \
       --output=/var/log/ishowtts/profile_%p.qdrep \
       --capture-range=cudaProfilerApi \
       cargo run --release -p ishowtts-backend
   ```

3. **GPU Utilization Dashboard**
   ```bash
   # Log GPU stats every 5s
   while true; do
       nvidia-smi --query-gpu=utilization.gpu,memory.used,temperature.gpu \
           --format=csv,noheader,nounits >> /var/log/ishowtts/gpu_stats.log
       sleep 5
   done
   ```

---

## 📊 Expected Performance Timeline

| Phase | Optimization | Expected RTF | Cumulative Speedup |
|-------|--------------|--------------|-------------------|
| **Current** | All Phase 1 optimizations | **0.278** | **1.0x baseline** |
| Phase 2.1 | + TensorRT Vocoder | 0.15-0.20 | 1.4-1.9x |
| Phase 2.2 | + INT8 Quantization | 0.10-0.15 | 1.9-2.8x |
| Phase 2.3 | + CUDA Graphs | 0.09-0.13 | 2.1-3.1x |
| Phase 2.4 | + NFE=6 (optional) | 0.06-0.10 | 2.8-4.6x |

**Conservative Target**: RTF < 0.15 (6.6x real-time)
**Aggressive Target**: RTF < 0.10 (10x real-time)

---

## 🎯 Recommended Next Steps

### Immediate (This Week)
1. ✅ Lock GPU frequencies (DONE - added to docs)
2. ✅ Validate current performance (DONE - RTF=0.278)
3. **Add startup script for jetson_clocks**
   ```bash
   # Create: scripts/setup_performance_mode.sh
   #!/bin/bash
   sudo jetson_clocks
   sudo nvpmodel -m 0
   echo "Performance mode enabled"
   ```

### Short-term (Next 2 Weeks)
4. **Implement E2E tests** - Ensure stability before advanced opts
5. **Add metrics endpoint** - Track performance in production
6. **Begin TensorRT vocoder work** - Highest impact optimization

### Medium-term (Next Month)
7. **Complete TensorRT integration** - Target RTF < 0.20
8. **Implement batch processing** - Improve throughput
9. **Setup continuous profiling** - Monitor for regressions

### Long-term (Next Quarter)
10. **Explore INT8 quantization** - If more speed needed
11. **Consider model distillation** - Train smaller model
12. **Investigate streaming inference** - Lower perceived latency

---

## 🚫 What NOT to Do

1. **Don't optimize prematurely**
   - Current performance meets requirements
   - Focus on stability and testing first

2. **Don't break quality**
   - Always validate with subjective listening tests
   - Set quality floor (MOS > 4.0)

3. **Don't add complexity without benefit**
   - Measure before/after carefully
   - Only add if >10% improvement

4. **Don't skip testing**
   - Every optimization must have tests
   - E2E tests prevent regressions

---

## 📚 Resources

### TensorRT on Jetson
- [Jetson TensorRT Guide](https://docs.nvidia.com/deeplearning/tensorrt/developer-guide/)
- [TensorRT Python API](https://docs.nvidia.com/deeplearning/tensorrt/api/python_api/)
- [TensorRT ONNX Workflow](https://github.com/NVIDIA/TensorRT/tree/main/samples/python/onnx_packnet)

### Quantization
- [PyTorch Quantization](https://pytorch.org/docs/stable/quantization.html)
- [NVIDIA Quantization Toolkit](https://github.com/NVIDIA/TensorRT-Model-Optimizer)

### CUDA Optimization
- [CUDA Graphs](https://developer.nvidia.com/blog/cuda-graphs/)
- [PyTorch CUDA Best Practices](https://pytorch.org/docs/stable/notes/cuda.html)

### Model Optimization
- [Model Distillation](https://arxiv.org/abs/1503.02531)
- [Knowledge Distillation for TTS](https://arxiv.org/abs/2010.12238)

---

## 🎓 Learning & Experimentation

For experimentation and learning:

1. **Profile Current Bottlenecks**
   ```bash
   nsys profile python scripts/test_max_autotune.py
   nsys stats profile.qdrep
   ```

2. **Measure Component Times**
   ```python
   # Add timing to each component
   - Preprocessing: ?ms
   - Model forward: ?ms
   - Vocoder: ?ms
   - Postprocessing: ?ms
   ```

3. **Try Different torch.compile Configs**
   ```python
   torch._inductor.config.triton.cudagraph_trees = True
   torch._inductor.config.max_autotune = True
   torch._inductor.config.freezing = True
   ```

---

## ✅ Success Metrics

For any new optimization:

1. **Performance**
   - RTF improvement > 10%
   - Variance increase < 5%

2. **Quality**
   - MOS score drop < 0.2
   - Pass A/B blind test

3. **Stability**
   - No crashes in 1000 requests
   - Memory usage stable

4. **Maintainability**
   - Code reviewed
   - Tests written
   - Documentation updated

---

**Status**: Ready for Phase 2 optimizations
**Next Major Milestone**: TensorRT Vocoder (RTF < 0.20)
**Long-term Goal**: RTF < 0.15 (whisper-level TTS with extra headroom)