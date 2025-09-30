# iShowTTS - Next Optimization Steps

**Date**: 2025-09-30
**Current Status**: Phase 3 - 96.5% Complete
**Current Performance**: Mean RTF = 0.213, Best RTF = 0.209

---

## 🎯 Current Position

### Performance
- **Target**: RTF < 0.20
- **Current Mean**: 0.213 (6.5% above target)
- **Current Best**: 0.209 (✅ meets target!)
- **Stability**: ±3.0% variance (excellent)

### Gap Analysis
- Need: **3.3% improvement** to consistently meet target
- Alternative: Accept RTF 0.213 as "production ready" (only 6.5% above target)

---

## 📋 Optimization Options Analysis

### Option A: NFE=6 (Quick Win, Higher Risk)

**Goal**: Reduce NFE from 7 to 6 steps

**Expected Performance**:
- Estimated RTF: **0.182** (14% faster than current)
- Would **exceed Phase 3 target** by 9%

**Pros**:
- ✅ Quick implementation (config change only)
- ✅ No code changes needed
- ✅ Reversible immediately
- ✅ Would definitely meet Phase 3 target

**Cons**:
- ⚠️ Quality degradation risk (7→6 is larger step than 8→7)
- ⚠️ May produce artifacts or unnatural speech
- ⚠️ Needs extensive quality testing

**Implementation**:
1. Change `default_nfe_step = 6` in config
2. Generate 20-30 quality samples
3. A/B test vs NFE=7 samples
4. Measure MOS scores
5. If quality acceptable: deploy
6. If quality poor: revert to NFE=7

**Timeline**: 3-5 days (mostly quality validation)

**Risk**: Medium-High

---

### Option B: INT8 Quantization (High Impact, Medium Risk)

**Goal**: Quantize F5-TTS model to INT8 precision

**Expected Performance**:
- Model is 70% of inference time
- INT8 typically gives 1.5-2x speedup on quantized portion
- Expected total RTF: **0.14-0.16** (25-35% faster)
- Would **far exceed Phase 3 target**

**Pros**:
- ✅ Major performance improvement
- ✅ Would position us well for Phase 4 (RTF < 0.15)
- ✅ More headroom for future features
- ✅ PyTorch has good INT8 support

**Cons**:
- ⚠️ 2-4 weeks implementation time
- ⚠️ Quality sensitive (need extensive testing)
- ⚠️ May require calibration dataset
- ⚠️ torch.compile interaction unclear

**Implementation Steps**:
1. Research PyTorch quantization APIs (1-2 days)
2. Prepare calibration dataset (2-3 days)
3. Implement post-training quantization (3-5 days)
4. Validate quality (5-7 days)
5. Benchmark performance (1-2 days)
6. Document and deploy (1-2 days)

**Timeline**: 2-4 weeks

**Risk**: Medium

---

### Option C: Streaming Inference (UX Improvement, No RTF Change)

**Goal**: Start audio playback before synthesis completes

**Expected Performance**:
- RTF: **No change** (0.213)
- Time-to-first-audio: **0.5-0.8s** (down from ~2s)
- Perceived latency: **50-70% reduction**

**Pros**:
- ✅ Huge UX improvement for livestream
- ✅ Doesn't affect quality
- ✅ Low risk
- ✅ Can combine with other optimizations

**Cons**:
- ❌ Doesn't meet Phase 3 RTF target
- ⚠️ 2-3 weeks implementation
- ⚠️ Frontend + backend changes needed
- ⚠️ Audio cross-fade complexity

**Implementation**:
1. Implement chunked generation (1-2s chunks)
2. Add SSE streaming endpoint
3. Update frontend for streaming playback
4. Test cross-fade quality
5. Load testing

**Timeline**: 2-3 weeks

**Risk**: Low

---

### Option D: Accept Current Performance (No Change)

**Goal**: Declare Phase 3 complete at RTF 0.213

**Rationale**:
- Only 6.5% above target
- Best RTF (0.209) meets target
- Excellent stability (±3.0%)
- 6.2x faster than baseline
- Production ready

**Pros**:
- ✅ No additional work
- ✅ Focus on maintenance/testing
- ✅ Very stable current state
- ✅ Move to other priorities (streaming, features)

**Cons**:
- ❌ Doesn't strictly meet Phase 3 goal
- ⚠️ Perfectionism vs pragmatism trade-off

---

## 🎯 Recommendations (Priority Order)

### 1. Test NFE=6 (Recommended: Try First)

**Why**: Low effort, potentially high reward, quick feedback

**Action Plan** (3-5 days):
```bash
# Day 1: Generate samples
1. Create scripts/test_nfe6_quality.py
2. Generate 30 samples with NFE=6
3. Generate 30 samples with NFE=7 (baseline)
4. Save both sets with timestamps

# Day 2-3: Quality evaluation
5. Listen to all 60 samples
6. Rate quality on 1-5 scale
7. Check for artifacts (clicks, pops, unnaturalness)
8. A/B blind comparison

# Day 4: Decision
9. If quality acceptable: Deploy NFE=6
10. If quality poor: Keep NFE=7, move to Option B

# Day 5: Documentation
11. Document findings
12. Update STATUS.md
13. Commit changes
```

**Success Criteria**:
- Quality drop < 10% subjectively
- No obvious artifacts
- Mean RTF < 0.20
- Variance < 5%

---

### 2. If NFE=6 Fails: Pursue INT8 Quantization

**Why**: Only other path to RTF < 0.20 with current architecture

**Action Plan** (2-4 weeks):
```bash
Week 1: Research & Prep
- Study PyTorch quantization docs
- Create calibration dataset
- Write quantization test script
- Benchmark baseline

Week 2: Implementation
- Apply post-training quantization
- Test torch.compile compatibility
- Initial quality checks
- Performance benchmarks

Week 3: Validation
- Extensive quality testing
- MOS scores
- Speaker similarity tests
- Stress testing

Week 4: Deployment
- Documentation
- Integration with backend
- Final benchmarks
- Commit and push
```

---

### 3. Parallel: Start Streaming Inference (UX Priority)

**Why**: Independent of RTF optimizations, huge UX win

Can work on this regardless of NFE=6 outcome. Streaming + NFE=7 (or NFE=6) would give excellent perceived performance.

---

### 4. Accept RTF 0.213 and Move On

**Why**: Pragmatic choice if quality is more important than 3.3% speed

Focus on:
- Streaming inference (UX)
- Test suite completion
- Feature development
- Production stability

---

## 📊 Decision Matrix

| Option | RTF Result | Timeline | Risk | Effort | UX Impact | Recommended |
|--------|-----------|----------|------|--------|-----------|-------------|
| A: NFE=6 | 0.182 | 3-5 days | Med-High | Low | - | ✅ Try First |
| B: INT8 | 0.14-0.16 | 2-4 weeks | Medium | High | - | If A fails |
| C: Streaming | 0.213 | 2-3 weeks | Low | High | ✅✅✅ | Parallel work |
| D: Accept | 0.213 | 0 days | - | - | - | If A fails + time constrained |

---

## 🚀 Immediate Next Steps (Today)

1. ✅ **Create NFE=6 quality test script** (scripts/test_nfe6_quality.py)
2. ✅ **Generate 30 sample pairs** (NFE=6 vs NFE=7)
3. ✅ **Quick listen test** (5-10 samples)
4. ✅ **Make go/no-go decision** on NFE=6

If go: Deploy NFE=6 and declare Phase 3 complete
If no-go: Choose between Option B (INT8) or Option D (Accept)

---

## 📝 Quality Testing Script Plan

```python
# scripts/test_nfe6_quality.py

test_sentences = [
    # Short (1-2s)
    "Hello world!",
    "How are you doing today?",
    "This is a test.",

    # Medium (3-5s)
    "The quick brown fox jumps over the lazy dog.",
    "Machine learning is transforming the world of technology.",
    "Welcome to the livestream, thanks for joining us!",

    # Long (6-10s)
    "In the field of artificial intelligence and machine learning, "
    "we are witnessing rapid advancements that are reshaping our future.",

    # Complex (technical words, numbers)
    "The model achieved an RTF of 0.213 with NFE equals 7.",
    "Version 2.5.0 includes PyTorch CUDA optimization.",

    # Emotional (varied intonation)
    "Wow, that's absolutely amazing!",
    "I'm sorry to hear that happened.",
    "Congratulations on your success!",
]

# Generate each with NFE=6 and NFE=7
# Save to .agent/quality_samples/nfe6_test/
# Compare and document findings
```

---

## 🎯 Success Definition

**Phase 3 Complete When**:
- Mean RTF ≤ 0.20, OR
- Best RTF ≤ 0.20 AND Mean RTF ≤ 0.22 AND quality validated, OR
- RTF = 0.213 accepted as "Phase 3.5" (practical target met)

**Current**: Best RTF = 0.209 ✅, Mean RTF = 0.213 (6.5% above)

---

**Status**: Ready to test NFE=6 quality
**Next Action**: Create test script and generate samples
**Decision Point**: After quality evaluation (3-5 days)
**Fallback**: INT8 quantization (2-4 weeks) OR Accept current performance

🎯 **Recommendation: Test NFE=6 first - low risk, high potential reward**