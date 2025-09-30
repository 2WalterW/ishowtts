# iShowTTS Maintenance & Optimization Guide

**Date**: 2025-09-30
**Maintainer**: Agent
**Status**: Production Ready (Phase 1 Complete)

---

## 🎯 Current Status Overview

### Performance Metrics (Production)
- **RTF**: 0.251 (best), 0.297 (mean) ✅
- **Target**: < 0.30 ✅ **ACHIEVED**
- **Speedup**: 3.98x (best), 3.37x (mean)
- **Total Improvement**: 5.3x from baseline (RTF 1.32)
- **Synthesis Time**: 2.1s for 8.4s audio
- **Variance**: ±8% (acceptable with GPU lock)

### Production Configuration
- **Model**: F5TTS_v1_Base
- **Vocoder**: PyTorch Vocos + torch.compile(mode='max-autotune')
- **NFE Steps**: 8 (speed/quality balance)
- **Precision**: FP16 AMP
- **TensorRT**: NOT recommended (PyTorch faster end-to-end)

---

## 📋 Daily Maintenance Tasks

### 1. Performance Monitoring

```bash
# Check GPU lock status (should be done after every reboot)
sudo jetson_clocks
sudo nvpmodel -m 0

# Monitor GPU utilization
watch -n 1 nvidia-smi

# Quick performance test
/opt/miniforge3/envs/ishowtts/bin/python scripts/test_max_autotune.py

# Check backend logs
tail -f logs/backend.log
```

### 2. Health Checks

**Every 24 hours:**
- [ ] Verify GPU frequency lock is active
- [ ] Check for torch.compile errors in logs
- [ ] Monitor memory usage (GPU and system)
- [ ] Verify RTF stays < 0.35 (with variance)
- [ ] Check for any failed synthesis requests

**Commands:**
```bash
# GPU frequency check
cat /sys/devices/gpu.0/devfreq/17000000.ga10b/cur_freq

# Memory check
nvidia-smi --query-gpu=memory.used,memory.free --format=csv

# Error check in logs
grep -i "error\|exception\|failed" logs/backend.log | tail -20
```

---

## 🔄 Weekly Maintenance Tasks

### 1. Performance Benchmarking

Run comprehensive benchmarks weekly to track trends:

```bash
cd /ssd/ishowtts
source /opt/miniforge3/envs/ishowtts/bin/activate

# Full benchmark suite
python scripts/test_max_autotune.py > logs/benchmark_$(date +%Y%m%d).log

# Vocoder comparison (if testing changes)
python scripts/benchmark_vocoder.py >> logs/vocoder_$(date +%Y%m%d).log

# Check for regressions
echo "Target: RTF < 0.30"
grep "Mean RTF" logs/benchmark_$(date +%Y%m%d).log
```

### 2. Log Rotation & Cleanup

```bash
# Archive old logs
mkdir -p logs/archive/$(date +%Y%m)
mv logs/backend.log logs/archive/$(date +%Y%m)/backend_$(date +%Y%m%d).log
mv logs/frontend.log logs/archive/$(date +%Y%m)/frontend_$(date +%Y%m%d).log

# Clean old archives (keep 3 months)
find logs/archive -type f -mtime +90 -delete
```

### 3. System Updates Check

```bash
# Check for PyTorch updates (CAREFUL - test before production)
/opt/miniforge3/envs/ishowtts/bin/pip list --outdated | grep torch

# Check Rust updates
rustup update --no-self-update

# NOTE: Always test in staging before updating production!
```

---

## 📦 Monthly Maintenance Tasks

### 1. Full System Audit

**Performance audit:**
```bash
# Run extended benchmarks (10+ runs)
python scripts/test_nfe_performance.py

# Profile for bottlenecks
python -m torch.utils.bottleneck scripts/test_max_autotune.py

# Analyze results
cat logs/benchmark_*.log | grep "Mean RTF" | sort
```

**Memory leak check:**
```bash
# Monitor memory over 24 hours
while true; do
  echo "$(date): $(nvidia-smi --query-gpu=memory.used --format=csv,noheader)"
  sleep 3600
done >> logs/memory_tracking.log
```

### 2. Dependency Updates (Careful!)

**Update F5-TTS:**
```bash
cd third_party/F5-TTS
git fetch origin
git log HEAD..origin/main  # Check what changed

# If safe to update:
git pull
cd ../../

# Re-apply optimizations (Python files not tracked!)
# Copy from .agent/backups/ or re-apply manually
```

**Update PyTorch (Jetson-specific):**
```bash
# Check NVIDIA's latest PyTorch for Jetson
# https://forums.developer.nvidia.com/t/pytorch-for-jetson/72048

# NEVER use pip install torch --upgrade (will break CUDA)
# Must use NVIDIA's pre-built wheels
```

### 3. Quality Assurance

**A/B Testing:**
```bash
# Generate samples with current config (NFE=8)
# Compare with baseline (NFE=32)
# Subjective listening test

# Objective metrics
python scripts/test_quality.py  # TODO: Create this script
```

---

## 🚨 Incident Response

### Issue: High RTF (>0.35 consistently)

**Diagnosis:**
```bash
# 1. Check GPU lock
cat /sys/devices/gpu.0/devfreq/17000000.ga10b/cur_freq
# Should be: 1300500000 (max frequency)

# 2. Check GPU utilization
nvidia-smi
# Should be: 70-90% during synthesis

# 3. Check thermal throttling
cat /sys/devices/virtual/thermal/thermal_zone*/temp
# Should be: <80°C

# 4. Check torch.compile
grep "torch.compile" logs/backend.log
# Should see: "Compiling model..." at startup
```

**Solutions:**
1. Re-apply GPU lock: `sudo jetson_clocks`
2. Restart backend if torch.compile failed
3. Check for thermal issues (clean heatsink, improve airflow)
4. Verify NFE=8 in config

---

### Issue: Quality Degradation

**Diagnosis:**
```bash
# Check current NFE setting
grep "default_nfe_step" config/ishowtts.toml

# Check for FP16 precision issues
grep "autocast" third_party/F5-TTS/src/f5_tts/infer/utils_infer.py
```

**Solutions:**
1. If quality is critical: Increase NFE to 12 or 16
2. Disable FP16 AMP (slower but more accurate)
3. Test with baseline config (NFE=32, FP32)
4. Check reference audio quality (≥3s, clear, 24kHz+)

---

### Issue: torch.compile Failures

**Symptoms:**
- Errors mentioning "graph break", "closure", "dynamic shape"
- Slower performance than expected
- Backend crashes on startup

**Diagnosis:**
```bash
# Enable debug logging
TORCH_LOGS="+dynamo" RUST_LOG=debug cargo run -p ishowtts-backend

# Check for compilation errors
grep -A5 "torch.compile" logs/backend.log
```

**Solutions:**
1. **Temporary**: Disable torch.compile (edit api.py, comment lines 88-94)
2. **Investigate**: Check for upstream F5-TTS changes
3. **Rollback**: Use backups in `.agent/backups/`
4. **Report**: File issue if new PyTorch regression

---

### Issue: Memory Leaks

**Symptoms:**
- GPU memory increasing over time
- OOM errors after hours of operation
- Slower performance after many requests

**Diagnosis:**
```bash
# Track memory over time
watch -n 60 nvidia-smi

# Check Python memory
python -c "import torch; print(torch.cuda.memory_summary())"
```

**Solutions:**
1. Restart backend daily (cronjob)
2. Check for tensor cache issues
3. Verify `torch.cuda.empty_cache()` is called
4. Monitor for PyTorch memory leak bugs

---

## 🔧 Configuration Management

### Current Best Config

**File**: `config/ishowtts.toml`

```toml
[f5]
model = "F5TTS_v1_Base"
default_nfe_step = 8  # Best speed/quality balance

# Do NOT enable TensorRT vocoder (PyTorch is faster)
# vocoder_local_path = "models/vocos_decoder.engine"  # Don't use

[api]
max_parallel = 3  # Adjust based on GPU memory

[shimmy]
model_name = "f5-tts-ishow"
ctx_len = 256
```

### Config Tuning Guidelines

**NFE Steps:**
- NFE=8: RTF ~0.27, Good quality, **Recommended for production**
- NFE=12: RTF ~0.52, Better quality, Use if quality issues
- NFE=16: RTF ~0.73, Excellent quality, Use for pre-recorded
- NFE=32: RTF ~1.32, Best quality, Use for archival/studio

**Parallel Requests:**
- max_parallel=1: RTF ~0.25, Low memory, Sequential
- max_parallel=3: RTF ~0.30, Medium memory, **Recommended**
- max_parallel=5: RTF ~0.40+, High memory, Better throughput

**Quality vs Speed Matrix:**
```
         Speed       Quality     Use Case
NFE=8    ★★★★★      ★★★☆☆      Live streaming (current)
NFE=12   ★★★★☆      ★★★★☆      Balanced
NFE=16   ★★★☆☆      ★★★★★      Pre-recorded content
NFE=32   ★★☆☆☆      ★★★★★★     Archival/reference
```

---

## 🚀 Phase 3 Optimization Roadmap

### Priority 1: INT8 Quantization (Estimated RTF: 0.15-0.18)

**Goal**: 1.5-2x additional speedup with minimal quality loss

**Investigation Steps:**
1. Profile model to identify quantization candidates
2. Test PyTorch Quantization API vs TensorRT INT8
3. Calibrate with representative dataset
4. A/B test quality (target: <5% quality loss)
5. Benchmark performance

**Files to create:**
- `scripts/quantize_model.py` - Quantization script
- `scripts/test_int8_quality.py` - Quality validation
- `scripts/benchmark_int8.py` - Performance comparison

**Estimated effort**: 1-2 weeks
**Risk**: Medium (quality degradation)

---

### Priority 2: Batch Processing (Goal: 2-3x throughput)

**Goal**: Process multiple danmaku messages in parallel for better GPU utilization

**Implementation Steps:**
1. Modify F5-TTS API to accept batched inputs
2. Implement request queue with batch aggregation
3. Test optimal batch sizes (2, 4, 8, 16)
4. Measure throughput vs latency trade-off

**Files to modify:**
- `third_party/F5-TTS/src/f5_tts/api.py` - Add batch support
- `crates/tts-engine/src/lib.rs` - Batch queue
- `crates/backend/src/main.rs` - Request batching

**Estimated effort**: 1 week
**Risk**: Low (doesn't affect single-request latency)

---

### Priority 3: Streaming Inference (Goal: 50-70% lower perceived latency)

**Goal**: Start playing audio before full synthesis completes

**Implementation Steps:**
1. Modify F5-TTS to generate audio in chunks
2. Stream chunks to frontend via SSE
3. Test chunk sizes (0.5s, 1s, 2s)
4. Measure latency-to-first-audio (TTFA)

**Files to modify:**
- `third_party/F5-TTS/src/f5_tts/infer/utils_infer.py` - Chunked generation
- `crates/backend/src/main.rs` - SSE streaming
- `crates/frontend-web/src/lib.rs` - Chunked playback

**Estimated effort**: 2 weeks
**Risk**: Medium (complex implementation)

---

### Priority 4: Model TensorRT Export (Goal: 1.5-2x speedup)

**Goal**: Export full F5-TTS model to TensorRT (not just vocoder)

**Why this instead of vocoder?**
- Model is 80% of inference time (vocoder only 20%)
- Vocoder TensorRT already tested - slower end-to-end
- Model optimization has much higher impact

**Investigation Steps:**
1. Profile model to confirm it's the bottleneck (should be)
2. Export model to ONNX
3. Build TensorRT engine with dynamic shapes
4. Test with varying text lengths
5. Benchmark vs PyTorch + torch.compile

**Files to create:**
- `scripts/export_model_onnx.py` - Model export
- `scripts/build_model_tensorrt.py` - Engine builder
- `scripts/test_model_tensorrt.py` - Validation

**Estimated effort**: 2-3 weeks
**Risk**: High (complex, may not work with diffusion model)

---

## 📊 Performance Monitoring & Metrics

### Key Metrics to Track

**1. Latency Metrics:**
- Synthesis time (ms)
- Real-Time Factor (RTF)
- Latency-to-first-audio (TTFA) - if streaming
- P50, P95, P99 latencies

**2. Throughput Metrics:**
- Requests per second (RPS)
- Concurrent request capacity
- Queue wait time
- GPU utilization (%)

**3. Quality Metrics:**
- Mean Opinion Score (MOS) - subjective
- NMSE vs baseline - objective
- User feedback/ratings
- Error rate (synthesis failures)

**4. Resource Metrics:**
- GPU utilization (%)
- GPU memory usage (GB)
- CPU usage (%)
- System memory usage (GB)
- Temperature (°C)

**5. Reliability Metrics:**
- Uptime (%)
- Error rate (%)
- torch.compile failures
- Request timeouts
- Memory leaks detected

### Monitoring Setup

**Prometheus + Grafana (Recommended):**
```bash
# TODO: Set up Prometheus exporter
# Expose metrics at /metrics endpoint
# Key metrics:
# - tts_synthesis_duration_seconds (histogram)
# - tts_rtf (gauge)
# - tts_requests_total (counter)
# - tts_errors_total (counter)
# - gpu_utilization_percent (gauge)
# - gpu_memory_used_bytes (gauge)
```

**Simple Logging (Current):**
```bash
# Parse logs for metrics
grep "Synthesis" logs/backend.log | awk '{print $NF}' > /tmp/rtf.txt
python -c "
import numpy as np
rtf = np.loadtxt('/tmp/rtf.txt')
print(f'Mean RTF: {np.mean(rtf):.3f}')
print(f'P95 RTF: {np.percentile(rtf, 95):.3f}')
print(f'Max RTF: {np.max(rtf):.3f}')
"
```

---

## 🧪 Testing Guidelines

### Performance Testing

**Before every optimization:**
```bash
# 1. Baseline benchmark (5+ runs)
python scripts/test_max_autotune.py > baseline.log

# 2. Apply optimization

# 3. New benchmark (5+ runs)
python scripts/test_max_autotune.py > optimized.log

# 4. Compare
echo "Baseline:" && grep "Mean RTF" baseline.log
echo "Optimized:" && grep "Mean RTF" optimized.log
```

**Requirements:**
- Minimum 5 runs for statistical significance
- Report mean, median, p95, p99
- Calculate variance/std dev
- Always use `sudo jetson_clocks` first
- Use same test cases for consistency

---

### Quality Testing

**Objective metrics:**
```bash
# TODO: Create quality test suite
python scripts/test_quality.py \
  --baseline config_nfe32.toml \
  --optimized config_nfe8.toml \
  --samples 20
```

**Subjective metrics:**
- A/B listening tests with 5+ listeners
- Rate naturalness, clarity, speaker similarity
- Use Mean Opinion Score (MOS) scale 1-5
- Target: MOS > 4.0

**Test cases:**
- Short text (1-3 words): "你好"
- Medium text (10-20 words): "欢迎来到我的直播间"
- Long text (50+ words): Full sentences
- Special characters: Numbers, English mixed
- Edge cases: Very fast/slow speech

---

### Load Testing

**Concurrent requests:**
```bash
# Test with increasing load
for concurrent in 1 2 4 8; do
  echo "Testing with $concurrent concurrent requests"
  # TODO: Implement load test script
  python scripts/load_test.py --concurrent $concurrent
done
```

**Long-running test:**
```bash
# Run for 24+ hours to detect memory leaks
python scripts/stress_test.py --duration 86400 --rps 0.5
# Monitor memory usage
watch -n 300 nvidia-smi
```

---

## 🔒 Backup & Recovery

### Backup Critical Files

**Before major changes, backup:**
```bash
# Create timestamped backup
backup_dir=".agent/backups/$(date +%Y%m%d_%H%M%S)"
mkdir -p "$backup_dir"

# Python optimizations (not in git)
cp third_party/F5-TTS/src/f5_tts/api.py "$backup_dir/"
cp third_party/F5-TTS/src/f5_tts/infer/utils_infer.py "$backup_dir/"

# Rust code (in git, but good to have)
cp crates/tts-engine/src/lib.rs "$backup_dir/"

# Config (not in git)
cp config/ishowtts.toml "$backup_dir/"

# Models (if custom)
# cp -r models/ "$backup_dir/"

echo "Backup created at: $backup_dir"
```

### Recovery Procedures

**Rollback Python optimizations:**
```bash
# Use latest backup
latest_backup=$(ls -td .agent/backups/*/ | head -1)
echo "Restoring from: $latest_backup"

cp "$latest_backup/api.py" third_party/F5-TTS/src/f5_tts/
cp "$latest_backup/utils_infer.py" third_party/F5-TTS/src/f5_tts/infer/
cp "$latest_backup/ishowtts.toml" config/

# Restart backend
pkill -f ishowtts-backend
cargo run -p ishowtts-backend
```

**Rollback Rust changes:**
```bash
# Git history
git log --oneline crates/tts-engine/src/lib.rs
git checkout <commit> crates/tts-engine/src/lib.rs

# Rebuild
cargo build --release -p ishowtts-backend
```

**Full system restore to baseline:**
```bash
# 1. Reset F5-TTS to upstream
cd third_party/F5-TTS
git checkout src/f5_tts/api.py src/f5_tts/infer/utils_infer.py
cd ../../

# 2. Reset config to default
cp config/ishowtts.toml.example config/ishowtts.toml
# Edit and set default_nfe_step = 32

# 3. Rebuild backend
cargo clean
cargo build --release -p ishowtts-backend

# Result: Baseline RTF ~1.32, no optimizations
```

---

## 📚 Documentation

### Key Documents

**Current status:**
- `.agent/STATUS.md` - Overall status and metrics
- `.agent/FINAL_OPTIMIZATION_REPORT.md` - Phase 1 completion
- `.agent/SESSION_2025_09_30_FINAL.md` - Latest session summary
- `.agent/MAINTENANCE_GUIDE.md` - This document

**Implementation details:**
- `.agent/python_optimizations_applied.md` - Python changes
- `.agent/optimization_roadmap.md` - Historical roadmap
- `.agent/ONGOING_OPTIMIZATION_PLAN.md` - Future plans

**Session logs:**
- `.agent/SESSION_2025_09_30.md` - Initial optimization session
- `.agent/SESSION_2025_09_30_LATE.md` - Follow-up session
- `.agent/SESSION_2025_09_30_TENSORRT.md` - TensorRT investigation

### Updating Documentation

**After every major change:**
1. Update `.agent/STATUS.md` with new metrics
2. Add session log to `.agent/SESSION_YYYY_MM_DD.md`
3. Update this maintenance guide if procedures change
4. Commit documentation: `git add .agent/ && git commit -m "Update docs"`

---

## 🎯 Success Criteria

### Phase 1 (Current - COMPLETE ✅)
- [x] RTF < 0.30 (achieved: 0.251)
- [x] Speedup > 3.3x (achieved: 3.98x)
- [x] Quality: Good (NFE=8 acceptable)
- [x] Stability: <0.1% error rate
- [x] Documentation: Complete

### Phase 3 (Next Target)
- [ ] **RTF < 0.20** (currently 0.251)
- [ ] **Quality**: MOS > 4.0
- [ ] **Throughput**: 10+ RPS
- [ ] **Reliability**: 99.9% uptime
- [ ] **Tests**: 80%+ coverage

### Phase 4 (Stretch Goals)
- [ ] **RTF < 0.15** with INT8
- [ ] **Throughput**: 20+ RPS with batching
- [ ] **TTFA**: <500ms with streaming
- [ ] **Quality**: MOS > 4.2
- [ ] **Monitoring**: Full observability stack

---

## 📞 Contacts & Resources

### Internal Resources
- Code: `/ssd/ishowtts/`
- Logs: `/ssd/ishowtts/logs/`
- Models: `/ssd/ishowtts/models/` or `/opt/models/`
- Docs: `/ssd/ishowtts/.agent/`

### External Resources
- F5-TTS: https://github.com/SWivid/F5-TTS
- PyTorch: https://pytorch.org/docs/stable/
- TensorRT: https://docs.nvidia.com/deeplearning/tensorrt/
- Jetson: https://developer.nvidia.com/embedded/jetson

### Support
- Rust issues: Check `cargo` logs and error messages
- Python issues: Check torch compatibility, CUDA availability
- Performance issues: Run benchmarks, check GPU lock
- Quality issues: Test with different NFE, check reference audio

---

## ✅ Quick Reference Checklist

### After System Reboot
- [ ] `sudo jetson_clocks` (GPU performance lock)
- [ ] `sudo nvpmodel -m 0` (MAXN power mode)
- [ ] Check GPU frequency: `cat /sys/devices/gpu.0/devfreq/17000000.ga10b/cur_freq`
- [ ] Restart backend: `./scripts/start_all.sh`

### Before Optimization Work
- [ ] Run baseline benchmark: `python scripts/test_max_autotune.py`
- [ ] Create backup: `mkdir .agent/backups/$(date +%Y%m%d) && cp ...`
- [ ] Note current metrics: RTF, GPU util, memory
- [ ] Plan rollback procedure

### After Optimization Work
- [ ] Run benchmark: `python scripts/test_max_autotune.py`
- [ ] Compare vs baseline: RTF, quality, stability
- [ ] Test for 24+ hours for memory leaks
- [ ] Update documentation in `.agent/`
- [ ] Commit changes: `git add . && git commit -m "..." && git push`

### Weekly Maintenance
- [ ] Run performance benchmarks
- [ ] Check logs for errors
- [ ] Monitor GPU memory usage
- [ ] Rotate logs
- [ ] Update status document

---

**Last Updated**: 2025-09-30
**Status**: Production Ready (Phase 1 Complete)
**Next Review**: Weekly