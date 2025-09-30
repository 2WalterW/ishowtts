# iShowTTS Quick Reference - Performance Optimization

**Last Updated**: 2025-09-30 Evening
**Current Status**: ✅ Phase 3+ Optimizations Applied

---

## 🎯 Current Performance

| Metric | Value | Target | Status |
|--------|-------|--------|--------|
| **Mean RTF** | 0.169 → **0.143-0.157** (estimated) | < 0.20 | ✅ EXCEEDED |
| **Best RTF** | 0.165 → **~0.140** (estimated) | < 0.20 | ✅ EXCEEDED |
| **Speedup** | 5.92x → **6.4-7.0x** (estimated) | > 3.3x | ✅ EXCEEDED |
| **Variance** | ±5.6% | < 10% | ✅ MET |

**Note**: New metrics need validation testing (see Testing section below)

---

## 🔧 Applied Optimizations

### Phase 1-3 (Completed)
1. ✅ torch.compile(mode='max-autotune')
2. ✅ Automatic Mixed Precision (FP16)
3. ✅ Reference audio tensor caching
4. ✅ CUDA stream async operations
5. ✅ NFE reduced to 7
6. ✅ GPU frequency locking
7. ✅ Skip unnecessary spectrogram generation

### Phase 3+ Evening Session (New)
8. ✅ FP16 consistency through vocoder (5-10% speedup)
9. ✅ Remove torch.cuda.empty_cache() overhead (2-5% speedup)
10. ✅ Fix RMS caching correctness

**Total Estimated Improvement**: RTF 0.169 → 0.143-0.157 (10-15% speedup)

---

## 🧪 Testing & Validation

### Run Quick Test
```bash
# Ensure GPU locked to max performance
sudo jetson_clocks

# Run FP16 optimization test
python scripts/test_fp16_optimization.py
```

### Run Extended Performance Test
```bash
# 20 runs with detailed statistics
python scripts/extended_performance_test.py
```

### Expected Results
- **RTF < 0.16**: ✅ Excellent!
- **RTF 0.16-0.17**: ✅ Good progress
- **RTF > 0.17**: ⚠️ Investigate (may need GPU lock)

---

## 📁 Important Files

### Documentation
- `.agent/STATUS.md` - Overall status and metrics
- `.agent/OPTIMIZATION_SESSION_2025_09_30_EVENING.md` - Latest session details
- `.agent/analysis_2025_09_30.md` - Code analysis and findings
- `.agent/LONG_TERM_ROADMAP.md` - Future optimization plans

### Code Changes
- `.agent/optimizations_2025_09_30.patch` - Patch file for F5-TTS
- `third_party/F5-TTS/src/f5_tts/infer/utils_infer.py` - Main optimization file
- `third_party/F5-TTS/src/f5_tts/api.py` - Model loading and compilation

### Testing
- `scripts/test_fp16_optimization.py` - Validation script
- `scripts/extended_performance_test.py` - Comprehensive benchmarking
- `scripts/quick_performance_test.py` - Fast performance check

---

## 🚀 Quick Start Commands

### Apply Optimizations (if needed)
```bash
cd third_party/F5-TTS
git apply ../../.agent/optimizations_2025_09_30.patch
```

### Lock GPU to Max Performance
```bash
sudo jetson_clocks
sudo nvpmodel -m 0
```

### Run Backend with Optimizations
```bash
# Activate environment
source /opt/miniforge3/envs/ishowtts/bin/activate

# Start backend with warmup
cargo run -p ishowtts-backend -- --config config/ishowtts.toml --warmup
```

### Monitor Performance
```bash
# Watch GPU utilization
watch -n 1 nvidia-smi

# Check performance metrics
python scripts/monitor_performance.py

# Detect regressions
python scripts/detect_regression.py
```

---

## ⚠️ Important Notes

### GPU Performance Lock
**CRITICAL**: GPU frequency must be locked after every reboot!
```bash
sudo jetson_clocks
```

Without this, performance drops by ~50% (RTF 0.169 → 0.352)

### Third-Party Code
The optimizations are in `third_party/F5-TTS/`, which is a git submodule:
- Changes are NOT tracked in main repo
- Apply patch manually after F5-TTS updates
- Keep patch file in `.agent/` directory

### Configuration
**NFE Steps**: Set to 7 in `config/ishowtts.toml`
```toml
[f5]
default_nfe_step = 7  # Don't change unless testing
```

---

## 🔍 Troubleshooting

### Performance Degradation
1. **Check GPU lock**: `sudo jetson_clocks`
2. **Check power mode**: `sudo nvpmodel -m 0`
3. **Run regression test**: `python scripts/detect_regression.py`
4. **Clear cache and restart**: Reboot system

### Quality Issues
1. **Check FP16**: Vocoder should be in FP16 context
2. **Verify NFE=7**: May need NFE=8 for better quality
3. **RMS consistency**: Ensure cache is working correctly

### Memory Issues
1. **Check memory**: `nvidia-smi` (should have plenty on Orin 32GB)
2. **Clear Python cache**: Restart Python process
3. **NOT recommended**: Re-add `torch.cuda.empty_cache()` only if OOM

---

## 📊 Benchmarking

### Quick Benchmark (1 run)
```bash
time python scripts/quick_performance_test.py
```

### Extended Benchmark (20 runs)
```bash
python scripts/extended_performance_test.py > .agent/performance_results_new.txt
```

### Compare Results
```bash
diff .agent/performance_results_extended.txt .agent/performance_results_new.txt
```

---

## 🛠️ Maintenance Schedule

### Daily
- ✅ Run `python scripts/detect_regression.py` (1 min)
- ✅ Check GPU lock: `sudo jetson_clocks` if needed

### Weekly
- ✅ Run extended performance test (5 min)
- ✅ Review `.agent/performance_log.json` for trends
- ✅ Check for quality degradation reports

### After Updates
- ✅ Re-apply optimization patch to F5-TTS
- ✅ Validate performance with extended tests
- ✅ Update baseline: `python scripts/detect_regression.py --update-baseline`

---

## 🎯 Next Optimization Targets

### If RTF < 0.15 Needed

1. **NFE=6** (High Priority)
   - Potential: 14% speedup → RTF ~0.122-0.135
   - Risk: Quality degradation
   - Effort: Quality testing required

2. **INT8 Quantization** (Medium Priority)
   - Potential: 1.5-2x speedup → RTF ~0.072-0.105
   - Risk: Quality degradation, complex
   - Effort: 2-4 weeks

3. **CUDA Graphs** (Low Priority)
   - Potential: 10-15% speedup for fixed shapes
   - Risk: Complex, limited applicability
   - Effort: 1-2 weeks

### UX Improvements (No RTF change)

4. **Streaming Inference**
   - Benefit: Lower perceived latency
   - No RTF improvement but better UX

5. **Batch Processing**
   - Benefit: Higher throughput during peaks
   - Better GPU utilization

---

**Status**: ✅ READY FOR TESTING
**Last Optimization**: 2025-09-30 Evening
**Next Review**: After validation testing

---

*Quick reference for iShowTTS performance optimization and maintenance*