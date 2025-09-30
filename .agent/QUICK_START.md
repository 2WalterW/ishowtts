# iShowTTS Quick Start Guide
**Version**: 2025-09-30
**Status**: Production Ready (RTF 0.168)

---

## 🚀 Essential Commands

### Start System
```bash
# 1. Fix protobuf issue (one-time)
export PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION=python

# 2. Lock GPU to max performance (after every reboot!)
sudo jetson_clocks
sudo nvpmodel -m 0

# 3. Activate Python environment
source /opt/miniforge3/bin/activate ishowtts

# 4. Start backend with warmup (recommended)
cargo run -p ishowtts-backend -- --config config/ishowtts.toml --warmup

# Or use the all-in-one script
./scripts/start_all.sh --wait 900 --no-tail
```

### Stop System
```bash
# Press Ctrl+C or
pkill -f ishowtts-backend
pkill -f trunk
```

---

## 📊 Performance Check

### Quick Test (~10s)
```bash
/opt/miniforge3/envs/ishowtts/bin/python scripts/quick_performance_test.py
```

### Extended Test (~2 min, 20 runs)
```bash
/opt/miniforge3/envs/ishowtts/bin/python scripts/extended_performance_test.py
```

### Regression Check (Daily)
```bash
/opt/miniforge3/envs/ishowtts/bin/python scripts/detect_regression.py
```

**Expected Results**:
- RTF: ~0.165-0.170 (target <0.20) ✅
- Variance: <10% ✅
- Speedup: ~5.9-6.1x ✅

---

## 🔧 Critical Setup

### GPU Performance Lock (MUST DO!)
```bash
# Check if locked
cat /sys/devices/gpu.0/devfreq/17000000.ga10b/cur_freq

# Lock to max (run after EVERY reboot)
sudo jetson_clocks
sudo nvpmodel -m 0
```

**Impact**: 2x performance difference!
- Without lock: RTF ~0.35
- With lock: RTF ~0.17

### Protobuf Fix
```bash
# Add to ~/.bashrc (permanent fix)
echo 'export PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION=python' >> ~/.bashrc
source ~/.bashrc

# Or set for current session
export PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION=python
```

---

## ⚙️ Configuration

### Performance Settings
```toml
# config/ishowtts.toml
[f5]
default_nfe_step = 7  # Don't change unless testing
```

**NFE Trade-offs**:
- NFE=6: Faster (RTF ~0.145), slightly lower quality
- NFE=7: Optimal balance ✅ (RTF ~0.168)
- NFE=8: Slightly slower (RTF ~0.19), better quality

---

## 🐛 Troubleshooting

### Performance Degradation (RTF >0.20)
```bash
# 1. Check GPU lock
sudo jetson_clocks

# 2. Check power mode
sudo nvpmodel -q  # Should be mode 0

# 3. Check temperature
tegrastats  # GPU should be <80°C

# 4. Run regression test
/opt/miniforge3/envs/ishowtts/bin/python scripts/detect_regression.py
```

### Python Module Not Found
```bash
# Use correct Python
/opt/miniforge3/envs/ishowtts/bin/python

# Or activate environment
source /opt/miniforge3/bin/activate ishowtts
```

### First Inference Slow (~15s)
**Normal!** torch.compile JIT compilation on first run.
- Solution: Use `--warmup` flag when starting backend

### Protobuf Error
```bash
export PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION=python
```

---

## 📚 Documentation

### Quick Reference
- **This file**: Quick start commands
- `.agent/KNOWN_ISSUES.md`: Common problems and fixes
- `.agent/STATUS.md`: Current performance status
- `.agent/OPTIMIZATION_QUICK_REFERENCE.md`: Optimization commands

### Detailed Guides
- `.agent/MAINTENANCE_PLAN_2025_09_30_LATEST.md`: Full maintenance guide
- `.agent/OPTIMIZATION_SUMMARY_2025_09_30_FINAL.md`: Complete optimization history
- `.agent/NEXT_OPTIMIZATION_IDEAS.md`: Future optimization ideas
- `README.md`: Project overview

---

## 🔍 Monitoring

### GPU Status
```bash
# Real-time monitoring
watch -n 1 nvidia-smi

# Check memory
nvidia-smi --query-gpu=memory.used,memory.free --format=csv
```

### Performance Trends
```bash
# View latest results
cat .agent/performance_results_extended.txt

# Check for regressions
/opt/miniforge3/envs/ishowtts/bin/python scripts/detect_regression.py
```

### System Health
```bash
# Temperatures
tegrastats

# Disk space
df -h

# Memory
free -h
```

---

## 🎯 Performance Targets

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| Mean RTF | <0.20 | 0.168 | ✅ 16% better |
| Best RTF | <0.20 | 0.164 | ✅ 18% better |
| Speedup | >3.3x | 5.95x | ✅ 80% better |
| Variance | <10% | 4.7% | ✅ 53% better |

---

## 🚨 Emergency Recovery

### System Hanging
```bash
# Kill processes
pkill -9 -f ishowtts-backend
pkill -9 -f trunk

# Clear GPU memory
sudo systemctl restart nvargus-daemon  # If needed

# Reboot if necessary
sudo reboot
```

### After Reboot Checklist
1. ✅ Lock GPU: `sudo jetson_clocks`
2. ✅ Set power mode: `sudo nvpmodel -m 0`
3. ✅ Verify: Run quick performance test
4. ✅ If slow: Check GPU frequency lock

### Validate System
```bash
# Quick validation
/opt/miniforge3/envs/ishowtts/bin/python scripts/quick_performance_test.py

# Should see:
# - RTF ~0.16-0.17
# - No errors
# - Warmup then fast inference
```

---

## 📞 Getting Help

### Check These First
1. `.agent/KNOWN_ISSUES.md` - Common problems
2. `.agent/MAINTENANCE_PLAN_2025_09_30_LATEST.md` - Detailed troubleshooting
3. Git commit history - Recent changes
4. Performance results - Baseline comparison

### Debug Information to Collect
```bash
# System info
uname -a
python --version
/opt/miniforge3/envs/ishowtts/bin/python -c "import torch; print(torch.__version__)"
nvidia-smi

# Performance baseline
/opt/miniforge3/envs/ishowtts/bin/python scripts/extended_performance_test.py

# GPU status
cat /sys/devices/gpu.0/devfreq/17000000.ga10b/cur_freq
sudo nvpmodel -q
```

---

## ✅ Daily Checklist

**Morning** (1 min):
- [ ] Check GPU lock: `sudo jetson_clocks` (if rebooted)
- [ ] Run regression test: `python scripts/detect_regression.py`

**Evening** (optional):
- [ ] Review logs: `tail -50 logs/backend.log`
- [ ] Check disk space: `df -h`

**Weekly** (5 min):
- [ ] Full performance test: `python scripts/extended_performance_test.py`
- [ ] Review performance trends
- [ ] Check for updates

---

## 🎓 Pro Tips

1. **Always lock GPU after reboot** - Most common performance issue!
2. **Use warmup flag** - Saves time on first inference
3. **Monitor daily** - Catch regressions early
4. **Keep documentation updated** - Future you will thank you
5. **Test before deploying** - Run quick performance test

---

## 🔗 Quick Links

**Local Files**:
- Config: `config/ishowtts.toml`
- Logs: `logs/backend.log`, `logs/frontend.log`
- Results: `.agent/performance_results_extended.txt`
- Samples: `.agent/quality_samples/`

**Scripts**:
- Start: `./scripts/start_all.sh`
- Test: `scripts/extended_performance_test.py`
- Profile: `scripts/profile_bottlenecks.py`
- Monitor: `scripts/monitor_performance.py`

---

**Remember**: GPU lock + Protobuf fix = Happy TTS! 🎉

**Status**: Production Ready (RTF 0.168, 7.8x faster than baseline)

---

**Last Updated**: 2025-09-30