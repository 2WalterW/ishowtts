# Next Session Plan - 2025-09-30

## 📊 Current Status Summary

**Performance**: ✅ **EXCELLENT**
- Best RTF: **0.233** (target < 0.30) ✅ **NEW RECORD**
- Mean RTF: **0.250** ✅
- Speedup: **5.7x** from baseline
- Variance: **±3.7%** (excellent stability)
- **Phase 1 COMPLETE & EXCEEDED TARGET**

**System Health**: ✅ **OPTIMAL**
- GPU: Locked to MAXN mode (1300.5 MHz)
- CPU: All cores at 2.2 GHz
- Memory: Stable, no leaks detected
- Optimizations: All active and verified

**Documentation**: ✅ **UP TO DATE**
- Performance analysis complete
- All scripts documented
- Test suite in place
- Regression detection ready

---

## 🎯 Phase 3 Goals (Next Major Milestone)

### Target: RTF < 0.20 (25% faster)

**Current**: RTF = 0.233
**Target**: RTF = 0.20
**Gap**: 0.033 (14% improvement needed)

### Priority Optimizations

#### 1. INT8 Quantization (HIGHEST PRIORITY)
**Goal**: 1.5-2x speedup on model inference (70% of time)
**Expected RTF**: 0.12-0.16 (would exceed Phase 3 target)

**Tasks**:
- [ ] Research PyTorch quantization APIs
- [ ] Prepare calibration dataset
- [ ] Implement post-training quantization
- [ ] Validate quality (target: <5% MOS drop)
- [ ] Benchmark performance
- [ ] Compare with baseline

**Resources**:
- https://pytorch.org/docs/stable/quantization.html
- https://pytorch.org/tutorials/recipes/quantization.html
- `.agent/LONG_TERM_ROADMAP.md` (lines 32-73)

**Estimated Effort**: 1-2 weeks
**Risk**: Medium (quality sensitive)

#### 2. Streaming Inference (HIGH PRIORITY - UX)
**Goal**: Reduce perceived latency by 50-70%
**Note**: Does NOT improve RTF, but much better UX

**Tasks**:
- [ ] Implement chunked generation (1-2s chunks)
- [ ] Add SSE streaming to backend
- [ ] Update frontend to stream playback
- [ ] Test cross-fade between chunks

**Estimated Effort**: 2 weeks
**Risk**: Low

#### 3. Batch Processing (MEDIUM PRIORITY - Throughput)
**Goal**: 2-3x requests/second during peak loads
**Trade-off**: +50-100ms latency per request

**Tasks**:
- [ ] Implement request batching (50-100ms window)
- [ ] Update backend to batch process
- [ ] Benchmark throughput improvement
- [ ] Test under load

**Estimated Effort**: 1 week
**Risk**: Low

---

## ⚠️ Critical Findings This Session

### 1. NFE Configuration Discrepancy

**Issue**: F5TTS Python API defaults to NFE=32 (slow), but backend uses NFE=8 (fast)

**Impact**:
- Direct Python API calls: RTF = 0.91 ❌ (very slow)
- Backend with config: RTF = 0.233 ✅ (optimal)

**Root Cause**:
```python
# In f5_tts/api.py:129
def infer(self, ..., nfe_step=32, ...):  # ← Default is 32!
```

Backend correctly overrides with:
```toml
# In config/ishowtts.toml:19
default_nfe_step = 8
```

**Solution**:
All Python scripts MUST pass `nfe_step=8` explicitly:
```python
model.infer(..., nfe_step=8, ...)  # ← CRITICAL
```

**Action Items**:
- [ ] Update all scripts to pass nfe_step=8
- [ ] Add warning in documentation
- [ ] Consider patching F5TTS API (optional)

### 2. GPU Lock Critical for Performance

**Without lock**:
- Mean RTF: 0.352
- Variance: ±16%

**With lock**:
- Mean RTF: 0.250
- Variance: ±3.7%

**Lock command** (must run after each reboot):
```bash
sudo jetson_clocks
sudo nvpmodel -m 0
```

**Recommendation**: Add to startup script or systemd service

### 3. Component Breakdown

Based on profiling estimates:

| Component | Time | % | Optimization Status |
|-----------|------|---|---------------------|
| Model | 1.47s | 70% | torch.compile + FP16 ✅ |
| Vocoder | 0.52s | 25% | torch.compile + FP16 ✅ |
| Audio | 0.05s | 2.5% | Rust optimized ✅ |
| Memory | 0.05s | 2.5% | CUDA streams ✅ |

**Primary Bottleneck**: Model inference (70%)
**Next Target**: INT8 quantization of model

---

## 📋 Immediate Action Items (Next Session Start)

### Quick Checks (5 minutes)
1. [ ] Verify GPU lock status: `sudo jetson_clocks --show`
2. [ ] Run quick performance test: `python scripts/test_max_autotune.py`
3. [ ] Check for system updates or changes
4. [ ] Review git status

### If Performance Degraded
1. [ ] Lock GPU: `sudo jetson_clocks && sudo nvpmodel -m 0`
2. [ ] Check for competing processes: `top`, `nvidia-smi`
3. [ ] Verify optimizations: Read F5TTS api.py lines 88-99
4. [ ] Check NFE config: `grep default_nfe_step config/ishowtts.toml`

### If Starting INT8 Quantization
1. [ ] Read PyTorch quantization docs
2. [ ] Create calibration dataset (100-500 samples)
3. [ ] Create branch: `git checkout -b feat/int8-quantization`
4. [ ] Create test script: `scripts/test_int8_quantization.py`
5. [ ] Document approach in `.agent/INT8_PLAN.md`

### If Starting Streaming Inference
1. [ ] Review SSE implementation in backend
2. [ ] Create branch: `git checkout -b feat/streaming-inference`
3. [ ] Design chunking strategy (1-2s chunks)
4. [ ] Update API design document
5. [ ] Create prototype: `scripts/test_streaming.py`

---

## 🔧 Maintenance Tasks

### Daily (Automated)
- [ ] Run regression detection: `python scripts/detect_regression.py`
- [ ] Monitor logs for errors
- [ ] Check GPU lock status

### Weekly
- [ ] Run full test suite: `bash tests/run_all_tests.sh`
- [ ] Review performance trends
- [ ] Update documentation if needed
- [ ] Rotate logs

### Monthly
- [ ] Comprehensive benchmark
- [ ] Quality evaluation (MOS testing)
- [ ] Update dependencies
- [ ] Review optimization roadmap

---

## 📁 Key Files & Locations

### Documentation
- `.agent/STATUS.md` - Quick status reference
- `.agent/PERFORMANCE_ANALYSIS_2025_09_30.md` - Latest analysis **NEW**
- `.agent/LONG_TERM_ROADMAP.md` - Phase 3+ roadmap
- `.agent/FINAL_OPTIMIZATION_REPORT.md` - Phase 1 complete report
- `README.md` - Project overview

### Scripts
- `scripts/test_max_autotune.py` - Quick performance test (5 runs)
- `scripts/quick_profile.py` - Component profiling **NEW**
- `scripts/profile_bottlenecks.py` - Detailed profiling
- `scripts/detect_regression.py` - Automated regression detection
- `scripts/test_nfe_performance.py` - NFE comparison

### Configuration
- `config/ishowtts.toml` - Backend config (NFE=8 setting)
- `crates/tts-engine/src/lib.rs` - Rust engine
- `third_party/F5-TTS/src/f5_tts/api.py` - Python API (NOT in git)
- `third_party/F5-TTS/src/f5_tts/infer/utils_infer.py` - Inference utils (NOT in git)

### Logs & Results
- `logs/performance_history.json` - Regression detection history
- `logs/regression/` - Daily regression test results
- `logs/backend.log` - Backend logs
- `logs/frontend.log` - Frontend logs

---

## 🚀 Quick Start Commands

### Performance Testing
```bash
# Quick test (5 runs, ~2 minutes)
/opt/miniforge3/envs/ishowtts/bin/python scripts/test_max_autotune.py

# Regression detection (5 runs + analysis)
/opt/miniforge3/envs/ishowtts/bin/python scripts/detect_regression.py

# Component profiling (3 runs, detailed)
/opt/miniforge3/envs/ishowtts/bin/python scripts/quick_profile.py
```

### System Status
```bash
# GPU lock status
sudo jetson_clocks --show
sudo nvpmodel -q

# GPU utilization
nvidia-smi

# Lock GPU to max performance
sudo jetson_clocks
sudo nvpmodel -m 0
```

### Development
```bash
# Create feature branch
git checkout -b feat/optimization-name

# Run backend
cargo run --release -p ishowtts-backend -- --config config/ishowtts.toml

# Run tests
cargo test --workspace

# Commit and push
git add -A
git commit -m "Your message

🤖 Generated with Claude Code

Co-Authored-By: Claude <noreply@anthropic.com>"
git push origin branch-name
```

---

## 📖 Phase 3 Research Resources

### INT8 Quantization
- **PyTorch Official**: https://pytorch.org/docs/stable/quantization.html
- **Tutorial**: https://pytorch.org/tutorials/advanced/static_quantization_tutorial.html
- **Dynamic Quantization**: https://pytorch.org/tutorials/recipes/recipes/dynamic_quantization.html
- **Best Practices**: https://pytorch.org/blog/quantization-in-practice/

### Model Optimization
- **torch.compile**: https://pytorch.org/tutorials/intermediate/torch_compile_tutorial.html
- **CUDA Graphs**: https://pytorch.org/blog/accelerating-pytorch-with-cuda-graphs/
- **Mixed Precision**: https://pytorch.org/docs/stable/notes/amp_examples.html

### Jetson Resources
- **Performance Guide**: https://docs.nvidia.com/jetson/archives/r35.4.1/DeveloperGuide/
- **jetson_clocks**: https://docs.nvidia.com/jetson/archives/r35.4.1/DeveloperGuide/text/SD/PlatformPowerAndPerformance.html

### TTS Research
- **F5-TTS Paper**: https://arxiv.org/abs/2410.06885
- **Flow Matching**: Recent diffusion model improvements
- **Consistency Models**: 1-step generation research

---

## 🎯 Success Metrics

### Phase 3 Target (RTF < 0.20)
- **Current**: RTF = 0.233
- **Target**: RTF = 0.20
- **Stretch**: RTF = 0.15

### Quality Requirements
- MOS score: > 4.0 (maintain current)
- Speaker similarity: > 0.85
- Intelligibility: > 95%
- Artifact rate: < 1%

### System Requirements
- Uptime: > 99.5%
- Error rate: < 0.1%
- Memory: No leaks
- GPU utilization: > 80%

---

## 💡 Tips & Tricks

### Profiling
- Use `torch.cuda.synchronize()` before/after timing for accurate GPU timing
- Always run warmup iterations (2-3) before profiling
- Lock GPU frequencies for consistent results

### NFE Tuning
- NFE=8: RTF ~0.25, good quality ← **CURRENT**
- NFE=6: RTF ~0.17, fair quality (experimental)
- NFE=12: RTF ~0.52, better quality
- NFE=32: RTF ~1.3, best quality (baseline)

### torch.compile Modes
- `default`: Fast compile, okay speed
- `reduce-overhead`: Faster execution
- `max-autotune`: Slowest compile, **fastest execution** ← **CURRENT**

### Troubleshooting
- If RTF suddenly increases: Check GPU lock and system load
- If variance high (>20%): Lock GPU or reduce background processes
- If quality degrades: Check NFE setting and model weights
- If OOM: Check batch size and model precision

---

## 🔄 Handoff Notes

### What Was Done This Session (2025-09-30)
1. ✅ Verified current performance (RTF 0.233 - NEW RECORD)
2. ✅ Profiled system and identified bottlenecks
3. ✅ Discovered NFE configuration discrepancy
4. ✅ Created comprehensive performance analysis
5. ✅ Updated STATUS.md with latest results
6. ✅ Committed and pushed all changes

### What's Ready for Next Session
1. ✅ GPU locked to maximum performance
2. ✅ All optimizations verified active
3. ✅ Performance baseline established
4. ✅ Documentation up to date
5. ✅ Test scripts ready
6. ✅ Regression detection configured

### Recommended Next Steps (Priority Order)
1. **HIGH**: Start INT8 quantization research (Phase 3 main goal)
2. **HIGH**: Implement streaming inference (UX improvement)
3. **MEDIUM**: Add batch processing (throughput optimization)
4. **LOW**: Create more unit tests (technical debt)
5. **LOW**: Investigate NFE=6 (experimental, risky)

### Open Questions
1. What's the acceptable quality trade-off for INT8 quantization?
2. Should we implement streaming first (UX) or INT8 first (performance)?
3. Do we need TensorRT for model (not just vocoder)?
4. Should we distill F5-TTS to smaller model?

---

**Status**: ✅ Phase 1 COMPLETE & EXCEEDED
**Next Milestone**: Phase 3 - INT8 Quantization (RTF < 0.20)
**Estimated Time**: 4-8 weeks for Phase 3 completion
**Confidence**: HIGH (solid foundation, clear path forward)

🚀 **Ready for Phase 3 Optimization!**