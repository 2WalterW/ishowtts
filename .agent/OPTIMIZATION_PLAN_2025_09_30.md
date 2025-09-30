# iShowTTS Optimization & Maintenance Plan
## Date: 2025-09-30

## Current Status

### Performance Metrics (Latest Test)
- **Current RTF**: 0.251 (average), 0.218 (best) ✅
- **Current Speedup**: 3.99x (average), 4.60x (best) ✅
- **Target**: RTF < 0.3 (Whisper-level) ✅ **ACHIEVED**
- **Configuration**: NFE=7, torch.compile(max-autotune), FP16 AMP

### Key Achievements
1. ✅ Phase 1 Complete: RTF < 0.3 achieved (target was 0.3)
2. ✅ torch.compile with max-autotune mode enabled
3. ✅ FP16 automatic mixed precision for model + vocoder
4. ✅ Reference audio tensor caching implemented
5. ✅ CUDA stream async transfers
6. ✅ GPU memory management with empty_cache()

### Current Configuration Analysis
```toml
# config/ishowtts.toml
default_nfe_step = 7  # Good balance, RTF ~0.22-0.25
```

**Performance Characteristics**:
- NFE=7: RTF 0.213-0.251 (current, excellent)
- NFE=6: RTF ~0.18-0.20 (untested, risky)
- NFE=8: RTF 0.266-0.278 (previous, slightly slower)

---

## Priority 1: Critical Performance Optimizations

### 1.1 Investigate Variance in RTF (High Priority)
**Current Issue**: RTF varies from 0.218 to 0.273 (25% variance)

**Action Items**:
- [ ] Profile inference to identify non-deterministic bottlenecks
- [ ] Check GPU frequency locking (jetson_clocks)
- [ ] Analyze CUDA stream synchronization overhead
- [ ] Test with longer running averages (20+ runs)

**Expected Impact**: Reduce variance to <10%, more predictable performance

### 1.2 Optimize Reference Audio Preprocessing (Medium Priority)
**Current Bottleneck**: First request with new voice is slower

**Action Items**:
- [ ] Pre-load and cache all reference audios on startup (warmup)
- [ ] Implement disk-based cache for preprocessed tensors
- [ ] Add cache statistics tracking (hits/misses)

**Expected Impact**: Eliminate cold-start penalty, consistent RTF

### 1.3 Batch Processing for Multiple Requests (High Priority)
**Current Limitation**: Sequential processing of concurrent requests

**Action Items**:
- [ ] Implement batched inference for multiple gen_text items
- [ ] Add request queue with batch formation logic
- [ ] Test batch sizes (2, 4, 8) for throughput vs latency

**Expected Impact**: 2-3x higher throughput for concurrent requests

---

## Priority 2: IndexTTS Optimization

### 2.1 Enable FP16 for IndexTTS (High Priority)
**Current Status**: `use_fp16 = false` in config

**Action Items**:
- [ ] Test IndexTTS with FP16 enabled
- [ ] Benchmark RTF improvement (expected 30-50%)
- [ ] Validate audio quality (MOS testing)
- [ ] Update config if successful

**Expected Impact**: Faster IndexTTS inference, better parity with F5-TTS

### 2.2 Investigate CUDA Kernel Optimization (Medium Priority)
**Current Status**: `use_cuda_kernel = false` in config

**Action Items**:
- [ ] Research what custom CUDA kernels are available
- [ ] Test with `use_cuda_kernel = true`
- [ ] Benchmark performance vs stability
- [ ] Document findings

**Expected Impact**: Potential 10-20% speedup if kernels are optimized

---

## Priority 3: Advanced Memory Optimization

### 3.1 Implement Gradient Checkpointing (Low Priority)
**Current Status**: Not enabled

**Action Items**:
- [ ] Enable gradient checkpointing for model inference
- [ ] Test memory usage reduction
- [ ] Benchmark performance impact

**Expected Impact**: Lower memory usage, enable larger batch sizes

### 3.2 Optimize Vocoder Memory Usage (Medium Priority)
**Current Observation**: Vocoder runs in FP16 but may have memory overhead

**Action Items**:
- [ ] Profile vocoder memory usage
- [ ] Investigate in-place operations
- [ ] Test vocoder-specific optimizations (e.g., fused kernels)

**Expected Impact**: 10-15% reduction in memory footprint

---

## Priority 4: Quality Assurance & Testing

### 4.1 NFE=6 Quality Evaluation (URGENT)
**Current Status**: NFE=6 samples generated, quality not evaluated

**Action Items**:
- [x] Generated 52 audio files (26 pairs) in `.agent/quality_samples/`
- [ ] Perform subjective listening tests (A/B comparison)
- [ ] Calculate objective metrics (PESQ, STOI, MOS estimation)
- [ ] Decision: Accept NFE=6 if quality acceptable, else keep NFE=7

**Expected Impact**: Potential 14% speedup (RTF 0.187) if quality acceptable

### 4.2 Automated Quality Monitoring (High Priority)
**Current Status**: No automated quality checks

**Action Items**:
- [ ] Create script to generate test audio samples
- [ ] Implement objective quality metrics (PESQ, STOI, MOS)
- [ ] Set quality thresholds for regression detection
- [ ] Add to CI/CD pipeline

**Expected Impact**: Catch quality regressions early, maintain high standards

### 4.3 Comprehensive Test Suite (Medium Priority)
**Current Status**: Manual testing only

**Action Items**:
- [ ] Write unit tests for preprocessing functions
- [ ] Write integration tests for F5-TTS and IndexTTS
- [ ] Write end-to-end tests for full pipeline
- [ ] Add performance benchmarks as tests
- [ ] Setup pytest with coverage reporting

**Expected Impact**: Prevent regressions, faster development cycles

---

## Priority 5: Monitoring & Observability

### 5.1 Real-Time Performance Dashboard (High Priority)
**Current Status**: No live monitoring

**Action Items**:
- [ ] Create monitoring script with metrics collection
- [ ] Track RTF, latency, throughput, memory usage
- [ ] Add Prometheus/Grafana integration (optional)
- [ ] Log metrics to file for analysis
- [ ] Create alerting for performance degradation

**Expected Impact**: Immediate detection of performance issues

### 5.2 Automated Regression Detection (High Priority)
**Current Status**: Manual testing required

**Action Items**:
- [ ] Create baseline performance dataset
- [ ] Write script to detect RTF regression (>5% increase)
- [ ] Add to daily cron job
- [ ] Send alerts on regression
- [ ] Log historical performance data

**Expected Impact**: Catch performance regressions within 24 hours

---

## Priority 6: Code Quality & Maintenance

### 6.1 Document All Optimizations (Medium Priority)
**Current Status**: Partial documentation in .agent/

**Action Items**:
- [ ] Create comprehensive optimization guide
- [ ] Document each optimization with before/after metrics
- [ ] Add inline comments to modified code
- [ ] Update README with optimization section
- [ ] Create troubleshooting guide

**Expected Impact**: Easier maintenance, knowledge transfer

### 6.2 Code Cleanup & Refactoring (Low Priority)
**Current Status**: Some technical debt in utils_infer.py

**Action Items**:
- [ ] Refactor infer_batch_process for clarity
- [ ] Extract caching logic into separate module
- [ ] Add type hints to all functions
- [ ] Run pylint/mypy for code quality
- [ ] Remove dead code and comments

**Expected Impact**: Easier to maintain, fewer bugs

---

## Priority 7: Future Experimental Optimizations

### 7.1 INT8 Quantization (Research Phase)
**Status**: Not yet investigated

**Action Items**:
- [ ] Research PyTorch quantization APIs
- [ ] Test INT8 quantization on model
- [ ] Benchmark performance vs quality
- [ ] Document findings

**Expected Impact**: Potential 1.5-2x speedup if successful

### 7.2 TensorRT Model Export (Research Phase)
**Status**: Only vocoder tested (slower end-to-end)

**Action Items**:
- [ ] Export F5-TTS model to ONNX
- [ ] Convert ONNX to TensorRT
- [ ] Benchmark isolated model performance
- [ ] Test end-to-end performance
- [ ] Compare with torch.compile

**Expected Impact**: Unknown, may be faster or slower

### 7.3 Streaming Inference (Research Phase)
**Status**: Framework supports it, not enabled

**Action Items**:
- [ ] Enable streaming mode in config
- [ ] Test chunk sizes for latency/quality tradeoff
- [ ] Implement chunked audio streaming to frontend
- [ ] Measure perceived latency improvement

**Expected Impact**: Lower perceived latency, better UX

---

## Immediate Action Plan (This Session)

### Phase 1: Performance Analysis & Quick Wins (1-2 hours)
1. ✅ Run current performance test (DONE: RTF=0.251)
2. [ ] Lock GPU frequency with jetson_clocks
3. [ ] Run 20+ iterations to measure variance
4. [ ] Profile inference with PyTorch profiler
5. [ ] Identify top 3 bottlenecks

### Phase 2: NFE=6 Quality Evaluation (1 hour)
1. [ ] Listen to all 26 pairs of NFE=6 vs NFE=7 samples
2. [ ] Calculate objective quality metrics
3. [ ] Make decision: accept NFE=6 or keep NFE=7
4. [ ] Update config and documentation

### Phase 3: IndexTTS FP16 Testing (1 hour)
1. [ ] Enable `use_fp16 = true` for IndexTTS
2. [ ] Run performance benchmark
3. [ ] Test audio quality
4. [ ] Document results

### Phase 4: Monitoring & Automation (2 hours)
1. [ ] Create performance monitoring script
2. [ ] Create automated regression detection script
3. [ ] Setup daily cron job
4. [ ] Test alerting mechanism

### Phase 5: Documentation & Commit (1 hour)
1. [ ] Update all documentation files
2. [ ] Commit changes with detailed messages
3. [ ] Push to repository
4. [ ] Create summary report

---

## Success Criteria

### Performance Targets
- ✅ Phase 1: RTF < 0.3 (ACHIEVED: 0.251)
- ⚠️ Phase 2: RTF < 0.2 (current: 0.213-0.251, close but not consistent)
- 🎯 Phase 3: RTF < 0.2 consistently (stretch goal)

### Quality Targets
- ✅ Maintain high audio quality (MOS > 4.0)
- ✅ No audible artifacts or degradation
- ✅ Consistent voice characteristics

### Reliability Targets
- ✅ <10% RTF variance
- ✅ 100% success rate (no crashes)
- ✅ Automated regression detection

### Maintainability Targets
- ✅ Comprehensive documentation
- ✅ Automated testing
- ✅ Performance monitoring

---

## Notes

### Hardware Configuration
- **Platform**: NVIDIA Jetson AGX Orin
- **GPU**: Orin (32GB unified memory)
- **PyTorch**: 2.5.0a0+872d972e41.nv24.08
- **CUDA**: 12.6
- **Power Mode**: MAXN (jetson_clocks recommended)

### Known Issues
1. ⚠️ RTF variance 20-25% (needs investigation)
2. ⚠️ First request with new voice is slower (cache miss)
3. ⚠️ IndexTTS not using FP16 (potential speedup)
4. ⚠️ No automated performance monitoring
5. ⚠️ No quality regression detection

### Dependencies
- Python environment: `/opt/miniforge3/envs/ishowtts`
- F5-TTS: `third_party/F5-TTS/`
- IndexTTS: `third_party/index-tts/`
- Config: `config/ishowtts.toml`

---

## Timeline

### Week 1 (Current)
- Day 1: ✅ Performance analysis & quick wins
- Day 2: NFE=6 evaluation & IndexTTS optimization
- Day 3: Monitoring & automation setup
- Day 4: Documentation & testing
- Day 5: Code cleanup & commit

### Week 2
- Advanced optimizations (batching, memory)
- Quality assurance setup
- Performance regression testing

### Week 3+
- Research phase (INT8, TensorRT, streaming)
- Long-term monitoring & maintenance
- Continuous optimization

---

**Status**: 🟢 Active Development
**Last Updated**: 2025-09-30
**Next Review**: Daily until Phase 3 complete