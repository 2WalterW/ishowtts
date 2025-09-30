# iShowTTS Test Suite

Comprehensive testing infrastructure for iShowTTS TTS pipeline.

## Test Organization

### Unit Tests (`test_tts_core.py`)
Tests individual components in isolation:
- Model loading and initialization
- Reference audio preprocessing
- Tensor caching mechanisms
- GPU memory management
- Optimization features (torch.compile, AMP)
- Configuration validation
- Performance metric calculations

**Run**:
```bash
/opt/miniforge3/envs/ishowtts/bin/python tests/test_tts_core.py
```

### Integration Tests (`test_integration.py`)
Tests complete end-to-end synthesis pipeline:
- Basic TTS synthesis
- Multiple voice support
- NFE step variations
- Concurrent synthesis requests
- Memory stability over time
- GPU resource cleanup
- Audio quality checks

**Run**:
```bash
/opt/miniforge3/envs/ishowtts/bin/python tests/test_integration.py
```

**Note**: Integration tests require CUDA and will load the full F5-TTS model.

## Running Tests

### Quick Test
```bash
# Run unit tests only (fast, no model loading)
cd /ssd/ishowtts
/opt/miniforge3/envs/ishowtts/bin/python tests/test_tts_core.py
```

### Full Test Suite
```bash
# Run all tests including integration tests
./scripts/run_test_suite.sh
```

### Individual Test Classes
```bash
# Run specific test class
/opt/miniforge3/envs/ishowtts/bin/python -m unittest tests.test_tts_core.TestModelLoading

# Run specific test
/opt/miniforge3/envs/ishowtts/bin/python -m unittest tests.test_tts_core.TestModelLoading.test_cuda_available
```

## Performance Testing

### Regression Detection
```bash
# Detect performance regressions (alerts if RTF > 0.35)
/opt/miniforge3/envs/ishowtts/bin/python scripts/detect_regression.py

# Custom threshold
/opt/miniforge3/envs/ishowtts/bin/python scripts/detect_regression.py --baseline 0.30 --threshold 0.20
```

### Profiling
```bash
# Identify optimization targets
/opt/miniforge3/envs/ishowtts/bin/python scripts/profile_next_optimization.py

# Save results
/opt/miniforge3/envs/ishowtts/bin/python scripts/profile_next_optimization.py --output logs/profile.json
```

### Benchmarking
```bash
# Quick performance test
/opt/miniforge3/envs/ishowtts/bin/python scripts/test_max_autotune.py

# Comprehensive benchmark
/opt/miniforge3/envs/ishowtts/bin/python scripts/benchmark_tts_performance.py
```

## Test Coverage

### Current Coverage
- ✅ Model loading and initialization
- ✅ Audio preprocessing
- ✅ Tensor operations and caching
- ✅ GPU memory management
- ✅ End-to-end synthesis
- ✅ Performance metrics
- ✅ Regression detection
- ⏳ Quality metrics (MOS, speaker similarity) - Planned
- ⏳ Stress testing - Planned

### Coverage Goals
- Unit tests: 70%+ of core components
- Integration tests: Critical paths covered
- Performance tests: All optimizations validated

## Continuous Integration

### Daily Checks
```bash
# Run regression detection
/opt/miniforge3/envs/ishowtts/bin/python scripts/detect_regression.py

# Quick performance test
/opt/miniforge3/envs/ishowtts/bin/python scripts/quick_performance_test.py
```

### Weekly Checks
```bash
# Full test suite
./scripts/run_test_suite.sh

# Comprehensive profiling
/opt/miniforge3/envs/ishowtts/bin/python scripts/profile_next_optimization.py
```

## Writing New Tests

### Unit Test Template
```python
import unittest
import torch

class TestNewFeature(unittest.TestCase):
    """Test description."""

    def setUp(self):
        """Set up test fixtures."""
        pass

    def tearDown(self):
        """Clean up after test."""
        pass

    def test_feature(self):
        """Test specific behavior."""
        result = my_function()
        self.assertEqual(result, expected)
```

### Integration Test Template
```python
import unittest
import torch
from f5_tts.api import F5TTS

class TestNewIntegration(unittest.TestCase):
    """Integration test description."""

    @classmethod
    def setUpClass(cls):
        """Load model once for all tests."""
        cls.tts = F5TTS(device="cuda")

    @unittest.skipIf(not torch.cuda.is_available(), "CUDA required")
    def test_integration(self):
        """Test end-to-end behavior."""
        wav, sr, _ = self.tts.infer(...)
        self.assertIsNotNone(wav)
```

## Test Maintenance

### Adding New Tests
1. Write test following template
2. Run locally to validate
3. Add to test suite (`run_test_suite.sh`)
4. Update this README
5. Commit with clear message

### Debugging Test Failures
```bash
# Run with verbose output
/opt/miniforge3/envs/ishowtts/bin/python -m unittest -v tests.test_tts_core

# Run single test with debugging
/opt/miniforge3/envs/ishowtts/bin/python -m pdb -m unittest tests.test_tts_core.TestModelLoading.test_cuda_available
```

### Test Data
- Reference audio: `data/voices/*.wav`
- Temporary files: Created in system temp, auto-cleaned
- Test outputs: `logs/test_*.log`

## Performance Targets

### Test Execution Time
- Unit tests: <5 seconds total
- Integration tests: <60 seconds (includes model loading)
- Full test suite: <2 minutes
- Regression detection: <30 seconds

### Performance Thresholds
- RTF target: <0.30
- RTF warning: >0.35
- RTF critical: >0.50
- Memory leak: >100 MB increase over 5 runs

## CI/CD Integration

### Pre-commit Hooks (Future)
```bash
# Run unit tests before commit
pre-commit run --all-files
```

### GitHub Actions (Future)
```yaml
# .github/workflows/tests.yml
name: Tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - run: ./scripts/run_test_suite.sh
```

## Troubleshooting

### Common Issues

**CUDA not available**
```bash
# Check CUDA
python -c "import torch; print(torch.cuda.is_available())"

# Install correct PyTorch version
# See: scripts/bootstrap_python_env.sh
```

**Model loading fails**
```bash
# Check model files exist
ls third_party/F5-TTS/

# Check Python path
echo $PYTHONPATH
```

**Tests timing out**
```bash
# Increase timeout for slow systems
# Edit test file or skip slow tests
python -m unittest tests.test_integration -k "not test_concurrent"
```

## Resources

- Main project: `/ssd/ishowtts`
- Test logs: `logs/`
- Test results: `logs/regression/`
- Documentation: `.agent/`

## Contributing

When adding optimizations:
1. Write tests FIRST (TDD)
2. Run tests to verify they fail
3. Implement optimization
4. Run tests to verify they pass
5. Run regression detection
6. Commit with tests included

**Test coverage is mandatory for all new optimizations.**

---

**Last Updated**: 2025-09-30
**Maintainer**: Agent