# iShowTTS Performance Optimization Plan

## Goal
Optimize audio synthesis speed to reach whisper-TTS level performance while maintaining quality.

## Current Architecture Analysis

### Bottleneck Areas Identified:

1. **Python F5-TTS Inference Pipeline** (`utils_infer.py`, `api.py`)
   - Reference audio preprocessing (silence detection, trimming, resampling)
   - Text chunking and tokenization
   - Model inference with ODE sampling (default 32 NFE steps)
   - Vocoder decoding (Vocos/BigVGAN)
   - Post-processing (cross-fading, silence removal)

2. **Rust-Python Bridge** (`lib.rs`)
   - Data marshalling between Rust and Python
   - GIL (Global Interpreter Lock) contention
   - Resampling in Rust after synthesis

3. **Model Loading & Warmup**
   - Cold start latency for model initialization
   - Vocoder loading
   - Reference audio caching

## Optimization Strategies

### Phase 1: Python Inference Optimizations (High Impact)

#### 1.1 Reduce NFE Steps (Quick Win)
- Default is 32 steps, try 16 or even 8
- Trade-off: slight quality loss for 2-4x speedup
- **Target**: Add adaptive NFE based on text length

#### 1.2 Optimize Reference Audio Preprocessing
- Cache preprocessed audio tensors (not just file paths)
- Skip silence detection on already-processed clips
- Use faster resampling (torchaudio GPU ops)

#### 1.3 Model Inference Optimizations
- Enable torch.compile() for model inference (PyTorch 2.0+)
- Use torch.amp (mixed precision) FP16 on GPU
- Optimize batch processing for multiple chunks

#### 1.4 Vocoder Optimizations
- TensorRT vocoder (mentioned in config)
- Cached vocoder warmup
- Reduce vocoder latency with smaller chunks

#### 1.5 Remove Unnecessary Operations
- Skip spectrogram generation for API calls
- Disable tqdm progress bars (already has F5_TTS_QUIET)
- Remove unnecessary dtype conversions

### Phase 2: Rust-Side Optimizations (Medium Impact)

#### 2.1 Reduce GIL Contention
- Release GIL during long-running operations
- Pre-allocate buffers for audio data transfer

#### 2.2 Optimize Audio Processing
- Use SIMD-optimized resampling (rubato crate)
- Optimize WAV encoding (direct write without cursor)
- Remove redundant f64→f32 conversions

#### 2.3 Parallel Processing
- Process multiple TTS requests in parallel (already has semaphore)
- Consider voice-specific worker pools

### Phase 3: Architectural Improvements (Long Term)

#### 3.1 Model Quantization
- INT8 quantization for model weights
- Dynamic quantization for linear layers

#### 3.2 Streaming Inference
- Implement chunk-by-chunk streaming
- Start playing audio before full synthesis complete

#### 3.3 Pre-computation
- Pre-encode reference audio mel-spectrograms
- Cache text embeddings for common phrases

## Performance Metrics to Track

- **Synthesis Latency**: Time from request to first audio byte
- **RTF (Real-Time Factor)**: synthesis_time / audio_duration (target: <0.5)
- **Memory Usage**: Peak GPU/CPU memory
- **Quality**: MOS scores, WER for intelligibility

## Testing Strategy

- Benchmark suite with various text lengths (short/medium/long)
- Compare quality metrics before/after optimizations
- A/B testing with original vs optimized pipeline
- Load testing for concurrent requests

## Implementation Priority

1. ✅ Reduce NFE steps (16 instead of 32)
2. ✅ Enable torch.compile() and mixed precision
3. ✅ Cache reference audio tensors
4. ✅ Remove unnecessary operations
5. Optimize Rust resampling
6. TensorRT vocoder integration
7. Streaming inference

## Notes

- Jetson AGX Orin has good Tensor Core support for FP16
- Need to preserve quality for live streaming use case
- Warmup time is less critical than per-request latency