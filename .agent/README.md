# Agent Workspace - iShowTTS Optimization

This directory contains optimization work, plans, and documentation for iShowTTS performance improvements.

## 🎯 Mission Status: Phase 3 at 96.5% ⏳

**Current Performance**: RTF = 0.213 (Mean), 0.209 (Best) ✅
**Target**: RTF < 0.20 (Phase 3)
**Speedup**: 6.2x real-time ✅
**Status**: Production ready, NFE=6 evaluation pending
**Next**: Quality evaluation of NFE=6 samples (52 files ready)

---

## 📁 Key Documents (START HERE)

### 🌟 Current Session (2025-09-30 Maintenance)
1. **[CURRENT_SESSION_2025_09_30.md](CURRENT_SESSION_2025_09_30.md)** ⭐ **START HERE**
   - Comprehensive current status (NFE=7, RTF 0.213)
   - NFE=6 evaluation status (52 samples ready)
   - Decision matrix and next steps
   - Maintenance checklist and quick commands

2. **[MAINTENANCE_SESSION_2025_09_30.md](MAINTENANCE_SESSION_2025_09_30.md)** - Latest session report
   - Maintenance tools created (monitor_performance.py, quick_status.sh)
   - Deployment checklist for NFE=6
   - Monitoring strategy and regression detection

### 📊 Current Status
3. **[STATUS.md](STATUS.md)** - Quick status summary & performance metrics
4. **[QUICK_SUMMARY_2025_09_30.md](QUICK_SUMMARY_2025_09_30.md)** - Quick reference guide
5. **[OPTIMIZATION_NEXT_STEPS.md](OPTIMIZATION_NEXT_STEPS.md)** - Decision matrix for NFE=6

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

| Phase | NFE | RTF (Mean) | RTF (Best) | Speedup | Status |
|-------|-----|------------|------------|---------|--------|
| **Phase 3** | **7** | **0.213** | **0.209** | 6.2x | ⏳ **Current** |
| Phase 1 | 8 | 0.243 | 0.239 | 5.4x | ✅ Complete |
| Early | 8 | 0.266 | 0.264 | 5.0x | ✅ Complete |
| Baseline | 32 | 1.322 | - | 0.76x | ❌ Slow |
| **Phase 3+** | **6** | **~0.187** | **~0.182** | 7.1x | 🔬 **Testing** |

**Improvement**: 6.2x faster than baseline (RTF 1.322 → 0.213)

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
- ✅ NFE steps: 32 → 8 → 7 (Phase 3 tuning)
- ✅ GPU frequency locking (jetson_clocks)

### Test Scripts
- ✅ quick_performance_test.py (NFE=8, 3 runs)
- ✅ test_max_autotune.py (NFE=8, 5 runs)
- ✅ test_nfe_performance.py (NFE comparison)
- ✅ validate_nfe7.py (NFE=7 validation)
- ✅ test_nfe6_quality.py (NFE=6 quality samples)
- ✅ monitor_performance.py (automated regression detection) **NEW**
- ✅ quick_status.sh (one-command status check) **NEW**
- ✅ setup_performance_mode.sh (GPU locking)

---

## 🎯 Current Focus: NFE=6 Quality Evaluation

### Immediate (High Priority)
**Action**: Evaluate 52 quality samples (NFE=6 vs NFE=7)
**Location**: `.agent/quality_samples/nfe6_vs_nfe7_20250930_124505/`
**Expected RTF**: ~0.187 (14% faster, would exceed Phase 3 target)

**Decision Matrix**:
- IF quality acceptable → Deploy NFE=6, Phase 3 complete
- IF quality marginal → Keep NFE=7, accept 96.5% completion
- ELSE → Pursue Phase 4 (INT8 quantization)

### Phase 2 Status: TensorRT Investigation Complete ✅
**Result**: TensorRT vocoder NOT recommended
- Isolated: 1.96x faster (5.80ms → 2.96ms)
- End-to-end: 16% SLOWER (RTF 0.251 → 0.292)
- **Recommendation**: Use PyTorch + torch.compile

### Phase 4 Options (After NFE=6 Decision)

**Option A: INT8 Quantization** (if NFE=6 rejected)
- Expected RTF: 0.14-0.16 (25-35% faster)
- Timeline: 2-4 weeks
- Risk: Medium (quality sensitive)

**Option B: Streaming Inference** (parallel work)
- Expected: Time-to-first-audio reduced by 50-70%
- Timeline: 2-3 weeks
- Risk: Low (doesn't affect RTF)

**Option C: Batch Processing** (throughput)
- Expected: Better GPU utilization
- Timeline: 1-2 weeks
- Risk: Low

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
**Critical setting**: `default_nfe_step = 7` (Phase 3)
**Testing**: NFE=6 for potential final optimization

---

## 🧪 Testing Commands

### Quick Status Check (5 seconds)
```bash
./scripts/quick_status.sh
# Shows GPU lock, config, performance, and service status
```

### Quick Test (30 seconds)
```bash
/opt/miniforge3/envs/ishowtts/bin/python scripts/validate_nfe7.py
# Expected: RTF ≈ 0.213 (current performance)
```

### Full Test (60 seconds)
```bash
/opt/miniforge3/envs/ishowtts/bin/python scripts/test_max_autotune.py
# Expected: Mean RTF < 0.25, Best RTF < 0.22
```

### Performance Monitoring (regression detection)
```bash
/opt/miniforge3/envs/ishowtts/bin/python scripts/monitor_performance.py
# Automated regression detection with historical comparison
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

### Phase 1 ✅ COMPLETE
- [x] RTF < 0.30: **0.243** ✅
- [x] Variance < 10%: **±3%** ✅
- [x] Speedup > 3.3x: **5.4x** ✅
- [x] Documentation complete ✅
- [x] All optimizations tested ✅

### Phase 2 ✅ INVESTIGATED
- [x] TensorRT vocoder tested (not recommended)
- [x] End-to-end comparison complete
- [x] Decision: Keep PyTorch + torch.compile

### Phase 3 ⏳ 96.5% COMPLETE
- [x] RTF < 0.22: **0.213** (mean) ✅
- [x] RTF < 0.21: **0.209** (best) ✅
- [ ] RTF < 0.20: Pending NFE=6 evaluation
- [x] Variance < 5%: **±3.0%** ✅
- [x] Speedup > 6x: **6.2x** ✅
- [x] Quality good ✅
- [x] Monitoring tools created ✅

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

**Phase 1**: ✅ **COMPLETE** (RTF=0.243, target <0.3 achieved)
**Phase 2**: ✅ **INVESTIGATED** (TensorRT not recommended, PyTorch better)
**Phase 3**: ⏳ **96.5% COMPLETE** (RTF=0.213, target <0.20)
**Repository**: 🚀 **PRODUCTION READY** with monitoring tools

**Next Action**: Evaluate NFE=6 quality samples (52 files ready)
**Decision**: Accept NFE=6 (RTF ~0.187) OR keep NFE=7 (RTF 0.213)
**Read**: [CURRENT_SESSION_2025_09_30.md](CURRENT_SESSION_2025_09_30.md)

---

**Last Updated**: 2025-09-30 (Maintenance Session)
**Agent**: Repository Maintainer & Performance Optimizer
**Status**: Ready for NFE=6 evaluation and final Phase 3 decision
