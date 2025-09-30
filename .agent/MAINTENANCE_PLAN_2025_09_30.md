# iShowTTS Maintenance & Optimization Plan
**Date**: 2025-09-30
**Agent**: Performance Optimization & Maintenance
**Status**: 🟢 Production Ready

---

## 🎯 Current Status

### Performance Metrics ✅
- **RTF**: 0.278 (mean), 0.274 (best) - **TARGET ACHIEVED** (< 0.3)
- **Speedup**: 3.59x real-time (3.65x best)
- **Consistency**: ±1.5% variance (with GPU locked)
- **Synthesis Time**: 2.3s for 8.4s audio

### System Status
- **Backend**: Running (PID 1829207)
- **Power Mode**: MAXN ✅
- **GPU Lock**: Applied via jetson_clocks
- **Configuration**: NFE=8, torch.compile='max-autotune', FP16 AMP

---

## 📋 Daily Maintenance Checklist

### Performance Validation (Daily)
```bash
# 1. Verify GPU is locked
sudo nvpmodel -q  # Should show "MAXN"

# 2. Check backend is running
ps aux | grep ishowtts-backend

# 3. Quick performance test (if needed)
cd /ssd/ishowtts
./.venv/bin/python scripts/quick_performance_test.py

# Expected: RTF < 0.35 (0.30 target, 0.35 warning threshold)
```

### System Health (Daily)
```bash
# 1. Check GPU temperature
nvidia-smi --query-gpu=temperature.gpu --format=csv,noheader

# 2. Check memory usage
free -h

# 3. Check disk space
df -h /ssd

# 4. Check log files
tail -n 50 logs/backend.log
```

### Performance Degradation Troubleshooting
If RTF > 0.35:
1. **Check GPU lock**: `sudo jetson_clocks && sudo nvpmodel -m 0`
2. **Check system load**: `htop` (should be < 6.0)
3. **Check thermal throttling**: `nvidia-smi` (temp should be < 85°C)
4. **Restart backend**: `pkill ishowtts-backend && ./scripts/start_all.sh`
5. **Verify Python optimizations**: See STATUS.md for file locations

---

## 🚀 Optimization Roadmap

### Phase 1: Complete ✅
All optimizations applied and validated:
- torch.compile(mode='max-autotune')
- FP16 AMP with vocoder
- Reference audio tensor caching
- CUDA stream optimization
- NFE=8 steps
- GPU frequency locking

### Phase 2: Advanced Optimizations (Priority Order)

#### 1. TensorRT Vocoder (HIGHEST PRIORITY)
**Expected Impact**: RTF 0.278 → 0.15-0.20 (40-50% faster)
**Effort**: Medium (1-2 weeks)
**Risk**: Low-Medium

**Steps**:
1. Export Vocos vocoder to ONNX
2. Convert ONNX to TensorRT engine with FP16
3. Integrate TensorRT inference into Python pipeline
4. Benchmark and validate quality
5. Update configuration and documentation

**Files to Create**:
- `scripts/export_vocoder_onnx.py`
- `scripts/convert_vocoder_tensorrt.sh`
- `scripts/benchmark_vocoder.py`
- `crates/tts-engine/src/tensorrt_wrapper.rs` (if needed)

**Success Criteria**:
- RTF < 0.20
- No quality degradation (MOS score maintained)
- Stable performance over 100+ runs

#### 2. Batch Processing (MEDIUM PRIORITY)
**Expected Impact**: 2-3x throughput for queued requests
**Effort**: Medium (1 week)
**Risk**: Medium

**Steps**:
1. Implement batch inference in Python
2. Add queue batching in Rust backend
3. Configure batch size and timeout
4. Test with concurrent requests

**Benefits**:
- Better GPU utilization
- Higher overall throughput
- Lower cost per request

**Success Criteria**:
- Process 4 requests in < 1.5x single request time
- No increase in single-request latency
- Stable under load testing

#### 3. E2E and Load Testing (MEDIUM PRIORITY)
**Expected Impact**: Stability and confidence
**Effort**: Low (3-5 days)
**Risk**: Low

**Steps**:
1. Create E2E integration tests
2. Create load testing suite
3. Add performance regression tests
4. Setup CI/CD for automated testing

**Files to Create**:
- `tests/test_e2e.py`
- `tests/test_load.py`
- `tests/test_performance_regression.py`
- `.github/workflows/performance.yml`

#### 4. Metrics and Monitoring (LOW PRIORITY)
**Expected Impact**: Better visibility and debugging
**Effort**: Low (2-3 days)
**Risk**: Low

**Steps**:
1. Add `/api/metrics` endpoint
2. Log RTF, latency, errors
3. Create dashboard or log analysis tools
4. Setup alerts for performance degradation

#### 5. INT8 Quantization (OPTIONAL - FUTURE)
**Expected Impact**: RTF 0.15 → 0.10-0.12 (if combined with TensorRT)
**Effort**: High (2-3 weeks)
**Risk**: High (quality degradation)

**Steps**:
1. Collect calibration dataset
2. Apply post-training quantization
3. Validate quality (may need QAT)
4. Benchmark performance

**Success Criteria**:
- RTF improvement > 20%
- MOS score drop < 0.2
- Pass subjective A/B testing

---

## 🔧 Technical Debt & Improvements

### Code Quality
- [ ] Add unit tests for Rust TTS engine
- [ ] Add integration tests for backend API
- [ ] Improve error handling and logging
- [ ] Add input validation for TTS parameters

### Documentation
- [x] Performance optimization report
- [x] Maintenance checklist (this document)
- [x] Phase 2 roadmap
- [ ] API documentation
- [ ] Deployment guide
- [ ] Troubleshooting guide

### Infrastructure
- [ ] Setup systemd service for backend
- [ ] Add auto-restart on failure
- [ ] Configure log rotation
- [ ] Setup monitoring/alerting
- [ ] Create backup/restore scripts

---

## 📊 Performance Targets

| Milestone | Current | Target | Timeline |
|-----------|---------|--------|----------|
| **Phase 1** | **0.278** | **< 0.30** | **✅ Complete** |
| Phase 2.1 (TensorRT) | 0.278 | < 0.20 | 2-4 weeks |
| Phase 2.2 (Batching) | - | 2-3x throughput | 4-6 weeks |
| Phase 2.3 (INT8) | - | < 0.15 | 8-12 weeks (optional) |

---

## 🚨 Critical Reminders

### After System Reboot
**CRITICAL**: GPU frequency lock resets on reboot!
```bash
sudo /ssd/ishowtts/scripts/setup_performance_mode.sh
```

Or manually:
```bash
sudo jetson_clocks
sudo nvpmodel -m 0
```

**Impact if not locked**:
- Performance degrades to RTF ~0.35 (vs 0.28)
- High variance (±16% vs ±1.5%)
- Unpredictable latency

### Before Major Updates
1. Backup current working state:
   ```bash
   cp -r /ssd/ishowtts/.agent/backups/optimized_python_files /ssd/ishowtts/.agent/backups/backup_$(date +%Y%m%d)
   ```

2. Document current performance:
   ```bash
   ./.venv/bin/python scripts/test_max_autotune.py > .agent/performance_$(date +%Y%m%d).log
   ```

3. Commit all changes before proceeding

### Python Optimizations Location
**IMPORTANT**: Python files are NOT in git (third_party submodule)

Optimized files:
- `third_party/F5-TTS/src/f5_tts/api.py` - torch.compile
- `third_party/F5-TTS/src/f5_tts/infer/utils_infer.py` - AMP, caching, streams

Backups:
- `.agent/backups/optimized_python_files/`

To restore optimizations:
```bash
cp .agent/backups/optimized_python_files/* third_party/F5-TTS/src/f5_tts/
```

---

## 📈 Performance Tracking

### Benchmark Command
```bash
# Quick test (3 runs, ~30s)
./.venv/bin/python scripts/quick_performance_test.py

# Full test (5 runs, ~60s)
./.venv/bin/python scripts/test_max_autotune.py

# NFE comparison (tests multiple NFE values)
./.venv/bin/python scripts/test_nfe_performance.py
```

### Acceptable Performance Ranges
| Metric | Good | Warning | Critical |
|--------|------|---------|----------|
| Mean RTF | < 0.30 | 0.30-0.35 | > 0.35 |
| Variance | < 5% | 5-10% | > 10% |
| GPU Temp | < 75°C | 75-85°C | > 85°C |
| System Load | < 6.0 | 6.0-8.0 | > 8.0 |

### Performance Log Format
```
Date: YYYY-MM-DD
RTF Mean: X.XXX
RTF Best: X.XXX
Variance: ±X.X%
GPU Locked: Yes/No
Notes: [any observations]
```

---

## 🔬 Experimentation Guidelines

### When to Optimize Further
Only if:
1. Current performance insufficient for use case
2. Clear bottleneck identified via profiling
3. Optimization has high expected ROI (>10% improvement)
4. Quality can be maintained or validated

### Optimization Process
1. **Measure baseline** - Run benchmarks 3-5 times
2. **Profile bottlenecks** - Use nsys, py-spy, or similar
3. **Implement optimization** - One change at a time
4. **Measure impact** - Compare to baseline
5. **Validate quality** - Listen to samples, check metrics
6. **Document results** - Update STATUS.md and commit

### Quality Validation
For any optimization that may affect quality:
1. Generate 10-20 diverse samples
2. Listen to all samples (subjective)
3. Run objective metrics (if available)
4. Compare with baseline samples
5. Document any quality changes

---

## 📚 Reference Documents

### Status & Reports
- `.agent/STATUS.md` - Quick status summary
- `.agent/FINAL_OPTIMIZATION_REPORT.md` - Complete Phase 1 report
- `.agent/OPTIMIZATION_ROADMAP_NEXT.md` - Detailed Phase 2 plans
- `.agent/SESSION_2025_09_30.md` - Latest session summary
- `.agent/MAINTENANCE_PLAN_2025_09_30.md` - This document

### Configuration
- `config/ishowtts.toml` - Backend configuration (NFE, paths, etc.)
- `config/danmaku_gateway.toml` - Queue and filter settings

### Scripts
- `scripts/setup_performance_mode.sh` - GPU locking
- `scripts/test_max_autotune.py` - Full benchmark (5 runs)
- `scripts/quick_performance_test.py` - Quick test (3 runs)
- `scripts/test_nfe_performance.py` - NFE comparison
- `scripts/benchmark_tts_performance.py` - Comprehensive benchmark

---

## 🎯 Next Session Checklist

For the next agent/developer session:

### Immediate Tasks
1. [ ] Run performance validation test
2. [ ] Check GPU lock status
3. [ ] Review logs for errors
4. [ ] Update this document with findings

### Short-term Goals (Next 2 Weeks)
1. [ ] Begin TensorRT vocoder export
2. [ ] Implement E2E tests
3. [ ] Add metrics endpoint
4. [ ] Setup systemd service

### Medium-term Goals (Next Month)
1. [ ] Complete TensorRT integration
2. [ ] Implement batch processing
3. [ ] Add load testing suite
4. [ ] Create monitoring dashboard

### Long-term Goals (Next Quarter)
1. [ ] Explore INT8 quantization
2. [ ] Investigate model distillation
3. [ ] Add streaming inference
4. [ ] Optimize for lower latency

---

## ✅ Success Metrics

### Phase 1 (Current) ✅
- [x] RTF < 0.30: **0.278** ✅
- [x] Variance < 10%: **±1.5%** ✅
- [x] Speedup > 2.8x: **3.59x** ✅
- [x] Documentation complete ✅
- [x] All optimizations tested ✅

### Phase 2 (Targets)
- [ ] RTF < 0.20 (with TensorRT vocoder)
- [ ] Throughput > 10 req/min (with batching)
- [ ] E2E tests passing
- [ ] Metrics endpoint live
- [ ] Production monitoring setup

---

## 🎉 Summary

**Current State**: Production-ready, target achieved
**Performance**: RTF 0.278 (3.59x real-time)
**Consistency**: Excellent (±1.5% variance)
**Next Priority**: TensorRT vocoder export (expected RTF < 0.20)

**Critical Reminder**: Run `sudo jetson_clocks` after every reboot!

---

**Last Updated**: 2025-09-30
**Next Review**: As needed for Phase 2 work