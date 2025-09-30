# Agent Workspace - iShowTTS Optimization

This directory contains optimization work, plans, and documentation for iShowTTS performance improvements.

## 🎯 Mission: Achieve Whisper-level TTS Speed (RTF < 0.3)

**Status**: ✅ **COMPLETE** (2025-09-30)
**Current Result**: RTF = 0.278 (Mean), 0.274 (Best) ✅
**Speedup**: 3.59x real-time ✅
**Phase**: Ready for Phase 2 optimizations (TensorRT, batching)

## 📁 Key Documents (Priority Order)

### ⭐ START HERE
1. **[MAINTENANCE_PLAN_2025_09_30.md](MAINTENANCE_PLAN_2025_09_30.md)** - Complete maintenance & optimization guide
2. **[STATUS.md](STATUS.md)** - Quick status summary & metrics
3. **[OPTIMIZATION_ROADMAP_NEXT.md](OPTIMIZATION_ROADMAP_NEXT.md)** - Phase 2 detailed plans

### Current Status & Reports
- **[SESSION_2025_09_30.md](SESSION_2025_09_30.md)** - Latest session summary
- **[FINAL_OPTIMIZATION_REPORT.md](FINAL_OPTIMIZATION_REPORT.md)** - Complete Phase 1 report
- **[CURRENT_STATUS_2025_09_30.md](CURRENT_STATUS_2025_09_30.md)** - Troubleshooting guide

### Historical Documents
- **[OPTIMIZATION_COMPLETE.md](OPTIMIZATION_COMPLETE.md)** - Previous completion summary
- **[optimizations_latest.md](optimizations_latest.md)** - Latest optimizations applied
- **[python_optimizations_applied.md](python_optimizations_applied.md)** - Python-specific changes
- **[optimization_summary.md](optimization_summary.md)** - Mid-project summary
- **[optimization_roadmap.md](optimization_roadmap.md)** - Implementation roadmap
- **[optimization_plan.md](optimization_plan.md)** - Initial optimization strategy
- **[f5_tts_optimizations.md](f5_tts_optimizations.md)** - F5-TTS specific optimizations

## Quick Start

### 1. Build with Optimizations
```bash
cd /ssd/ishowtts
cargo build --release -p ishowtts-backend
```

### 2. Test Configuration
The config file has been updated with `default_nfe_step = 16` (was 32).
Location: `config/ishowtts.toml`

### 3. Run Benchmarks
```bash
# Start backend first
./scripts/start_all.sh

# In another terminal:
./scripts/benchmark_tts.sh

# Or test Python directly:
python3 scripts/test_optimizations.py
```

### 4. Monitor Performance
```bash
# Watch backend logs for timing info
RUST_LOG=ishowtts=debug cargo run --release -p ishowtts-backend
```

## Applied Optimizations

### Python (F5-TTS)
- ✅ Reference audio tensor caching
- ✅ Automatic mixed precision (FP16)
- ✅ Enhanced FP16 support for Jetson Orin

### Rust (TTS Engine)
- ✅ Optimized WAV encoding (no Cursor overhead)
- ✅ Fast linear resampling (f32, unsafe get_unchecked)
- ✅ Configurable NFE steps (default 16)

### Configuration
- ✅ Updated default_nfe_step from 32 to 16

## 📊 Performance Results

| Metric | Baseline | Optimized | Improvement |
|--------|----------|-----------|-------------|
| RTF | 1.322 | **0.266** | **5.0x faster** ✅ |
| Speedup | 0.76x | **3.76x** | **Target achieved** ✅ |
| NFE Steps | 32 | 8 | 4x fewer steps |
| Synthesis Time | 15.0s | 2.2s | **6.8x faster** |

### Key Changes
1. **torch.compile(mode='max-autotune')** - Changed from "reduce-overhead" (CRITICAL)
2. **NFE=8** - Reduced from 32 (CRITICAL)
3. **FP16 AMP on vocoder** - Extended autocast to vocoder (HIGH IMPACT)
4. **RMS bug fix** - Fixed closure issue for torch.compile (CRITICAL enabler)

## 🚀 Testing

### Run Performance Tests
```bash
# Quick test with final configuration (NFE=8, max-autotune)
/opt/miniforge3/envs/ishowtts/bin/python scripts/test_max_autotune.py

# Comprehensive NFE comparison (8, 12, 16, 20, 24, 32)
/opt/miniforge3/envs/ishowtts/bin/python scripts/test_nfe_performance.py

# Quick validation
/opt/miniforge3/envs/ishowtts/bin/python scripts/quick_performance_test.py
```

## 🎯 Future Optimizations (Optional)

For even more performance:
1. **TensorRT Vocoder** - 2-3x additional speedup possible
2. **INT8 Quantization** - 1.5-2x additional speedup
3. **Batch Processing** - Better throughput for multiple requests

Current performance (RTF=0.266) is sufficient for real-time livestream danmaku.

## Notes

- Python changes in `third_party/F5-TTS` are not tracked in git (.gitignored)
- Config changes in `config/` are not tracked in git (.gitignored)
- All Rust optimizations are committed and pushed
- Benchmark scripts are ready to use