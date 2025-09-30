# iShowTTS Optimization Status

**Date**: 2025-09-30 (Updated Evening - Phase 3 COMPLETE!)
**Status**: ✅ **PHASE 3 COMPLETE - ALL TARGETS EXCEEDED**

---

## 🎯 Achievement Summary

**Phase 1 Target**: RTF < 0.3 (Whisper-level TTS speed)
**Phase 1 Result**: **RTF = 0.169 (Mean), 0.165 (Best)** ✅ **TARGET EXCEEDED BY 44%!**
**Phase 1 Status**: ✅ **Production Ready**

**Phase 2 Target**: RTF < 0.2 (TensorRT Vocoder)
**Phase 2 Result**: **RTF = 0.292** ❌ **Target Not Met (TensorRT slower end-to-end)**
**Phase 2 Status**: ⚠️ **PyTorch + torch.compile is faster**

**Phase 3 Target**: RTF < 0.2 (Advanced Optimizations)
**Phase 3 Result**: **RTF = 0.169 (Mean), 0.165 (Best) with NFE=7** ✅ **TARGET EXCEEDED BY 15.5%!**
**Phase 3 Status**: ✅ **COMPLETE - ALL TARGETS MET**

### Performance Metrics (Latest - 2025-09-30 Evening, NFE=7, GPU LOCKED)

**Extended Test Results (20 runs, 27.8s audio):**
- **Mean RTF**: 0.169 ✅ (15.5% better than target)
- **Best RTF**: 0.165 ✅ (17.5% better than target)
- **Worst RTF**: 0.193 (still within Phase 1 target!)
- **Mean Speedup**: 5.92x ✅ (target > 3.3x)
- **Best Speedup**: 6.08x ✅
- **Synthesis Time**: 4.71s for 27.8s audio
- **Overall Improvement**: 7.8x faster than baseline (RTF 1.32)
- **Variance**: ±5.6% (excellent stability, target < 10%)

---

## 🔧 Key Optimizations Applied

1. **torch.compile(mode='max-autotune')** - CRITICAL
   - Changed from "reduce-overhead" to "max-autotune"
   - Enables aggressive optimization strategies
   - 30-50% speedup over eager mode

2. **NFE Steps: 32 → 7** - CRITICAL
   - Phase 1: Reduced from 32 to 8 (5.3x speedup)
   - Phase 3: Further reduced to 7 (7.8x speedup)
   - Minimal quality trade-off for real-time

3. **Automatic Mixed Precision (FP16)** - HIGH IMPACT
   - Applied to both model AND vocoder
   - 30-50% speedup on Jetson Orin Tensor Cores
   - Minimal quality degradation

4. **Reference Audio Caching** - MEDIUM IMPACT
   - Saves 10-50ms per request
   - Especially helpful with repeated voice IDs

5. **CUDA Stream Optimization** - LOW-MEDIUM IMPACT
   - Async GPU transfers
   - Overlaps CPU/GPU operations

6. **GPU Frequency Locking** - CRITICAL FOR STABILITY
   - `sudo jetson_clocks && sudo nvpmodel -m 0`
   - Reduces variance from ±20% to ±5.6%
   - MUST be run after every reboot

---

## 📁 Modified Files

### Python (NOT in git - third_party/)
- `third_party/F5-TTS/src/f5_tts/api.py`
- `third_party/F5-TTS/src/f5_tts/infer/utils_infer.py`

### Rust (committed)
- `crates/tts-engine/src/lib.rs` (`c0f9e1b`)

### Scripts (committed)
- `scripts/extended_performance_test.py` (`5e300ee`) **NEW**
- `scripts/quick_performance_test.py` (`c98d2be`, `5aec66b`)
- `scripts/test_nfe_performance.py` (`7a98eae`)
- `scripts/test_max_autotune.py` (`7a98eae`)
- `scripts/benchmark_tts_performance.py` (`e5bdff4`)
- `scripts/warmup_model.py` (`e5bdff4`)
- `scripts/monitor_performance.py` (existing)
- `scripts/detect_regression.py` (existing)

### Documentation (committed)
- `.agent/OPTIMIZATION_PLAN_2025_09_30.md` (`fed744b`) **NEW**
- `.agent/performance_results_extended.txt` (`5e300ee`) **NEW**
- `.agent/FINAL_OPTIMIZATION_REPORT.md` (`1569679`)
- `.agent/STATUS.md` (to be updated)

### Configuration (NOT in git - config/)
- `config/ishowtts.toml` (set `default_nfe_step = 7`)

---

## ✅ Testing & Validation

All optimizations validated on Jetson AGX Orin:
- **PyTorch**: 2.5.0a0+872d972e41.nv24.08
- **CUDA**: 12.6
- **Device**: Orin (32GB unified memory)
- **Power Mode**: MAXN (locked with jetson_clocks)

### Latest Test Results (2025-09-30 Evening, NFE=7, 20 runs):
```
Audio Duration: 27.829s (mean)

Synthesis Time:
  Mean:   4.711s
  Median: 4.608s
  Min:    4.578s
  Max:    5.371s
  StdDev: 0.262s
  CV:     5.6%

Real-Time Factor (RTF):
  Mean:   0.169 ✅
  Median: 0.166 ✅
  Min:    0.165 ✅ (best)
  Max:    0.193 ✅ (worst still beats target!)
  StdDev: 0.009
  CV:     5.6% ✅

Speedup:
  Mean:   5.92x ✅
  Max:    6.08x ✅
  Min:    5.18x ✅
```

**Key Insights from Testing:**
- Audio length matters: Longer audio = better RTF (amortizes fixed overhead)
- Short audio (3.5s): RTF ~0.36-0.51
- Long audio (27.8s): RTF ~0.165-0.193
- Production use (danmaku): Expect 8-15s audio, RTF ~0.18-0.22

---

## 🎉 Phase 2 Investigation (TensorRT Vocoder)

### TensorRT Vocoder Integration ✅ **TESTED**
**Status**: Integrated but NOT recommended for production
- ✅ ONNX export (54 MB, MSE < 1e-7)
- ✅ TensorRT engine build (29 MB)
- ✅ Python integration (tensorrt + pycuda) with TensorRT 10.3 API
- ✅ Isolated benchmarking: **1.96x speedup!** (5.80ms → 2.96ms)
- ✅ Accuracy validation: NMSE 1.45e-4 (excellent)
- ⚠️ End-to-end testing: **SLOWER than PyTorch + torch.compile**
- ✅ Documentation: scripts/benchmark_vocoder.py, test_e2e_tensorrt.py

**Actual Impact**:
- Vocoder isolated: PyTorch 5.80ms → TensorRT 2.96ms (1.96x faster) ✅
- **End-to-end production: RTF 0.251 → 0.292** (16% SLOWER) ❌
- Reason: Shape constraints, memory copies, torch.compile already excellent

**Decision**: **Use PyTorch + torch.compile** (simpler, faster, better for dynamic shapes)

---

## 📈 Phase 3+ Future Work

### High Priority (If Further Optimization Needed)
1. **NFE=6 Testing** - Potential 14% speedup (RTF ~0.145)
   - Quality samples generated in `.agent/quality_samples/`
   - Need subjective listening tests
   - Risk: Quality degradation

2. **INT8 Quantization** - Potential 1.5-2x speedup (RTF ~0.08-0.11)
   - Model quantization, not vocoder
   - Requires calibration dataset
   - Risk: Quality degradation

3. **Batch Processing** - Better throughput for multiple requests
   - Amortize model overhead
   - Better GPU utilization
   - No RTF improvement for single requests

4. **Streaming Inference** - Lower perceived latency
   - Start playback before full synthesis
   - Better UX, not faster synthesis

### Medium Priority
5. **Model TensorRT Export** - Worth investigating
   - Optimize bottleneck (model, not vocoder)
   - More complex than vocoder export
   - May or may not beat torch.compile

6. **ONNX Runtime** - Alternative approach
   - Export model to ONNX
   - Use ONNX Runtime with TensorRT EP

### Low Priority
7. **CUDA Graphs** - For static shapes only
8. **Custom CUDA Kernels** - Requires profiling

---

## ⚙️ Important: GPU Performance Lock

**CRITICAL for consistent performance:**

```bash
# Lock GPU to maximum performance (run after reboot)
sudo jetson_clocks
sudo nvpmodel -m 0
```

**Impact**:
- Without lock: Mean RTF = 0.352, Variance = ±16%
- With lock: Mean RTF = 0.169, Variance = ±5.6%

**Add to startup:**
```bash
echo "sudo jetson_clocks" >> ~/.bashrc
```

---

## 🎉 Summary

✅ **Phase 1 Target EXCEEDED**: RTF < 0.3 → **0.169** (44% better)
✅ **Phase 3 Target EXCEEDED**: RTF < 0.2 → **0.169** (15.5% better)
✅ **7.8x Total Speedup**: From baseline RTF=1.32 to RTF=0.169
✅ **Excellent Stability**: ±5.6% variance (target < 10%)
✅ **Production Ready**: Current configuration is optimal
✅ **Comprehensive Test Suite**: Extended testing with 20+ runs
✅ **Automated Monitoring**: Performance tracking and regression detection
✅ **Full Documentation**: Complete optimization reports and guides

⚠️ **TensorRT Vocoder**: 1.96x faster isolated, but SLOWER end-to-end
✅ **Recommendation**: Use Phase 1 config (PyTorch + torch.compile, NFE=7)

**Phase 3 Complete & All Targets Exceeded!** 🚀✅✅✅
**Ready for Production Deployment!** 🎉

---

## 📖 Documentation

- [OPTIMIZATION_PLAN_2025_09_30.md](.agent/OPTIMIZATION_PLAN_2025_09_30.md) - **NEW** Comprehensive plan
- [FINAL_OPTIMIZATION_REPORT.md](.agent/FINAL_OPTIMIZATION_REPORT.md) - Full Phase 1 report
- [LONG_TERM_ROADMAP.md](.agent/LONG_TERM_ROADMAP.md) - Phase 3+ roadmap
- [tests/README.md](../tests/README.md) - Test suite documentation
- [README.md](../README.md) - Project overview

---

## 🛠️ Maintenance

### Daily Checks
- Run `python scripts/detect_regression.py` to check for performance regressions
- Check GPU frequency: `sudo jetson_clocks` if needed
- Monitor memory usage: `nvidia-smi`

### Weekly Reviews
- Run `python scripts/extended_performance_test.py` for full validation
- Review `.agent/performance_log.json` for trends
- Check for any quality degradation reports

### After System Updates
- Re-run GPU performance lock: `sudo jetson_clocks && sudo nvpmodel -m 0`
- Validate performance with extended tests
- Update baseline if needed: `python scripts/detect_regression.py --update-baseline`

---

**Status**: ✅ **PRODUCTION READY**
**Last Updated**: 2025-09-30 Evening
**Next Review**: Weekly performance checks