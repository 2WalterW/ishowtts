# Performance Test Log - 2025-09-30

## Latest Test Results

**Date**: 2025-09-30 (Late session)
**Test**: quick_performance_test.py (NFE=8, 3 runs)
**System**: Jetson AGX Orin, GPU locked (MAXN)

### Results
```
Run 1: 2.246s | Audio: 9.269s | RTF: 0.242 | Speedup: 4.13x
Run 2: 2.251s | Audio: 9.269s | RTF: 0.243 | Speedup: 4.12x
Run 3: 2.219s | Audio: 9.269s | RTF: 0.239 | Speedup: 4.18x

Average synthesis time: 2.238s
Average RTF: 0.241 ✅
Average speedup: 4.14x real-time
```

**Status**: ✅ **SUCCESS** - Achieved Whisper-level performance (RTF < 0.3)

### Comparison with Previous Tests

| Date | RTF (Mean) | RTF (Best) | Speedup | Notes |
|------|------------|------------|---------|-------|
| 2025-09-30 (latest) | **0.241** | 0.239 | 4.14x | NFE=8, GPU locked |
| 2025-09-30 (morning) | 0.278 | 0.274 | 3.59x | NFE=8, GPU locked |
| 2025-09-30 (initial) | 0.266 | 0.264 | 3.76x | NFE=8, GPU locked |

### Performance Improvement
- **13% faster** than morning test (0.241 vs 0.278)
- **9% faster** than initial test (0.241 vs 0.266)
- Likely due to system warmup and cache effects

### Optimizations Verified
- ✅ torch.compile(mode='max-autotune') enabled
- ✅ FP16 AMP with autocast
- ✅ Reference audio tensor caching
- ✅ NFE=8 steps
- ✅ GPU frequency locked (MAXN)
- ✅ CUDA streams for async operations

### System Configuration
- PyTorch: 2.5.0a0+872d972e41.nv24.08
- CUDA: 12.6
- Device: Orin (32GB)
- Power Mode: MAXN
- GPU Lock: Yes (jetson_clocks)

### Notes
- Fixed quick_performance_test.py to use NFE=8 (was using 16)
- All optimizations confirmed working
- Performance exceeds target (RTF < 0.3)
- Ready for Phase 2 optimizations (TensorRT, batching)

---

**Last Updated**: 2025-09-30 11:20 (UTC+8)
