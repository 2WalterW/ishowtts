# Next Optimization Ideas for iShowTTS
**Date**: 2025-09-30
**Current RTF**: 0.168 (Target <0.20 ✅ EXCEEDED)
**Status**: Optional Optimizations

---

## Context

The system has already exceeded all performance targets with RTF=0.168 (16% better than target).
These optimizations are **optional** and should only be pursued if:
1. New requirements emerge (e.g., RTF <0.15)
2. Throughput needs to increase for concurrent requests
3. User experience improvements are prioritized

---

## Micro-Optimizations (Low Hanging Fruit)

### 1. Pinyin Conversion Caching
**Current State**: Convert text to pinyin on every inference
**File**: `third_party/F5-TTS/src/f5_tts/infer/utils_infer.py:533`
**Estimated Impact**: 1-5ms per inference
**Risk**: Low
**Effort**: 1-2 hours

**Implementation**:
```python
_pinyin_cache = {}  # Cache (text, hash) -> pinyin_result

def cached_convert_char_to_pinyin(text_list):
    cache_key = tuple(text_list)
    if cache_key in _pinyin_cache:
        return _pinyin_cache[cache_key]
    result = convert_char_to_pinyin(text_list)
    _pinyin_cache[cache_key] = result
    return result
```

**When to use**: If the same text is synthesized repeatedly (e.g., notification templates)

---

### 2. Warmup Optimization
**Current State**: First inference is slow due to compilation
**Estimated Impact**: 2-3s saved on startup (one-time)
**Risk**: Low
**Effort**: 2-3 hours

**Implementation**:
- Pre-compile model with representative input shapes
- Save compiled artifacts if possible
- Load compiled model directly

**Trade-off**: Startup time vs first inference speed

---

### 3. Tokenizer Optimization
**Current State**: Text tokenization happens on CPU
**File**: `third_party/F5-TTS/src/f5_tts/model/utils.py`
**Estimated Impact**: 1-2ms per inference
**Risk**: Low
**Effort**: 3-4 hours

**Implementation**:
- Profile tokenization overhead
- Consider batching tokenization
- Move to GPU if beneficial (unlikely)

---

### 4. Memory Pool Pre-allocation
**Current State**: PyTorch allocates memory dynamically
**Estimated Impact**: 1-3ms per inference (first few)
**Risk**: Low
**Effort**: 1-2 hours

**Implementation**:
```python
# Pre-allocate memory pool during warmup
dummy_input = torch.randn(expected_shape, device=device)
_ = model(dummy_input)
del dummy_input
torch.cuda.synchronize()
```

**Trade-off**: Memory usage vs allocation overhead

---

### 5. Remove Unnecessary Computations
**Current State**: Some operations may not be needed
**Estimated Impact**: Variable (need profiling)
**Risk**: Low if validated
**Effort**: 4-6 hours (requires profiling)

**Areas to investigate**:
- Duration calculation (can be cached for same text length)
- Text length computation (UTF-8 encoding overhead)
- Redundant tensor operations in model forward pass

---

## Medium-Effort Optimizations

### 6. NFE=6 with Quality Validation
**Status**: Quality samples generated
**Estimated Impact**: RTF ~0.145 (14% speedup)
**Risk**: Medium (quality degradation)
**Effort**: 2-3 hours (listening tests + validation)

**Next Steps**:
1. Conduct blind listening tests
2. Compare MOS scores if possible
3. A/B test with users
4. If acceptable, update config to NFE=6

**Files to check**:
- `.agent/quality_samples/nfe6_*.wav`
- `.agent/quality_samples/nfe7_*.wav` (baseline)

---

### 7. Dynamic NFE Based on Audio Length
**Current State**: Fixed NFE=7 for all inputs
**Estimated Impact**: 5-15% speedup on average
**Risk**: Medium (need quality validation per length)
**Effort**: 1 week

**Implementation**:
```python
def get_optimal_nfe(audio_length_seconds):
    if audio_length_seconds < 5:
        return 8  # Better quality for short audio
    elif audio_length_seconds < 15:
        return 7  # Balanced
    else:
        return 6  # Faster for long audio
```

**Rationale**: Longer audio can tolerate lower NFE (quality is amortized)

---

### 8. Batch Processing for Multiple Requests
**Current State**: Sequential processing of requests
**Estimated Impact**: 2-3x throughput for concurrent requests
**Risk**: Low (no impact on single request)
**Effort**: 1-2 weeks

**Implementation**:
- Modify `tts-engine` Rust wrapper to accumulate requests
- Batch requests with similar audio lengths
- Process batch in single forward pass
- Distribute results back to requesters

**Benefits**:
- Better GPU utilization
- Amortize fixed overhead across requests
- Higher throughput during peak times

**Considerations**:
- Increased latency for individual requests in batch
- Complexity in request routing

---

### 9. Streaming Inference (Chunked Output)
**Current State**: Full synthesis before returning audio
**Estimated Impact**: Lower perceived latency (no RTF improvement)
**Risk**: Medium (complex implementation)
**Effort**: 2-3 weeks

**Implementation**:
- Split diffusion sampling into chunks
- Stream vocoder output as chunks complete
- Update frontend to handle streaming audio playback

**Benefits**:
- Lower time-to-first-audio (TTFA)
- Better user experience
- No waiting for full synthesis

**Considerations**:
- Quality at chunk boundaries
- Buffering and synchronization
- Frontend changes needed

---

### 10. Model Distillation
**Current State**: F5-TTS Base model
**Estimated Impact**: 30-50% speedup
**Risk**: High (quality degradation, requires retraining)
**Effort**: 2-4 weeks + GPU resources

**Implementation**:
- Train smaller "student" model using F5-TTS as "teacher"
- Knowledge distillation techniques
- Validate quality with extensive testing

**Considerations**:
- Requires training data and GPU resources
- May not preserve quality
- Alternative: Use existing smaller TTS models

---

## Advanced Optimizations (High Effort)

### 11. INT8 Quantization
**Estimated Impact**: RTF ~0.08-0.11 (1.5-2x speedup)
**Risk**: High (quality degradation)
**Effort**: 2-4 weeks

**Implementation Steps**:
1. **Post-Training Quantization (PTQ)**
   - Calibration dataset preparation
   - Use PyTorch's quantization API
   - Validate accuracy

2. **Quantization-Aware Training (QAT)**
   - Fine-tune model with quantization
   - Requires training infrastructure
   - Better quality than PTQ

**Code Example**:
```python
import torch.quantization as quantization

# PTQ approach
model_int8 = quantization.quantize_dynamic(
    model, {torch.nn.Linear}, dtype=torch.qint8
)

# QAT approach (requires retraining)
model.qconfig = quantization.get_default_qat_qconfig('fbgemm')
model_prepared = quantization.prepare_qat(model)
# ... training loop ...
model_int8 = quantization.convert(model_prepared)
```

**Considerations**:
- Only quantize model, not vocoder (vocoder is already fast)
- Test quality extensively
- May need per-layer sensitivity analysis

---

### 12. Model Architecture Search (NAS)
**Estimated Impact**: 20-50% speedup
**Risk**: Very high (requires research)
**Effort**: 2-3 months

**Approach**:
- Automated search for efficient architectures
- Neural Architecture Search techniques
- Optimize for speed/quality tradeoff

**Not recommended**: Extremely high effort, uncertain outcome

---

### 13. Custom CUDA Kernels
**Estimated Impact**: 10-30% speedup for specific operations
**Risk**: High (maintenance burden)
**Effort**: 4-8 weeks

**Approach**:
1. Profile to find hotspots
2. Implement custom CUDA kernels for bottlenecks
3. Use Triton or raw CUDA
4. Validate correctness and performance

**When to consider**:
- Only if profiling shows specific operation is bottleneck
- Only if existing optimizations are exhausted
- Only if you have CUDA expertise

---

### 14. Model Export to ONNX + TensorRT
**Estimated Impact**: 20-40% speedup (uncertain)
**Risk**: Medium (may not beat torch.compile)
**Effort**: 2-3 weeks

**Why it may not help**:
- TensorRT vocoder was tested and slower end-to-end
- torch.compile is already excellent
- Dynamic shapes are challenging for TensorRT

**When to try**:
- If torch.compile stops improving
- If INT8 quantization is needed (TensorRT has better INT8 support)
- If deployment to non-PyTorch environments

---

### 15. CUDA Graphs (Static Shapes)
**Estimated Impact**: 10-15% speedup for fixed shapes
**Risk**: Medium (only works for static shapes)
**Effort**: 1-2 weeks

**Implementation**:
```python
# Record CUDA graph
g = torch.cuda.CUDAGraph()
with torch.cuda.graph(g):
    output = model(static_input)

# Replay graph (much faster)
g.replay()
```

**Limitation**: Requires fixed input/output shapes
- Text length varies
- Audio length varies
- Not suitable for production (unless batch to fixed sizes)

---

## Performance Analysis Tools

### Profiling Commands
```bash
# PyTorch profiler
/opt/miniforge3/envs/ishowtts/bin/python scripts/profile_bottlenecks.py

# NVIDIA Nsight Systems
nsys profile -o profile.qdrep python scripts/quick_performance_test.py

# NVIDIA Nsight Compute (kernel-level)
ncu --target-processes all python scripts/quick_performance_test.py
```

### What to Look For
1. **Time spent per operation**
   - Model forward: Should be ~70-80% of total time
   - Vocoder: Should be ~15-20%
   - Preprocessing: Should be <5%

2. **GPU utilization**
   - Should be >80% during inference
   - Low utilization = CPU bottleneck or sync issues

3. **Memory bandwidth**
   - Should be saturated during compute-intensive ops
   - Low bandwidth = compute-bound (good for optimizations)

---

## Optimization Decision Matrix

| Optimization | Impact | Risk | Effort | Priority |
|--------------|--------|------|--------|----------|
| Pinyin caching | Low | Low | Low | 🟢 If needed |
| NFE=6 | High | Medium | Low | 🟡 Quality test first |
| Batch processing | High* | Low | Medium | 🟢 For high load |
| Streaming inference | Medium** | Medium | High | 🟡 UX improvement |
| INT8 quantization | High | High | High | 🔴 Last resort |
| Dynamic NFE | Medium | Medium | Medium | 🟡 After analysis |
| Model distillation | High | High | Very High | 🔴 Not recommended |
| Custom CUDA | Medium | High | Very High | 🔴 Not recommended |

*High for throughput, not single-request latency
**High for perceived latency, not actual RTF

---

## Recommendation Priority

### Do Now (If Needed)
1. ✅ NFE=6 quality validation - Already have samples
2. ✅ Monitoring and stability - Already implemented

### Do Next (If Required)
1. Batch processing - If concurrent load increases
2. Streaming inference - If UX is priority
3. Dynamic NFE - After analysis of production patterns

### Research (Optional)
1. INT8 quantization - Only if RTF <0.10 needed
2. Model TensorRT - Only if INT8 is pursued

### Don't Do (Not Worth It)
1. Custom CUDA kernels - Too much effort
2. Model distillation - Too risky
3. Architecture search - Too expensive

---

## Monitoring for Optimization Opportunities

### Metrics to Track
```bash
# Daily regression check
/opt/miniforge3/envs/ishowtts/bin/python scripts/detect_regression.py

# Weekly performance review
/opt/miniforge3/envs/ishowtts/bin/python scripts/extended_performance_test.py

# Production metrics (add to backend)
- Requests per second
- Average audio length
- Queue depth
- GPU utilization
```

### When to Optimize
- [ ] RTF consistently > 0.20 (regression)
- [ ] User complaints about quality
- [ ] Concurrent request queue growing
- [ ] New hardware available (faster GPU)
- [ ] New requirements (lower latency)

### When NOT to Optimize
- ✅ Current RTF < 0.20 (already met target)
- ✅ Variance < 10% (stable performance)
- ✅ Users satisfied with quality
- ✅ System handles current load
- ✅ No new requirements

---

## Conclusion

**Current Status**: System exceeds all targets (RTF 0.168 vs target <0.20)

**Recommendation**:
- Focus on **monitoring** and **stability**
- Implement **batch processing** only if concurrent load requires it
- Validate **NFE=6** if 14% speedup is desired
- Consider **streaming inference** for better UX
- Avoid high-risk optimizations unless absolutely necessary

**Philosophy**:
> "Premature optimization is the root of all evil" - Donald Knuth
>
> The system is already fast enough. Focus on features, not performance.

---

**Last Updated**: 2025-09-30
**Next Review**: When new requirements emerge or production metrics indicate need