# Known Issues and Fixes

## Protobuf Version Compatibility Error

### Error Message
```
TypeError: Descriptors cannot be created directly.
If this call came from a _pb2.py file, your generated code is out of date and must be regenerated with protoc >= 3.19.0.
```

### Root Cause
Incompatibility between protobuf version and generated protobuf files on Jetson platform.

### Quick Fix (Recommended)
Set environment variable to use pure Python implementation:

```bash
export PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION=python
```

Add to `~/.bashrc` for permanent fix:
```bash
echo 'export PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION=python' >> ~/.bashrc
source ~/.bashrc
```

### Alternative Fixes

#### Option 1: Downgrade protobuf
```bash
pip install protobuf==3.20.3
```

#### Option 2: Upgrade protobuf (may break other dependencies)
```bash
pip install --upgrade protobuf
```

### Impact
- Pure Python parsing is slower (~10-20% overhead)
- Only affects initialization, not inference performance
- Minimal impact on overall TTS performance (< 1%)

### Verification
```bash
python -c "import google.protobuf; print(google.protobuf.__version__)"
echo $PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION
```

---

## Other Known Issues

### GPU Frequency Not Locked After Reboot
**Symptom**: Performance drops to RTF ~0.35 instead of ~0.17

**Fix**: Run after every reboot
```bash
sudo jetson_clocks
sudo nvpmodel -m 0
```

**Permanent Fix**: Add to crontab
```bash
sudo crontab -e
# Add line:
@reboot /usr/bin/jetson_clocks
```

### ONNX Runtime GPU Discovery Warning
**Warning**:
```
[W:onnxruntime:Default, device_discovery.cc:164] GPU device discovery failed
```

**Impact**: None - this is a harmless warning, GPU is still being used

**Fix**: Ignore or suppress with:
```bash
export ORT_LOGGING_LEVEL=3
```

### FFmpeg/avconv Warning from pydub
**Warning**:
```
RuntimeWarning: Couldn't find ffmpeg or avconv - defaulting to ffmpeg, but may not work
```

**Impact**: Only affects audio format conversion if using pydub for different formats

**Fix**: Install ffmpeg if needed
```bash
sudo apt-get install ffmpeg
```

### First Inference Slow (torch.compile)
**Symptom**: First TTS request takes 10-15s instead of 4-5s

**Cause**: PyTorch JIT compilation on first run

**Fix**: This is expected behavior. Use warmup:
```bash
cargo run -p ishowtts-backend -- --config config/ishowtts.toml --warmup
```

---

## Performance Troubleshooting

### RTF Higher Than Expected (>0.20)

#### Check 1: GPU Frequency Lock
```bash
# Check current frequency
cat /sys/devices/gpu.0/devfreq/17000000.ga10b/cur_freq

# Should show max frequency (e.g., 1300500000)
# If lower, re-lock:
sudo jetson_clocks
```

#### Check 2: Power Mode
```bash
# Check current mode
sudo nvpmodel -q

# Should be mode 0 (MAXN)
# If not:
sudo nvpmodel -m 0
```

#### Check 3: Thermal Throttling
```bash
# Check temperatures
tegrastats

# GPU temp should be < 80°C
# If higher, improve cooling
```

#### Check 4: NFE Configuration
```bash
# Check config
grep default_nfe_step config/ishowtts.toml

# Should be 7 for optimal performance
```

### High Variance in Performance

**Cause**: GPU frequency scaling

**Fix**: Lock GPU frequency (see above)

**Verify**:
```bash
/opt/miniforge3/envs/ishowtts/bin/python scripts/extended_performance_test.py
# CV should be < 10%
```

---

## Build Issues

### Rust Compilation Errors

**Error**: "linker 'cc' not found"

**Fix**:
```bash
sudo apt-get install build-essential
```

### Python Module Not Found

**Error**: "ModuleNotFoundError: No module named 'torch'"

**Fix**: Use correct Python environment
```bash
# Activate environment
source /opt/miniforge3/bin/activate ishowtts

# Or use directly
/opt/miniforge3/envs/ishowtts/bin/python
```

### Git Submodule Issues

**Error**: "fatal: No such remote or branch"

**Fix**:
```bash
git submodule update --init --recursive
```

---

## Quality Issues

### Robotic or Distorted Audio

**Possible Causes**:
1. NFE too low (< 6)
2. FP16 precision issues
3. Poor reference audio quality

**Fixes**:
1. Increase NFE to 8 or 10 in config
2. Test with FP32 (remove FP16 optimization temporarily)
3. Use higher quality reference audio (24kHz, clear speech)

### Incorrect Pronunciation

**Cause**: Text preprocessing or tokenization

**Fix**:
1. Check reference text matches reference audio
2. Verify language setting in config
3. Test with different text

---

## Memory Issues

### OOM (Out of Memory) Errors

**Symptom**: CUDA out of memory errors

**Check Memory**:
```bash
nvidia-smi
# Should have plenty of free memory on Orin (32GB)
```

**Fixes**:
1. Restart Python process (clear cache)
2. Reduce batch size if using batching
3. Check for memory leaks (monitor over time)
4. Last resort: Re-add `torch.cuda.empty_cache()` (with performance hit)

---

## Logging and Debugging

### Enable Verbose Logging

**Backend**:
```bash
RUST_LOG=debug cargo run -p ishowtts-backend
```

**Python TTS**:
```python
# In scripts
import os
os.environ['F5_TTS_QUIET'] = '0'
```

### Profile Performance

```bash
/opt/miniforge3/envs/ishowtts/bin/python scripts/profile_bottlenecks.py
```

### Monitor GPU

```bash
watch -n 1 nvidia-smi
```

---

## Contact and Support

For issues not covered here:
1. Check `.agent/` documentation
2. Review git commit history
3. Run regression tests
4. Create GitHub issue with:
   - Error message
   - System info (PyTorch version, CUDA version)
   - Steps to reproduce

---

**Last Updated**: 2025-09-30
**Status**: Active maintenance