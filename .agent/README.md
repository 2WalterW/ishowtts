# Agent Workspace

This directory contains optimization work, plans, and documentation for iShowTTS performance improvements.

## Files

### optimization_plan.md
Comprehensive optimization plan identifying bottlenecks and strategies for performance improvement.

### optimization_summary.md
**[MAIN DOCUMENT]** Complete summary of all optimizations applied, expected performance improvements, testing instructions, and rollback procedures.

### f5_tts_optimizations.md
Detailed documentation of Python F5-TTS optimizations (tensor caching, mixed precision AMP).

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

## Expected Results

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| NFE Steps | 32 | 16 | 2x faster |
| Synthesis Time | ~1.5s | ~0.5s | 3x faster |
| RTF | 0.7-1.0 | <0.3 | 2.5-3x |

## Next Steps

1. **Test**: Run benchmarks to verify improvements
2. **Quality Check**: A/B test audio quality
3. **Tune**: Adjust NFE steps if needed (8-32 range)
4. **Advanced**: Consider TensorRT vocoder for further speedup

## Notes

- Python changes in `third_party/F5-TTS` are not tracked in git (.gitignored)
- Config changes in `config/` are not tracked in git (.gitignored)
- All Rust optimizations are committed and pushed
- Benchmark scripts are ready to use