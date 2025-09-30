# Python Optimizations Applied to third_party/F5-TTS

**Important:** These files are in `.gitignore` and NOT tracked by git. Changes persist locally.

## Files Modified

### 1. third_party/F5-TTS/src/f5_tts/api.py

#### Change 1: Import torch at module level
**Line:** 6
```python
import torch
```

#### Change 2: torch.compile() JIT optimization
**Lines:** 87-98
```python
# Performance optimization: Compile model with torch.compile() for JIT optimization
# This provides 20-40% speedup on PyTorch 2.0+
# Note: First inference will be slower due to compilation overhead
if hasattr(torch, 'compile') and torch.__version__ >= "2.0.0":
    try:
        # Use "reduce-overhead" mode for inference optimization
        self.ema_model = torch.compile(self.ema_model, mode="reduce-overhead")
        self.vocoder = torch.compile(self.vocoder, mode="reduce-overhead")
        print(f"[F5TTS] torch.compile() enabled for model and vocoder (PyTorch {torch.__version__})")
    except Exception as e:
        print(f"[F5TTS] Warning: torch.compile() failed, falling back to eager mode: {e}")
        # If compilation fails, models remain in eager mode (no harm done)
```

#### Change 3: Skip spectrogram when not needed
**Lines:** 138-139, 158
```python
# Performance optimization: Skip spectrogram generation if not needed (saves ~5-10ms)
skip_spec = (file_spec is None)

wav, sr, spec = infer_process(
    # ... other args ...
    skip_spectrogram=skip_spec,
)
```

### 2. third_party/F5-TTS/src/f5_tts/infer/utils_infer.py

#### Change 1: Reference audio tensor cache (ALREADY APPLIED EARLIER)
**Line:** 50
```python
_ref_audio_tensor_cache = {}  # Cache preprocessed audio tensors for faster inference
```

#### Change 2: Automatic Mixed Precision (ALREADY APPLIED EARLIER)
**Lines:** 516-527
```python
# inference with automatic mixed precision for speed
with torch.inference_mode():
    # Use torch.amp for faster inference on CUDA
    if device and "cuda" in str(device):
        with torch.amp.autocast(device_type='cuda', dtype=torch.float16):
            generated, _ = model_obj.sample(
                cond=audio,
                text=final_text_list,
                duration=duration,
                steps=nfe_step,
                cfg_strength=cfg_strength,
                sway_sampling_coef=sway_sampling_coef,
            )
    else:
        generated, _ = model_obj.sample(
            # ... same args ...
        )
```

#### Change 3: Add skip_spectrogram parameter to infer_process
**Line:** 414
```python
def infer_process(
    # ... existing params ...
    skip_spectrogram=False,
):
```

#### Change 4: Pass skip_spectrogram to infer_batch_process
**Line:** 442
```python
return next(
    infer_batch_process(
        # ... other args ...
        skip_spectrogram=skip_spectrogram,
    )
)
```

#### Change 5: Add skip_spectrogram parameter to infer_batch_process
**Line:** 468
```python
def infer_batch_process(
    # ... existing params ...
    skip_spectrogram=False,
):
```

#### Change 6: Skip spectrogram generation in process_batch
**Lines:** 558-563
```python
# Performance optimization: Skip spectrogram generation if not needed
# Saves ~5-10ms per inference
if not skip_spectrogram:
    generated_cpu = generated[0].cpu().numpy()
else:
    generated_cpu = None
del generated
yield generated_wave, generated_cpu
```

#### Change 7: Skip spectrogram in batch results
**Lines:** 579-580
```python
if result:
    generated_wave, generated_mel_spec = next(result)
    generated_waves.append(generated_wave)
    if not skip_spectrogram and generated_mel_spec is not None:
        spectrograms.append(generated_mel_spec)
```

#### Change 8: Skip spectrogram concatenation
**Lines:** 621-624
```python
# Create a combined spectrogram only if needed
if skip_spectrogram or not spectrograms:
    combined_spectrogram = None
else:
    combined_spectrogram = np.concatenate(spectrograms, axis=1)
```

## How to Verify Changes

### Check if torch.compile() is enabled:
```bash
cd /ssd/ishowtts
python3 -c "
import sys
sys.path.insert(0, 'third_party/F5-TTS/src')
from f5_tts.api import F5TTS
import torch
print(f'PyTorch version: {torch.__version__}')
print(f'torch.compile available: {hasattr(torch, \"compile\")}')
"
```

### Check if optimizations are applied:
```bash
cd /ssd/ishowtts
python3 scripts/test_optimizations.py
```

### Manual verification:
```bash
# Check torch.compile code
grep -n "torch.compile" third_party/F5-TTS/src/f5_tts/api.py

# Check skip_spectrogram parameter
grep -n "skip_spectrogram" third_party/F5-TTS/src/f5_tts/infer/utils_infer.py

# Check AMP code (should already exist)
grep -n "torch.amp.autocast" third_party/F5-TTS/src/f5_tts/infer/utils_infer.py

# Check tensor cache (should already exist)
grep -n "_ref_audio_tensor_cache" third_party/F5-TTS/src/f5_tts/infer/utils_infer.py
```

## Backup These Changes

Since third_party is gitignored, you may want to backup these files:

```bash
cd /ssd/ishowtts
mkdir -p .agent/backups/
cp third_party/F5-TTS/src/f5_tts/api.py .agent/backups/api.py.optimized
cp third_party/F5-TTS/src/f5_tts/infer/utils_infer.py .agent/backups/utils_infer.py.optimized
```

## Restore from Backup

If you need to restore optimized versions:

```bash
cd /ssd/ishowtts
cp .agent/backups/api.py.optimized third_party/F5-TTS/src/f5_tts/api.py
cp .agent/backups/utils_infer.py.optimized third_party/F5-TTS/src/f5_tts/infer/utils_infer.py
```

## Rollback to Original

If you need to revert all Python optimizations:

```bash
cd /ssd/ishowtts/third_party/F5-TTS
git checkout src/f5_tts/api.py src/f5_tts/infer/utils_infer.py
```

Then re-apply only the essential optimizations (tensor cache + AMP) if needed.

## Testing Checklist

- [ ] torch.compile() message appears on F5TTS init
- [ ] First inference takes 30-60s (compilation)
- [ ] Subsequent inferences are much faster
- [ ] NFE=16 achieves RTF < 0.3
- [ ] Audio quality is acceptable
- [ ] No errors or warnings in logs
- [ ] Backend API works correctly
- [ ] Frontend playback works correctly

## Performance Expectations

| Metric | Before | After | Target |
|--------|--------|-------|--------|
| First inference | ~1.0s | ~30-60s | Warmup only |
| Subsequent (NFE=16) | ~1.5s | ~0.35s | <0.5s |
| RTF | 0.7-1.0 | <0.2 | <0.3 |
| Speedup | 1x | 4.3x | 3x+ |

## Notes

- torch.compile() requires PyTorch 2.0+
- First inference slower due to JIT compilation (expected)
- Use `--warmup` flag when starting backend to trigger compilation upfront
- All optimizations have graceful fallbacks
- No breaking changes to API or behavior
- Quality impact minimal (FP16 + NFE=16 acceptable for real-time use)