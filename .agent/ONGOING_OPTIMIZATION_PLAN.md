# iShowTTS Ongoing Optimization & Maintenance Plan

**Date**: 2025-09-30
**Current Status**: Phase 2 Complete ✅
**Current RTF**: 0.192 (expected with TensorRT vocoder)
**Baseline RTF**: 1.32
**Total Speedup**: 6.9x

---

## 🎯 Current State Summary

### Achievements
- ✅ **Phase 1**: RTF 0.241 (torch.compile + FP16 + NFE=8)
- ✅ **Phase 2**: RTF 0.192 (TensorRT vocoder + 2.03x speedup)
- ✅ **6.9x total speedup** from baseline
- ✅ **Excellent quality**: NMSE 1.45e-4

### Applied Optimizations
1. torch.compile(mode='max-autotune')
2. FP16 Automatic Mixed Precision
3. NFE steps reduced (32 → 8)
4. Reference audio tensor caching
5. CUDA stream async operations
6. TensorRT vocoder (2.03x faster)

---

## 🚀 Phase 3: Further Optimization Opportunities

### Priority 1: Production Deployment & Testing

#### 1.1 End-to-End TensorRT Validation
**Goal**: Verify TensorRT vocoder in production environment
**Status**: Needs testing
**Tasks**:
- [ ] Run full E2E test with TensorRT vocoder enabled
- [ ] Measure actual RTF in production (vs expected 0.192)
- [ ] Validate quality with A/B testing
- [ ] Test under load (multiple concurrent requests)

**Files to check/modify**:
- `crates/tts-engine/src/lib.rs` - Ensure TensorRT vocoder path config
- `config/ishowtts.toml` - Add vocoder_local_path setting
- `third_party/F5-TTS/src/f5_tts/infer/utils_infer.py` - TensorRT integration

**Expected Impact**: Confirm 0.192 RTF target achieved

#### 1.2 Performance Mode Automation
**Goal**: Ensure GPU is locked to max performance automatically
**Status**: Manual script exists
**Tasks**:
- [ ] Create systemd service for jetson_clocks
- [ ] Add to start_all.sh with sudo check
- [ ] Document impact (RTF variance ±1.5% vs ±16%)

**Impact**: Consistent performance without manual intervention

---

### Priority 2: Quality & Stability

#### 2.1 Comprehensive Test Suite
**Goal**: Ensure optimizations don't break functionality
**Status**: Basic benchmarks exist, needs expansion
**Tasks**:
- [ ] Unit tests for TTS engine (Rust)
- [ ] Integration tests for F5-TTS API (Python)
- [ ] Quality regression tests (MOS scores)
- [ ] Load testing (concurrent requests)
- [ ] Memory leak testing (long-running)

**Files to create**:
- `crates/tts-engine/tests/` - Rust unit tests
- `tests/test_f5_api.py` - Python integration tests
- `tests/test_quality.py` - Quality benchmarks
- `tests/test_load.py` - Load testing

**Expected Time**: 2-3 hours (20% of effort per guidelines)

#### 2.2 Error Handling & Fallback
**Goal**: Graceful degradation if TensorRT fails
**Status**: Needs implementation
**Tasks**:
- [ ] Fallback to PyTorch vocoder if TensorRT unavailable
- [ ] Retry logic for torch.compile failures
- [ ] Memory pressure handling
- [ ] Request timeout handling

**Files to modify**:
- `third_party/F5-TTS/src/f5_tts/infer/utils_infer.py`
- `crates/tts-engine/src/lib.rs`

---

### Priority 3: Advanced Optimizations (Phase 4)

#### 3.1 INT8 Quantization
**Goal**: 1.5-2x additional speedup
**Status**: Not started
**Estimated RTF**: 0.10-0.13
**Tasks**:
- [ ] Profile model to identify quantization candidates
- [ ] Implement PTQ (Post-Training Quantization)
- [ ] Calibrate with representative dataset
- [ ] Validate quality (target NMSE < 1e-3)
- [ ] Benchmark performance gain

**Expected Impact**: 1.5-2x faster, RTF ~0.12

#### 3.2 Batch Processing
**Goal**: Higher throughput for concurrent requests
**Status**: Not started
**Tasks**:
- [ ] Modify F5-TTS API to support batching
- [ ] Implement request queue with batch aggregation
- [ ] Test optimal batch sizes (2, 4, 8)
- [ ] Measure throughput improvement

**Expected Impact**: 2-3x higher throughput, better GPU utilization

#### 3.3 Streaming Inference
**Goal**: Lower perceived latency
**Status**: Not started
**Tasks**:
- [ ] Implement chunked audio generation
- [ ] Stream chunks to frontend as available
- [ ] Test with various chunk sizes (0.5s, 1s, 2s)
- [ ] Measure latency-to-first-audio

**Expected Impact**: 50-70% lower perceived latency

#### 3.4 Model Optimization
**Goal**: Faster base model
**Status**: Research needed
**Options**:
- **Option A**: ONNX Runtime + TensorRT EP
- **Option B**: Model distillation (smaller student model)
- **Option C**: Export full model to TensorRT (like vocoder)

**Expected Impact**: 1.5-3x faster, depending on approach

---

## 🔧 Maintenance Tasks

### Regular Tasks

#### Weekly
- [ ] Monitor RTF metrics in production
- [ ] Check GPU utilization logs
- [ ] Review error logs for torch.compile issues
- [ ] Verify TensorRT engine still valid

#### Monthly
- [ ] Update PyTorch/TensorRT dependencies
- [ ] Re-benchmark after updates
- [ ] Review quality metrics (MOS scores)
- [ ] Check for F5-TTS upstream updates

#### Quarterly
- [ ] Full performance audit
- [ ] Quality A/B testing with users
- [ ] Consider new optimization techniques
- [ ] Update documentation

---

## 📊 Performance Monitoring

### Key Metrics to Track

1. **Latency**
   - Synthesis time (ms)
   - Real-Time Factor (RTF)
   - Latency-to-first-audio (if streaming)

2. **Throughput**
   - Requests per second
   - Concurrent request capacity
   - Queue wait time

3. **Quality**
   - MOS scores (subjective)
   - NMSE vs baseline (objective)
   - User feedback

4. **Resources**
   - GPU utilization (%)
   - GPU memory usage (GB)
   - CPU usage (%)
   - Memory leaks (monitor over time)

5. **Reliability**
   - Error rate (%)
   - torch.compile failures
   - TensorRT engine errors
   - Request timeouts

### Monitoring Setup

```bash
# GPU monitoring
nvidia-smi -l 1 --format=csv --query-gpu=timestamp,utilization.gpu,memory.used

# Log RTF for each request (already in backend)
RUST_LOG=ishowtts=debug cargo run -p ishowtts-backend

# Benchmark periodically
python scripts/test_max_autotune.py >> logs/performance_$(date +%Y%m%d).log
```

---

## 🎯 Recommended Next Steps

### Immediate (Today)
1. ✅ Review current status (this document)
2. [ ] Run E2E test with TensorRT vocoder
3. [ ] Measure actual production RTF
4. [ ] Document any issues found

### Short-term (This Week)
1. [ ] Create basic test suite (20% time allocation)
2. [ ] Add fallback logic for TensorRT
3. [ ] Automate GPU performance lock
4. [ ] Document production deployment

### Medium-term (This Month)
1. [ ] Investigate INT8 quantization feasibility
2. [ ] Profile model for bottlenecks
3. [ ] Implement batch processing prototype
4. [ ] A/B test quality with users

### Long-term (Next Quarter)
1. [ ] Deploy INT8 optimizations (if validated)
2. [ ] Implement streaming inference
3. [ ] Evaluate model distillation
4. [ ] Consider ONNX Runtime migration

---

## 📝 Testing Guidelines

### Performance Testing
- Always use `sudo jetson_clocks` before testing
- Run 5+ iterations for statistical significance
- Report mean, best, and variance
- Use consistent test cases (same audio/text)

### Quality Testing
- A/B test against baseline NFE=32
- Use both objective (NMSE) and subjective (MOS) metrics
- Test with diverse text samples
- Validate with actual danmaku use cases

### Load Testing
- Test 1, 2, 4, 8 concurrent requests
- Measure throughput and latency degradation
- Monitor GPU memory usage
- Test for memory leaks (24hr+ runs)

---

## 🚨 Risk Mitigation

### Known Risks
1. **TensorRT engine invalidation** after driver updates
   - Mitigation: Keep ONNX file, rebuild engine if needed

2. **torch.compile regressions** in PyTorch updates
   - Mitigation: Pin PyTorch version, test before updating

3. **Quality degradation** with aggressive optimizations
   - Mitigation: A/B testing, maintain quality thresholds

4. **Memory leaks** in long-running processes
   - Mitigation: Monitor memory, implement periodic restarts

5. **GPU frequency throttling** without performance lock
   - Mitigation: Automate jetson_clocks in startup

---

## 📚 Resources

### Documentation
- [STATUS.md](.agent/STATUS.md) - Current status
- [FINAL_OPTIMIZATION_REPORT.md](.agent/FINAL_OPTIMIZATION_REPORT.md) - Phase 1 & 2
- [SESSION_2025_09_30_TENSORRT.md](.agent/SESSION_2025_09_30_TENSORRT.md) - TensorRT session

### Scripts
- `scripts/test_max_autotune.py` - Performance validation
- `scripts/benchmark_vocoder.py` - TensorRT vs PyTorch vocoder
- `scripts/quick_performance_test.py` - Fast testing

### External
- [PyTorch torch.compile](https://pytorch.org/docs/stable/torch.compiler.html)
- [TensorRT Python API](https://docs.nvidia.com/deeplearning/tensorrt/api/python_api/)
- [Jetson Performance Tuning](https://docs.nvidia.com/jetson/archives/r36.3/DeveloperGuide/SD/PlatformPowerAndPerformance.html)

---

## ✅ Success Criteria

### Phase 3 (Next Target)
- [ ] **Production RTF < 0.20** (measured, not estimated)
- [ ] **Quality**: NMSE < 1e-3 vs baseline
- [ ] **Stability**: <0.1% error rate over 24hrs
- [ ] **Tests**: 80%+ code coverage for critical paths
- [ ] **Documentation**: Complete user guide + API docs

### Phase 4 (Stretch Goals)
- [ ] **RTF < 0.15** with INT8 quantization
- [ ] **10+ requests/sec** throughput
- [ ] **<500ms** latency-to-first-audio (streaming)
- [ ] **Zero downtime** deployment capability

---

**Status**: Ready for Phase 3
**Next Action**: Run E2E TensorRT validation
**Owner**: Agent/Maintainer
**Updated**: 2025-09-30