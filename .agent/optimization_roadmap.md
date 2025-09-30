# iShowTTS Performance Optimization Roadmap

**Goal**: Achieve Whisper TTS-level latency (<0.3 RTF) while maintaining audio quality

## Current Status (as of 2025-09-30)

### Already Implemented ✅
1. **Rust WAV encoding optimization** (c0f9e1b)
   - Direct Vec<u8> writing, no Cursor overhead
   - Pre-allocated buffers

2. **Rust resampling optimization** (c0f9e1b)
   - f32 arithmetic instead of f64
   - unsafe `get_unchecked` for bounds-checked loops

3. **Configurable NFE steps** (c0f9e1b)
   - Default changed from 32 to 16 (2x speedup)
   - Configurable via `default_nfe_step` in config

4. **Python F5-TTS optimizations** (uncommitted, in third_party)
   - Reference audio tensor caching
   - Automatic mixed precision (FP16) with torch.amp
   - Enhanced documentation

### Current Performance Baseline
- **Expected RTF**: ~0.3-0.5 (with NFE=16 + FP16)
- **Synthesis time**: ~0.5-1.0s for typical short text
- **Target**: <0.3 RTF (Whisper TTS level)

## Phase 1: Advanced Python Optimizations (High Impact)

### 1.1 torch.compile() - JIT Compilation ⭐⭐⭐
**Impact**: 20-40% speedup
**Effort**: Low
**Implementation**:
```python
# In F5TTS.__init__ or load_model()
if torch.__version__ >= "2.0.0":
    self.model = torch.compile(self.model, mode="reduce-overhead")
    self.vocoder = torch.compile(self.vocoder, mode="reduce-overhead")
```
**Risks**:
- First inference slower (compilation overhead)
- May need warmup
- Some operations not supported

### 1.2 CUDA Graphs Capture ⭐⭐
**Impact**: 10-30% speedup for repeated inputs
**Effort**: Medium
**Implementation**:
- Capture inference graph after warmup
- Replay graph for subsequent calls
- Requires fixed input shapes
**File**: `third_party/F5-TTS/src/f5_tts/infer/utils_infer.py`

### 1.3 Disable Spectrogram Generation ⭐
**Impact**: 5-10ms saved
**Effort**: Low
**Implementation**:
- Add flag to skip spectrogram plotting
- Only generate when explicitly requested
**File**: `third_party/F5-TTS/src/f5_tts/infer/utils_infer.py`

### 1.4 Vocoder Optimization ⭐⭐⭐
**Impact**: 30-50% vocoder speedup
**Effort**: Medium
**Options**:
- TensorRT vocoder (already configurable)
- torch.compile() vocoder
- Lower vocoder resolution
**File**: Config option `vocoder_local_path`

## Phase 2: Rust/System Optimizations (Medium Impact)

### 2.1 SIMD-based Resampling ⭐⭐
**Impact**: 2-3x resampling speedup
**Effort**: Medium
**Implementation**:
- Use `std::simd` or `packed_simd` crate
- Vectorized f32 operations
**File**: `crates/tts-engine/src/lib.rs`

### 2.2 Parallel Audio Processing ⭐
**Impact**: 10-20% for multi-request scenarios
**Effort**: Low
**Implementation**:
- Use rayon for parallel WAV encoding
- Parallel resampling chunks
**File**: `crates/tts-engine/src/lib.rs`

### 2.3 Memory Pool for Audio Buffers ⭐
**Impact**: 5-10ms saved on allocation
**Effort**: Medium
**Implementation**:
- Pre-allocate audio buffers
- Reuse buffers across requests
**File**: `crates/tts-engine/src/lib.rs`

## Phase 3: Advanced Features (Low-Medium Impact)

### 3.1 Batch Inference ⭐⭐⭐
**Impact**: 2-3x throughput for multiple requests
**Effort**: High
**Implementation**:
- Batch multiple TTS requests
- Process in single forward pass
**Files**:
- `crates/backend/src/synth.rs`
- `crates/tts-engine/src/lib.rs`
- Python F5-TTS wrapper

### 3.2 Streaming Inference ⭐⭐
**Impact**: Perceived latency reduction (TTFB)
**Effort**: High
**Implementation**:
- Stream audio chunks as they're generated
- Start playback before full synthesis
**Files**: Backend + Frontend integration

### 3.3 Model Quantization ⭐
**Impact**: 10-30% speedup, 50% memory reduction
**Effort**: High
**Implementation**:
- INT8 or INT4 quantization
- Use PyTorch quantization API
**Risks**: Quality degradation

## Phase 4: Infrastructure Optimizations

### 4.1 Model Warmup on Startup ⭐⭐
**Impact**: Eliminates first-request latency
**Effort**: Low
**Implementation**:
- Already supported via `--warmup` flag
- Ensure it's used in production

### 4.2 GPU Memory Management ⭐
**Impact**: Prevents OOM, enables larger batches
**Effort**: Low
**Implementation**:
- Explicit CUDA cache clearing
- Memory monitoring and limits

### 4.3 Request Prioritization ⭐
**Impact**: Better UX for interactive use
**Effort**: Medium
**Implementation**:
- Priority queue for API vs danmaku
- Interrupt long-running synthesis

## Testing Strategy

### Unit Tests (20% of time)
- Test each optimization individually
- Verify correctness (audio checksum)
- Measure micro-benchmarks

### E2E Tests (80% of time)
- Real-world synthesis benchmarks
- A/B quality testing (MOS scores)
- Stress testing (concurrent requests)

### Benchmark Suite
```bash
./scripts/benchmark_tts.sh
python3 scripts/test_optimizations.py
```

## Success Metrics

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| RTF (NFE=16) | ~0.5 | <0.3 | 🟡 In Progress |
| Synthesis time (10 chars) | ~0.5s | <0.3s | 🟡 In Progress |
| TTFB (streaming) | N/A | <200ms | ⬜ Not Started |
| Throughput (batch) | 1 req/s | 5 req/s | ⬜ Not Started |
| GPU memory | ~4GB | <3GB | ⬜ Not Started |

## Implementation Priority

**Week 1-2**: Phase 1 (High Impact Python)
- torch.compile() ✅
- Disable unnecessary spectrogram generation ✅
- Vocoder optimization (TensorRT or torch.compile) ✅

**Week 3-4**: Phase 2 (Rust optimizations)
- SIMD resampling ⬜
- Memory pooling ⬜

**Week 5+**: Phase 3 (Advanced features)
- Batch inference ⬜
- Streaming ⬜

## Notes

- Focus on 80/20 rule: high-impact, low-effort optimizations first
- Maintain quality: A/B test all changes
- Document rollback procedures for each optimization
- Commit and push after each successful optimization