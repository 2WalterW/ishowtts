# Agent Workspace - iShowTTS Optimization

This directory contains optimization work, plans, and documentation for iShowTTS performance improvements.

## 🎯 Mission Status: Phase 1 COMPLETE ✅

**Current Performance**: RTF = 0.241 (Mean), 0.239 (Best) ✅
**Target**: RTF < 0.3 (Whisper-level speed)
**Speedup**: 4.14x real-time ✅
**Status**: Production ready, Phase 2 planning complete

---

## 📁 Key Documents (START HERE)

### 🌟 For Next Session
1. **[PHASE2_IMPLEMENTATION_PLAN.md](PHASE2_IMPLEMENTATION_PLAN.md)** ⭐ **START HERE FOR PHASE 2**
   - Complete TensorRT vocoder integration plan
   - Code examples and step-by-step guide
   - Timeline: 2-3 weeks, Target: RTF < 0.20

2. **[MAINTENANCE_PLAN_2025_09_30.md](MAINTENANCE_PLAN_2025_09_30.md)** - Daily maintenance checklist

### 📊 Current Status
3. **[STATUS.md](STATUS.md)** - Quick status summary & metrics
4. **[PERFORMANCE_LOG_2025_09_30.md](PERFORMANCE_LOG_2025_09_30.md)** - Latest test results (RTF=0.241)
5. **[SESSION_2025_09_30_LATE.md](SESSION_2025_09_30_LATE.md)** - Latest session summary

### 📚 Phase 1 Documentation
6. **[FINAL_OPTIMIZATION_REPORT.md](FINAL_OPTIMIZATION_REPORT.md)** - Complete Phase 1 report
7. **[SESSION_2025_09_30.md](SESSION_2025_09_30.md)** - Morning session summary
8. **[OPTIMIZATION_ROADMAP_NEXT.md](OPTIMIZATION_ROADMAP_NEXT.md)** - Phase 2 overview

### 📜 Historical Documents
9. **[OPTIMIZATION_COMPLETE.md](OPTIMIZATION_COMPLETE.md)** - Previous completion summary
10. **[optimizations_latest.md](optimizations_latest.md)** - Latest optimizations applied
11. **[python_optimizations_applied.md](python_optimizations_applied.md)** - Python-specific changes
12. **[optimization_summary.md](optimization_summary.md)** - Mid-project summary
13. **[optimization_roadmap.md](optimization_roadmap.md)** - Implementation roadmap
14. **[optimization_plan.md](optimization_plan.md)** - Initial optimization strategy
15. **[f5_tts_optimizations.md](f5_tts_optimizations.md)** - F5-TTS specific optimizations

---

## 🚀 Quick Start Guide

### Daily Routine
```bash
# 1. Check GPU is locked (CRITICAL!)
sudo nvpmodel -q  # Should show "MAXN"
sudo jetson_clocks  # Run if not locked

# 2. Quick performance test
cd /ssd/ishowtts
/opt/miniforge3/envs/ishowtts/bin/python scripts/quick_performance_test.py
# Expected: RTF < 0.30 (target), RTF < 0.25 (excellent)

# 3. If performance degraded, troubleshoot:
# - Check GPU locked (step 1)
# - Check system load: htop
# - Check Python optimizations: see MAINTENANCE_PLAN
# - Check thermal throttling: nvidia-smi
```

### Start Phase 2 Work
```bash
# Read the implementation plan
cat .agent/PHASE2_IMPLEMENTATION_PLAN.md

# Start with TensorRT research
# 1. Check TensorRT version
dpkg -l | grep tensorrt

# 2. Study Vocos architecture
# 3. Create ONNX export script (see PHASE2 plan)
# 4. Follow step-by-step guide in PHASE2_IMPLEMENTATION_PLAN.md
```

---

## 📊 Performance History

| Date | Session | RTF (Mean) | RTF (Best) | Speedup | Status |
|------|---------|------------|------------|---------|--------|
| 2025-09-30 | Late | **0.241** | **0.239** | 4.14x | ✅ Best |
| 2025-09-30 | Morning | 0.278 | 0.274 | 3.59x | ✅ Good |
| 2025-09-30 | Initial | 0.266 | 0.264 | 3.76x | ✅ Good |
| Baseline | Before | 1.322 | - | 0.76x | ❌ Slow |

**Improvement**: 5.5x faster than baseline (RTF 1.322 → 0.241)

---

## 🔧 Phase 1 Optimizations Applied ✅

### Python (F5-TTS)
- ✅ torch.compile(mode='max-autotune') for model and vocoder
- ✅ Automatic mixed precision (FP16 AMP)
- ✅ Reference audio tensor caching
- ✅ CUDA streams for async operations
- ✅ GPU memory management
- ✅ RMS variable bug fix (critical for torch.compile)

### Rust (TTS Engine)
- ✅ Optimized WAV encoding (no Cursor overhead)
- ✅ Fast linear resampling (f32, unsafe get_unchecked)
- ✅ Configurable NFE steps

### Configuration
- ✅ NFE steps: 32 → 8 (critical for target achievement)
- ✅ GPU frequency locking (jetson_clocks)

### Test Scripts
- ✅ quick_performance_test.py (NFE=8, 3 runs)
- ✅ test_max_autotune.py (NFE=8, 5 runs)
- ✅ test_nfe_performance.py (NFE comparison)
- ✅ setup_performance_mode.sh (GPU locking)

---

## 🎯 Phase 2 Roadmap (Next Work)

### Priority 1: TensorRT Vocoder (HIGHEST IMPACT)
**Timeline**: 2-3 weeks
**Expected**: RTF 0.241 → 0.15-0.20 (35-40% faster)

**Steps**:
1. Research & Preparation (2-3 days)
2. ONNX Export (3-5 days)
3. TensorRT Conversion (2-3 days)
4. Python Integration (5-7 days)
5. Testing & Validation (3-5 days)
6. Documentation (1-2 days)

**Detailed Plan**: See [PHASE2_IMPLEMENTATION_PLAN.md](PHASE2_IMPLEMENTATION_PLAN.md)

### Priority 2: E2E Testing (1 week)
- Basic TTS flow tests
- Danmaku integration tests
- Concurrent request tests
- Performance regression tests

### Priority 3: Batch Processing (1-2 weeks)
- Queue batching in Rust
- Batch inference in Python
- Expected: 2-3x throughput improvement

### Optional: INT8 Quantization (LOW PRIORITY)
- Only if more speed needed
- High risk (quality degradation)
- Expected: Additional 20-30%

---

## ⚠️ Critical Reminders

### GPU Frequency Lock (MUST DO AFTER REBOOT!)
```bash
sudo jetson_clocks
sudo nvpmodel -m 0
```

**Impact if not locked**:
- Performance degrades to RTF ~0.35 (vs 0.24)
- High variance (±16% vs ±2%)
- Unpredictable latency

### Python Optimizations Location (NOT IN GIT!)
**Critical**: Python files are in third_party submodule, not tracked in git!

**Optimized files**:
```
third_party/F5-TTS/src/f5_tts/api.py
third_party/F5-TTS/src/f5_tts/infer/utils_infer.py
```

**Backups**:
```
.agent/backups/optimized_python_files/api.py.optimized
.agent/backups/optimized_python_files/utils_infer.py.optimized
```

**To restore**:
```bash
cp .agent/backups/optimized_python_files/*.optimized third_party/F5-TTS/src/f5_tts/
# Then rename files (remove .optimized extension)
```

### Configuration (NOT IN GIT!)
**File**: `config/ishowtts.toml`
**Critical setting**: `default_nfe_step = 8`

---

## 🧪 Testing Commands

### Quick Test (30 seconds)
```bash
/opt/miniforge3/envs/ishowtts/bin/python scripts/quick_performance_test.py
# Expected: RTF < 0.30 (target), RTF < 0.25 (excellent)
```

### Full Test (60 seconds)
```bash
/opt/miniforge3/envs/ishowtts/bin/python scripts/test_max_autotune.py
# Expected: Mean RTF < 0.30, Best RTF < 0.25
```

### NFE Comparison (5 minutes)
```bash
/opt/miniforge3/envs/ishowtts/bin/python scripts/test_nfe_performance.py
# Tests NFE=8,12,16,20,24,32 and compares performance
```

### Benchmark (Extended)
```bash
/opt/miniforge3/envs/ishowtts/bin/python scripts/benchmark_tts_performance.py
# Comprehensive benchmark with multiple voice IDs
```

---

## 📈 Success Metrics

### Phase 1 (Current) ✅
- [x] RTF < 0.30: **0.241** ✅
- [x] Variance < 10%: **±2%** ✅
- [x] Speedup > 2.8x: **4.14x** ✅
- [x] Documentation complete ✅
- [x] All optimizations tested ✅

### Phase 2 (Targets)
- [ ] RTF < 0.20 (with TensorRT vocoder)
- [ ] Throughput > 10 req/min (with batching)
- [ ] E2E tests passing
- [ ] Performance regression tests in CI
- [ ] Production monitoring setup

---

## 🐛 Troubleshooting

### Performance Degraded (RTF > 0.35)
1. **Check GPU lock**: `sudo jetson_clocks && sudo nvpmodel -m 0`
2. **Check system load**: `htop` (should be < 6.0)
3. **Check thermal**: `nvidia-smi` (temp < 85°C)
4. **Restart backend**: `pkill ishowtts-backend && ./scripts/start_all.sh`
5. **Verify Python optimizations**: Compare with backups

### Test Script Errors
1. **Wrong Python**: Use `/opt/miniforge3/envs/ishowtts/bin/python`
2. **Missing dependencies**: `source /opt/miniforge3/envs/ishowtts/bin/activate && pip install -r requirements.txt`
3. **CUDA errors**: Check GPU lock and thermal throttling

### TTS Quality Issues
1. **Check NFE**: Should be 8 (speed) or 16+ (quality)
2. **Check reference audio**: Should be high-quality, >3s
3. **Check text**: Chinese text needs proper encoding

---

## 📚 Additional Resources

### Documentation
- Main README: `../README.md`
- Config examples: `../config/ishowtts.toml`
- Scripts: `../scripts/`

### Git Repository
- Remote: https://github.com/2WalterW/ishowtts.git
- Branch: main
- Latest commits: See git log

### External Resources
- F5-TTS: https://github.com/SWivid/F5-TTS
- TensorRT: https://docs.nvidia.com/deeplearning/tensorrt/
- Jetson Orin: https://developer.nvidia.com/embedded/jetson-agx-orin

---

## 🎉 Summary

**Phase 1**: ✅ **COMPLETE** (RTF=0.241, target achieved)
**Phase 2**: 📋 **PLANNED** (TensorRT vocoder, target RTF<0.20)
**Repository**: 🚀 **PRODUCTION READY**

**Next Action**: Start Phase 2 - TensorRT Vocoder Integration
**Read**: [PHASE2_IMPLEMENTATION_PLAN.md](PHASE2_IMPLEMENTATION_PLAN.md)

---

**Last Updated**: 2025-09-30 11:35 (UTC+8)
**Agent**: Performance Optimization & Maintenance
**Status**: Ready for Phase 2 Work
