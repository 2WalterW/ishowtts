# iShowTTS - Quick Start for Production

**Updated**: 2025-09-30
**Status**: Production Ready ✅
**Performance**: RTF 0.251 (5.3x from baseline)

---

## 🚀 Quick Start

### 1. Prerequisites

```bash
# Ensure you're on Jetson AGX Orin with:
# - PyTorch 2.5.0+ with CUDA support
# - Python 3.10+
# - Rust 1.76+
```

### 2. Environment Setup

```bash
# Activate Python environment
source /opt/miniforge3/envs/ishowtts/bin/activate

# Lock GPU to max performance (CRITICAL for consistent performance)
sudo jetson_clocks
sudo nvpmodel -m 0
```

### 3. Configuration

**File**: `config/ishowtts.toml`

```toml
[f5]
model = "F5TTS_v1_Base"
default_nfe_step = 8  # Optimized for speed (RTF < 0.3)

# DO NOT use TensorRT vocoder (PyTorch + torch.compile is faster)
# vocoder_local_path = "models/vocos_decoder.engine"  # Don't set this

[[f5.voices]]
id = "your_voice_id"
reference_audio = "/path/to/reference.wav"
reference_text = "Your reference text here"
language = "zh-CN"  # or "en"

default_voice = "your_voice_id"
```

### 4. Run

```bash
# Start backend + frontend
./scripts/start_all.sh --wait 900 --no-tail

# Or run with warmup (pre-compile torch.compile)
cargo run --release -p ishowtts-backend -- \
    --config config/ishowtts.toml \
    --warmup
```

### 5. Test

```bash
# Quick performance test
/opt/miniforge3/envs/ishowtts/bin/python scripts/test_max_autotune.py

# Expected output:
# Best RTF: ~0.25 (target < 0.3) ✅
# Speedup: ~4x real-time
```

---

## ⚙️ Configuration Details

### Optimizations (Already Applied)

These optimizations are **already enabled** in the codebase:

1. ✅ **torch.compile(mode='max-autotune')** - Automatic JIT compilation
2. ✅ **FP16 Automatic Mixed Precision** - Tensor Core acceleration
3. ✅ **Reference audio caching** - Avoid redundant preprocessing
4. ✅ **CUDA stream async ops** - Overlap CPU/GPU operations
5. ✅ **NFE=8** - Balanced speed/quality (configurable)

### Performance Settings

**NFE Steps** (Speed vs Quality):
- `8` - **Fastest** (RTF ~0.25) ← **Recommended for real-time**
- `12` - Balanced (RTF ~0.52)
- `16` - Better quality (RTF ~0.73)
- `32` - Best quality (RTF ~1.32)

**GPU Locking** (CRITICAL):
```bash
# Run after every reboot
sudo jetson_clocks
sudo nvpmodel -m 0

# Impact:
# Without: RTF 0.35 ± 16% variance
# With: RTF 0.25 ± 8% variance
```

---

## 📊 Expected Performance

### Benchmarks (Jetson AGX Orin)

| Metric | Value | Notes |
|--------|-------|-------|
| RTF (Best) | **0.251** | Target < 0.3 ✅ |
| RTF (Mean) | **0.297** | Still excellent |
| Speedup | **3.98x** | Real-time processing |
| Synthesis Time | **2.1s** | For 8s audio |
| First Inference | 30-60s | torch.compile warmup (one-time) |
| Quality | Good | Acceptable for streaming |
| GPU Memory | ~8GB | Fits in 32GB easily |

### Comparison to Baseline

- **Baseline RTF**: 1.32 (too slow)
- **Optimized RTF**: 0.251 (fast enough)
- **Total Speedup**: **5.3x faster** ✅

---

## 🚫 Common Mistakes

### ❌ Don't Use TensorRT Vocoder

```toml
# DON'T set this (slower end-to-end despite faster in isolation)
# vocoder_local_path = "models/vocos_decoder.engine"
```

**Why?**
- TensorRT vocoder: 1.96x faster in **isolation**
- But **slower end-to-end**: RTF 0.292 vs 0.251
- Reasons: Shape constraints, memory copies, torch.compile already excellent
- Recommendation: **Use PyTorch + torch.compile** (default)

### ❌ Don't Skip GPU Locking

```bash
# MUST run after every reboot
sudo jetson_clocks

# Without this: Performance variance ±16%
# With this: Performance variance ±8%
```

### ❌ Don't Set NFE Too Low

```toml
# NFE < 8 causes quality degradation
default_nfe_step = 8  # Minimum recommended
```

---

## 🔧 Troubleshooting

### Slow Performance (RTF > 0.3)

1. **Check GPU lock**:
   ```bash
   sudo jetson_clocks
   nvidia-smi  # Check GPU frequency
   ```

2. **Verify torch.compile working**:
   ```bash
   # Check logs for: "[F5TTS] torch.compile(mode='max-autotune') enabled"
   # First inference should be slow (30-60s), subsequent fast (<3s)
   ```

3. **Check NFE setting**:
   ```toml
   default_nfe_step = 8  # Not 16 or 32
   ```

### Quality Issues

1. **Increase NFE**:
   ```toml
   default_nfe_step = 12  # or 16
   # Trade-off: Slower (RTF ~0.52-0.73)
   ```

2. **Better reference audio**:
   - Use high-quality recording (no noise)
   - At least 3-5 seconds
   - Clear pronunciation

### Memory Issues

1. **Check GPU memory**:
   ```bash
   nvidia-smi
   # Should use ~8GB, have plenty left (24GB free on 32GB system)
   ```

2. **Reduce concurrent requests**:
   ```toml
   [api]
   max_parallel = 2  # Lower if memory constrained
   ```

---

## 📈 Monitoring

### Key Metrics

1. **RTF** (Real-Time Factor) - Target < 0.3
   - Check backend logs for synthesis times
   - Calculate: `synthesis_time / audio_duration`

2. **GPU Utilization** - Target 70-90%
   ```bash
   nvidia-smi -l 1
   ```

3. **GPU Memory** - Should be stable
   ```bash
   nvidia-smi -l 1 | grep MiB
   ```

4. **Error Rate** - Target < 0.1%
   - Monitor backend logs for errors
   - Check for torch.compile failures

### Performance Test

```bash
# Run periodic performance tests
/opt/miniforge3/envs/ishowtts/bin/python scripts/test_max_autotune.py

# Expected:
# Best RTF: 0.25-0.27 ✅
# Mean RTF: 0.29-0.31 ✅
# Variance: ±8% ✅
```

---

## 🎯 Production Checklist

### Before Deployment

- [ ] GPU locked with `jetson_clocks` ✅
- [ ] Config uses `default_nfe_step = 8` ✅
- [ ] PyTorch vocoder (NOT TensorRT) ✅
- [ ] Performance test passes (RTF < 0.3) ✅
- [ ] Reference audio high quality ✅
- [ ] Environment activated ✅

### After Deployment

- [ ] Monitor RTF in production logs
- [ ] Check GPU utilization (70-90%)
- [ ] Monitor error rate (< 0.1%)
- [ ] A/B test quality with users
- [ ] Set up automatic GPU locking on boot

---

## 📚 Documentation

### Quick Reference

- **This Guide**: `.agent/QUICK_START_PRODUCTION.md`
- **Full Status**: `.agent/STATUS.md`
- **Optimization Report**: `.agent/FINAL_OPTIMIZATION_REPORT.md`
- **Session Summary**: `.agent/SESSION_2025_09_30_FINAL.md`
- **Future Plans**: `.agent/ONGOING_OPTIMIZATION_PLAN.md`

### Scripts

- `scripts/test_max_autotune.py` - Performance validation
- `scripts/benchmark_vocoder.py` - Vocoder comparison
- `scripts/test_e2e_tensorrt.py` - TensorRT E2E test (reference)

---

## 🆘 Support

### Issues

1. Check troubleshooting section above
2. Review logs: `logs/backend.log`
3. Test with: `scripts/test_max_autotune.py`
4. Check GPU: `nvidia-smi`

### Performance Problems

If RTF > 0.3 consistently:
1. Verify GPU locked (`jetson_clocks`)
2. Check NFE setting (should be 8)
3. Ensure torch.compile working (check first inference slow)
4. Review system resources (other processes competing?)

---

## 🎉 Summary

**Best Configuration**:
- PyTorch vocoder + torch.compile ✅
- NFE = 8 ✅
- FP16 AMP ✅
- GPU locked ✅

**Performance**:
- RTF 0.251 (5.3x from baseline) ✅
- Production ready ✅
- Reliable and consistent ✅

**Recommendation**:
**Deploy this config, monitor, and enjoy fast TTS!** 🚀

---

**Last Updated**: 2025-09-30
**Status**: Production Ready ✅
**Maintainer**: Agent/Team