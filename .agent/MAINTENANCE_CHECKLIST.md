# iShowTTS Daily Maintenance Checklist

**Last Updated**: 2025-09-30

---

## 🔄 Daily Routine (5 minutes)

### 1. GPU Status Check ✅
```bash
# Check power mode
sudo nvpmodel -q | grep "NV Power Mode"
# Should show: MAXN

# Check GPU frequency
sudo jetson_clocks --show | grep GPU
# Should show: MinFreq=MaxFreq=1300500000

# If not locked, run:
sudo jetson_clocks
sudo nvpmodel -m 0
```

### 2. Quick Performance Test ✅
```bash
# Run quick validation (30 seconds)
/opt/miniforge3/envs/ishowtts/bin/python scripts/validate_nfe7.py

# Expected results:
# Mean RTF: 0.21 ± 0.01 (< 0.25 is good)
# Variance: < 5%
# Speedup: > 4.5x
```

### 3. System Health Check ✅
```bash
# Check GPU temperature
nvidia-smi
# Should be: < 85°C

# Check system load
htop
# CPU load should be: < 6.0

# Check memory
free -h
# Should have: > 10GB free
```

---

## 📅 Weekly Routine (30 minutes)

### 1. Full Test Suite ✅
```bash
# Run all unit tests (20 seconds)
cd /ssd/ishowtts
/opt/miniforge3/envs/ishowtts/bin/python tests/test_tts_core.py

# Expected: All tests pass (22/22)

# Run integration tests (2 minutes)
/opt/miniforge3/envs/ishowtts/bin/python tests/test_integration.py

# Expected: All critical tests pass
```

### 2. Performance Regression Check ✅
```bash
# Run performance monitoring (60 seconds)
/opt/miniforge3/envs/ishowtts/bin/python scripts/monitor_performance.py

# Should report:
# - Current vs historical performance
# - Alert if RTF > 0.25
# - Variance should be < 10%
```

### 3. Quality Sample Check ✅
```bash
# Generate fresh quality samples
/opt/miniforge3/envs/ishowtts/bin/python scripts/generate_quality_samples.py

# Listen to samples, check for:
# - Natural prosody
# - Clear pronunciation
# - No artifacts or glitches
# - Consistent quality
```

### 4. Disk Space Check ✅
```bash
# Check disk usage
df -h /ssd

# Clean up if needed:
# - Old logs: rm -rf logs/*.log.old
# - Old samples: find .agent/quality_samples -mtime +30 -delete
# - Build artifacts: cargo clean (if needed)
```

---

## 🔧 Monthly Routine (1 hour)

### 1. Dependency Updates ✅
```bash
# Update Rust dependencies
cargo update
cargo build --release -p ishowtts-backend

# Test after update
/opt/miniforge3/envs/ishowtts/bin/python scripts/validate_nfe7.py

# If performance degrades, revert:
git restore Cargo.lock
cargo build --release -p ishowtts-backend
```

### 2. Python Environment Check ✅
```bash
# Check for package updates
source /opt/miniforge3/envs/ishowtts/bin/activate
pip list --outdated

# Be cautious updating:
# - torch (may break optimizations)
# - torchaudio (compatibility)
# - Any F5-TTS dependencies

# If updating PyTorch:
# 1. Backup current environment
# 2. Test thoroughly after update
# 3. Verify torch.compile still works
# 4. Check performance hasn't degraded
```

### 3. Configuration Backup ✅
```bash
# Backup critical files (NOT in git)
cp config/ishowtts.toml .agent/backups/config_backup_$(date +%Y%m%d).toml
cp third_party/F5-TTS/src/f5_tts/api.py .agent/backups/api_backup_$(date +%Y%m%d).py
cp third_party/F5-TTS/src/f5_tts/infer/utils_infer.py .agent/backups/utils_infer_backup_$(date +%Y%m%d).py

# Clean old backups (keep last 3 months)
find .agent/backups -name "*backup*.py" -mtime +90 -delete
find .agent/backups -name "*backup*.toml" -mtime +90 -delete
```

### 4. Documentation Review ✅
```bash
# Review and update key documents
vim .agent/STATUS.md              # Update current metrics
vim README.md                      # Update if needed
vim .agent/MAINTENANCE_GUIDE.md   # Add any new findings

# Commit updates
git add .agent/ README.md
git commit -m "Update documentation - $(date +%Y-%m-%d)"
git push
```

---

## 🚨 Emergency Procedures

### Performance Degradation (RTF > 0.35)
```bash
# Step 1: Lock GPU
sudo jetson_clocks
sudo nvpmodel -m 0

# Step 2: Check thermal throttling
nvidia-smi
# If temp > 85°C, check cooling

# Step 3: Restart backend
pkill ishowtts-backend
./scripts/start_all.sh

# Step 4: Verify optimizations
grep "max-autotune" third_party/F5-TTS/src/f5_tts/api.py
# If missing, restore from backups

# Step 5: Test again
/opt/miniforge3/envs/ishowtts/bin/python scripts/validate_nfe7.py
```

### Quality Issues
```bash
# Step 1: Increase NFE for safety
vim config/ishowtts.toml
# Change: default_nfe_step = 8 (or 16 for best quality)

# Step 2: Restart backend
pkill ishowtts-backend
./scripts/start_all.sh

# Step 3: Generate test samples
/opt/miniforge3/envs/ishowtts/bin/python scripts/generate_quality_samples.py

# Step 4: Compare with baseline
# Listen to new samples vs known good samples
# If still poor, check reference audio file
```

### System Crashes
```bash
# Step 1: Check system logs
dmesg | tail -50
journalctl -xe

# Step 2: Check GPU status
nvidia-smi

# Step 3: Reboot if needed (last resort)
sudo reboot

# Step 4: After reboot, lock GPU FIRST
sudo jetson_clocks
sudo nvpmodel -m 0

# Step 5: Start services
cd /ssd/ishowtts
./scripts/start_all.sh

# Step 6: Verify performance
/opt/miniforge3/envs/ishowtts/bin/python scripts/validate_nfe7.py
```

### Lost Optimizations (after git update)
```bash
# Step 1: Verify what's missing
grep "torch.compile" third_party/F5-TTS/src/f5_tts/api.py
grep "autocast" third_party/F5-TTS/src/f5_tts/infer/utils_infer.py

# Step 2: Restore from backups
ls -lt .agent/backups/optimized_python_files/

# Copy most recent backups
cp .agent/backups/optimized_python_files/api.py.optimized \
   third_party/F5-TTS/src/f5_tts/api.py

cp .agent/backups/optimized_python_files/utils_infer.py.optimized \
   third_party/F5-TTS/src/f5_tts/infer/utils_infer.py

# Step 3: Restart and test
pkill ishowtts-backend
./scripts/start_all.sh --wait 900
/opt/miniforge3/envs/ishowtts/bin/python scripts/validate_nfe7.py
```

---

## 📊 Performance Targets & Alerts

### Green Zone ✅ (Normal Operation)
- Mean RTF: < 0.25
- Variance: < 5%
- GPU temp: < 75°C
- Memory free: > 10GB
- All tests: PASS

### Yellow Zone ⚠️ (Warning)
- Mean RTF: 0.25 - 0.30
- Variance: 5% - 10%
- GPU temp: 75-85°C
- Memory free: 5-10GB
- Some tests: SKIP

**Action**: Investigate and address within 24 hours

### Red Zone 🚨 (Critical)
- Mean RTF: > 0.30
- Variance: > 10%
- GPU temp: > 85°C
- Memory free: < 5GB
- Tests: FAIL

**Action**: Immediate attention required, follow emergency procedures

---

## 🔍 Monitoring Commands Reference

### One-Line Status Check
```bash
# Complete status in one command
./scripts/quick_status.sh

# Shows:
# - GPU lock status
# - Current config (NFE)
# - Quick performance test
# - Service status
```

### Performance Monitoring
```bash
# Real-time GPU monitoring
watch -n 1 nvidia-smi

# Real-time system monitoring
htop

# Backend logs
tail -f logs/backend.log

# Performance history
/opt/miniforge3/envs/ishowtts/bin/python scripts/monitor_performance.py --history
```

### Testing Commands
```bash
# Quick test (30s)
/opt/miniforge3/envs/ishowtts/bin/python scripts/validate_nfe7.py

# Full test suite (20s)
/opt/miniforge3/envs/ishowtts/bin/python tests/test_tts_core.py

# NFE comparison (5 min)
/opt/miniforge3/envs/ishowtts/bin/python scripts/test_nfe_performance.py

# Quality samples (2 min)
/opt/miniforge3/envs/ishowtts/bin/python scripts/generate_quality_samples.py
```

---

## 📝 Maintenance Log Template

```markdown
# Maintenance Log - YYYY-MM-DD

## Routine Checks
- [ ] GPU locked (MAXN, 1300.5 MHz)
- [ ] Performance test (RTF: ___, Variance: ___)
- [ ] System health (CPU: ___, Temp: ___, Mem: ___)
- [ ] Service status (Backend: ___, Frontend: ___)

## Tests Run
- [ ] Quick performance test (Result: ___)
- [ ] Unit tests (Pass/Fail: ___)
- [ ] Quality samples (Good/Fair/Poor: ___)

## Issues Found
- Issue 1: ___
  - Severity: Green/Yellow/Red
  - Action taken: ___
  - Resolved: Yes/No

## Changes Made
- Change 1: ___
- Change 2: ___

## Performance Metrics
- Mean RTF: ___
- Best RTF: ___
- Variance: ___
- Speedup: ___

## Notes
___
```

---

## 🎯 Current Status Summary

**Performance**: RTF 0.210 (6.3x speedup) ✅
**Stability**: ±2.5% variance ✅
**GPU**: Locked to MAXN ✅
**Tests**: 22/22 passing ✅
**Status**: Production ready ✅

**Next Action**: Evaluate NFE=6 quality samples for final Phase 3 decision

**Contact**: See `.agent/CURRENT_STATUS_2025_09_30_LATEST.md` for detailed status

---

**Remember**: GPU locking is CRITICAL! Always run `sudo jetson_clocks` after reboot.