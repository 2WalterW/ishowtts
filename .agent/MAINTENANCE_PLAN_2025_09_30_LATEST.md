# iShowTTS Maintenance & Optimization Plan
**Date**: 2025-09-30
**Status**: Active Monitoring & Optimization Phase

---

## Current Performance Status

### Benchmark Results (2025-09-30 Latest Test)
- **Mean RTF**: 0.168 ✅ (Target: <0.20)
- **Best RTF**: 0.164 ✅ (17.5% better than target)
- **Worst RTF**: 0.195 ✅ (Still within target!)
- **Mean Speedup**: 5.95x ✅ (Target: >3.3x)
- **Variance (CV)**: 4.7% ✅ (Excellent stability, target <10%)
- **Total Improvement**: 7.8x faster than baseline (RTF 1.32 → 0.168)

**Conclusion**: All Phase 1-3 targets exceeded! System is production-ready.

---

## Applied Optimizations Summary

### Core Optimizations (Phase 1-3)
1. ✅ **torch.compile(mode='max-autotune')** - Model and vocoder JIT compilation
2. ✅ **Automatic Mixed Precision (FP16)** - Leverages Tensor Cores on Jetson Orin
3. ✅ **Reference Audio Tensor Caching** - Avoids redundant preprocessing
4. ✅ **CUDA Stream Async Operations** - CPU/GPU parallelism
5. ✅ **NFE=7 Optimization** - Optimal speed/quality balance
6. ✅ **GPU Frequency Locking** - Consistent performance (RTF variance ±4.7%)
7. ✅ **Skip Unnecessary Spectrogram Generation** - Saves 5-10ms per inference
8. ✅ **FP16 Consistency Through Vocoder** - 5-10% additional speedup
9. ✅ **Remove torch.cuda.empty_cache()** - Eliminates 2-5% sync overhead

### Configuration
- **NFE Steps**: 7 (config/ishowtts.toml)
- **Model**: F5TTS_v1_Base
- **Vocoder**: Vocos (torch.compile, not TensorRT)
- **Power Mode**: MAXN (jetson_clocks)

---

## Next Optimization Opportunities

### High Priority (If RTF <0.15 Needed)

#### 1. NFE=6 Testing
**Estimated Impact**: RTF ~0.145 (14% speedup)
**Risk**: Quality degradation
**Effort**: 2-3 hours (quality validation)
**Status**: Quality samples already generated in `.agent/quality_samples/`

**Action Items**:
- [ ] Conduct subjective listening tests with NFE=6 samples
- [ ] Compare quality metrics (MOS scores if possible)
- [ ] If acceptable, update config and re-benchmark
- [ ] Document quality vs speed tradeoff

#### 2. Batch Processing Optimization
**Estimated Impact**: Better throughput during peak load
**Risk**: Low (no impact on single request latency)
**Effort**: 1-2 weeks
**Status**: Not started

**Action Items**:
- [ ] Implement dynamic batching in tts-engine Rust wrapper
- [ ] Test with concurrent requests (stress testing)
- [ ] Measure throughput improvement
- [ ] Update backend to use batch API

#### 3. Spectrogram Generation Removal
**Estimated Impact**: Already implemented via skip_spectrogram flag
**Status**: ✅ Complete (saves 5-10ms per inference)

### Medium Priority (Advanced Optimizations)

#### 4. INT8 Quantization
**Estimated Impact**: RTF ~0.08-0.11 (1.5-2x speedup)
**Risk**: Quality degradation, complex calibration
**Effort**: 2-4 weeks
**Status**: Research phase

**Action Items**:
- [ ] Research PyTorch INT8 quantization for diffusion models
- [ ] Prepare calibration dataset
- [ ] Implement post-training quantization (PTQ)
- [ ] Validate quality with extensive testing
- [ ] Consider quantization-aware training (QAT) if needed

#### 5. Model TensorRT Export
**Estimated Impact**: Potentially 20-40% speedup
**Risk**: Complex, may not beat torch.compile
**Effort**: 2-3 weeks
**Status**: Not started

**Notes**:
- TensorRT vocoder was tested but slower end-to-end (RTF 0.292 vs 0.251)
- torch.compile is already excellent for dynamic shapes
- Only pursue if INT8 quantization is needed

#### 6. Streaming Inference
**Estimated Impact**: Lower perceived latency (no RTF improvement)
**Risk**: Medium (complex implementation)
**Effort**: 2-3 weeks
**Status**: Not started

**Action Items**:
- [ ] Implement chunked diffusion inference
- [ ] Stream vocoder output as chunks complete
- [ ] Update frontend to handle streaming audio
- [ ] Test latency improvements (time-to-first-audio)

### Low Priority (Optimization Research)

#### 7. CUDA Graphs
**Estimated Impact**: 10-15% speedup for fixed shapes
**Risk**: Limited applicability (input shapes vary)
**Effort**: 1-2 weeks
**Status**: Research only

#### 8. Custom CUDA Kernels
**Estimated Impact**: Unknown (requires profiling)
**Risk**: High complexity, maintenance burden
**Effort**: 4-6 weeks
**Status**: Not recommended unless profiling shows hotspots

---

## Maintenance Tasks

### Daily
- ✅ Run `python scripts/detect_regression.py` (1 min)
  - Detects performance regressions automatically
  - Alerts if RTF > baseline + 10%
- ✅ Check GPU frequency lock after reboot
  ```bash
  sudo jetson_clocks
  sudo nvpmodel -m 0
  ```

### Weekly
- ✅ Run extended performance test (5 min)
  ```bash
  /opt/miniforge3/envs/ishowtts/bin/python scripts/extended_performance_test.py
  ```
- ✅ Review `.agent/performance_log.json` for trends
- ✅ Check for quality degradation reports from users
- ✅ Monitor system logs for errors or warnings

### Monthly
- ✅ Full system health check
  - Test all voice profiles
  - Validate audio quality
  - Check memory usage trends
- ✅ Update documentation
  - Performance metrics
  - Configuration changes
  - Known issues
- ✅ Review optimization roadmap
  - Re-prioritize based on user feedback
  - Update effort estimates

### After System Updates
- ✅ Re-apply GPU performance lock
  ```bash
  sudo jetson_clocks && sudo nvpmodel -m 0
  ```
- ✅ Validate performance with extended tests
- ✅ Re-apply F5-TTS optimizations if submodule updated
  ```bash
  cd third_party/F5-TTS
  git apply ../../.agent/optimizations_2025_09_30.patch
  ```
- ✅ Update baseline if needed
  ```bash
  python scripts/detect_regression.py --update-baseline
  ```

---

## Testing & Validation

### Performance Tests

#### Quick Test (1 run, ~10s)
```bash
/opt/miniforge3/envs/ishowtts/bin/python scripts/quick_performance_test.py
```

#### Extended Test (20 runs, ~2 min)
```bash
/opt/miniforge3/envs/ishowtts/bin/python scripts/extended_performance_test.py
```

#### Regression Detection (Daily)
```bash
/opt/miniforge3/envs/ishowtts/bin/python scripts/detect_regression.py
```

### Quality Validation

#### Generate Quality Samples
```bash
/opt/miniforge3/envs/ishowtts/bin/python scripts/generate_quality_samples.py
```

#### Listen to Samples
```bash
# Samples saved in .agent/quality_samples/
ls -la .agent/quality_samples/
```

### Profiling (When Investigating Bottlenecks)
```bash
/opt/miniforge3/envs/ishowtts/bin/python scripts/profile_bottlenecks.py
```

---

## Critical Setup Requirements

### 1. GPU Performance Lock (CRITICAL!)
**Impact**: RTF 0.168 vs 0.352 without lock (2x difference!)

```bash
# Lock GPU to maximum performance (run after every reboot)
sudo jetson_clocks
sudo nvpmodel -m 0

# Add to startup (recommended)
echo "sudo jetson_clocks" >> ~/.bashrc
```

### 2. Python Environment
**Path**: `/opt/miniforge3/envs/ishowtts`
**Activation**:
```bash
source /opt/miniforge3/bin/activate ishowtts
# or directly use:
/opt/miniforge3/envs/ishowtts/bin/python
```

### 3. Third-Party Code Management
**Important**: F5-TTS optimizations are in third_party/ (git submodule)
- Changes NOT tracked in main repo
- Apply patch manually after F5-TTS updates
- Keep patch file in `.agent/optimizations_2025_09_30.patch`

---

## Monitoring & Alerts

### Performance Metrics to Track
1. **Mean RTF** - Should stay < 0.20
2. **RTF Variance** - Should stay < 10%
3. **Synthesis Time** - Should stay < 5s for 27s audio
4. **Memory Usage** - Monitor for leaks
5. **GPU Utilization** - Should be high during inference

### Performance Degradation Symptoms
- RTF > 0.20 (investigate immediately)
- RTF variance > 10% (check GPU lock)
- Synthesis time increasing over time (memory leak?)
- OOM errors (check cache size)

### Debug Commands
```bash
# Check GPU status
nvidia-smi

# Monitor GPU in real-time
watch -n 1 nvidia-smi

# Check memory usage
free -h

# Check disk space
df -h

# View backend logs
tail -f logs/backend.log

# Profile inference
/opt/miniforge3/envs/ishowtts/bin/python scripts/profile_bottlenecks.py
```

---

## Troubleshooting

### Performance Degradation
1. **Check GPU lock**: `sudo jetson_clocks`
2. **Check power mode**: `sudo nvpmodel -m 0`
3. **Run regression test**: `python scripts/detect_regression.py`
4. **Check GPU frequency**: `cat /sys/devices/gpu.0/devfreq/17000000.ga10b/cur_freq`
5. **Reboot system** and re-lock GPU

### Quality Issues
1. **Verify NFE=7**: Check `config/ishowtts.toml`
2. **Check FP16**: Ensure vocoder uses FP16 context
3. **Test with NFE=8**: Slightly better quality
4. **Verify RMS cache**: Check cache consistency

### Memory Issues
1. **Check memory**: `nvidia-smi` (should have plenty on Orin 32GB)
2. **Clear Python cache**: Restart Python process
3. **Check for leaks**: Monitor memory over time
4. **Last resort**: Re-add `torch.cuda.empty_cache()` only if OOM

### Build Issues
1. **Rust build**: `cargo clean && cargo build -p ishowtts-backend`
2. **Python deps**: Re-run `./scripts/bootstrap_python_env.sh`
3. **Git submodules**: `git submodule update --init --recursive`

---

## Documentation

### Key Files
- **Status**: `.agent/STATUS.md` - Current status and metrics
- **Optimization Report**: `.agent/FINAL_OPTIMIZATION_REPORT.md` - Phase 1 details
- **Quick Reference**: `.agent/OPTIMIZATION_QUICK_REFERENCE.md` - Commands and tips
- **Roadmap**: `.agent/LONG_TERM_ROADMAP.md` - Future optimization plans
- **This File**: `.agent/MAINTENANCE_PLAN_2025_09_30_LATEST.md` - Ongoing maintenance

### Test Scripts
- `scripts/extended_performance_test.py` - Comprehensive benchmarking
- `scripts/quick_performance_test.py` - Fast performance check
- `scripts/detect_regression.py` - Automated regression detection
- `scripts/profile_bottlenecks.py` - Profiling and analysis
- `scripts/generate_quality_samples.py` - Quality validation

---

## Success Criteria

### Phase 3+ (Current) - ACHIEVED ✅
- ✅ RTF < 0.20 (achieved 0.168)
- ✅ Variance < 10% (achieved 4.7%)
- ✅ Speedup > 5x (achieved 5.95x)
- ✅ Production stability
- ✅ Comprehensive testing
- ✅ Full documentation

### Future Phase (Optional)
If RTF < 0.15 becomes requirement:
- [ ] NFE=6 quality validation
- [ ] INT8 quantization research
- [ ] Streaming inference UX improvements
- [ ] Batch processing optimization

---

## Risk Assessment

### Low Risk Optimizations
- ✅ torch.compile (already applied)
- ✅ FP16 AMP (already applied)
- ✅ Caching (already applied)
- ✅ Skip spectrogram (already applied)
- [ ] Batch processing (future)

### Medium Risk Optimizations
- [ ] NFE=6 (quality tradeoff)
- [ ] Streaming inference (complexity)
- [ ] CUDA Graphs (limited applicability)

### High Risk Optimizations
- [ ] INT8 Quantization (quality degradation)
- [ ] Model TensorRT (may not improve)
- [ ] Custom CUDA kernels (maintenance burden)

---

## Conclusion

The system has exceeded all performance targets with RTF=0.168 (target <0.20).
Current focus should be on:
1. **Monitoring** - Daily regression checks
2. **Stability** - Maintaining performance consistency
3. **Documentation** - Keeping records up to date
4. **User Feedback** - Collecting quality reports

Further optimization is **optional** and should only be pursued if:
- New requirements emerge (RTF <0.15)
- Quality can be maintained
- User experience improvements justify effort

**Current recommendation**: Focus on monitoring, stability, and feature development rather than further performance optimization.

---

**Last Updated**: 2025-09-30
**Next Review**: Weekly performance check
**Status**: ✅ Production Ready & Monitoring Phase