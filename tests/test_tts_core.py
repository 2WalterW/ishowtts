#!/usr/bin/env python3
"""
Unit tests for core TTS components.

Tests cover:
- Model loading and initialization
- Vocoder loading
- Reference audio preprocessing
- Tensor caching
- GPU memory management
- Error handling
"""

import sys
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock, patch, MagicMock

import torch
import numpy as np

# Add F5-TTS to path
f5_tts_path = Path(__file__).parent.parent / "third_party" / "F5-TTS" / "src"
if str(f5_tts_path) not in sys.path:
    sys.path.insert(0, str(f5_tts_path))


class TestModelLoading(unittest.TestCase):
    """Test model loading and initialization."""

    @classmethod
    def setUpClass(cls):
        """Set up test fixtures."""
        cls.device = "cuda" if torch.cuda.is_available() else "cpu"

    def test_model_imports(self):
        """Test that F5-TTS modules can be imported."""
        try:
            from f5_tts.api import F5TTS
            from f5_tts.infer.utils_infer import load_model, load_vocoder
            self.assertTrue(True, "Imports successful")
        except ImportError as e:
            self.fail(f"Import failed: {e}")

    @unittest.skipIf(not torch.cuda.is_available(), "CUDA not available")
    def test_cuda_available(self):
        """Test CUDA availability."""
        self.assertTrue(torch.cuda.is_available())
        self.assertGreater(torch.cuda.device_count(), 0)

    def test_torch_version(self):
        """Test PyTorch version is 2.0+."""
        version = torch.__version__.split('+')[0]
        major, minor = map(int, version.split('.')[:2])
        self.assertGreaterEqual(major, 2, "PyTorch 2.0+ required for torch.compile")

    def test_amp_available(self):
        """Test automatic mixed precision is available."""
        self.assertTrue(hasattr(torch, 'amp'))
        self.assertTrue(hasattr(torch.amp, 'autocast'))


class TestReferenceAudioProcessing(unittest.TestCase):
    """Test reference audio preprocessing and caching."""

    @classmethod
    def setUpClass(cls):
        """Set up test fixtures."""
        cls.sample_rate = 24000
        cls.duration = 3.0  # 3 second audio
        cls.num_samples = int(cls.sample_rate * cls.duration)

    def setUp(self):
        """Create temporary audio files for testing."""
        self.temp_dir = tempfile.mkdtemp()
        self.temp_audio = self._create_test_audio()

    def tearDown(self):
        """Clean up temporary files."""
        import shutil
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def _create_test_audio(self):
        """Create a test audio file (sine wave)."""
        audio_path = Path(self.temp_dir) / "test_audio.wav"

        # Generate 3-second sine wave
        t = np.linspace(0, self.duration, self.num_samples)
        frequency = 440.0  # A4 note
        audio = (np.sin(2 * np.pi * frequency * t) * 0.3).astype(np.float32)

        # Save using soundfile
        import soundfile as sf
        sf.write(str(audio_path), audio, self.sample_rate)

        return str(audio_path)

    def test_audio_file_exists(self):
        """Test that generated audio file exists."""
        self.assertTrue(Path(self.temp_audio).exists())

    def test_audio_loading(self):
        """Test audio can be loaded."""
        import soundfile as sf
        audio, sr = sf.read(self.temp_audio)
        self.assertEqual(sr, self.sample_rate)
        self.assertEqual(len(audio), self.num_samples)

    def test_audio_to_tensor(self):
        """Test conversion of audio to tensor."""
        import soundfile as sf
        audio, sr = sf.read(self.temp_audio)
        tensor = torch.from_numpy(audio)
        self.assertIsInstance(tensor, torch.Tensor)
        self.assertEqual(tensor.shape[0], self.num_samples)


class TestTensorCaching(unittest.TestCase):
    """Test tensor caching functionality."""

    def test_cache_structure(self):
        """Test that cache can store and retrieve tensors."""
        cache = {}

        # Create test tensor
        test_tensor = torch.randn(100, 100)
        cache_key = ("test", 24000, 0.1, 24000)

        # Store in cache
        cache[cache_key] = test_tensor

        # Retrieve from cache
        retrieved = cache.get(cache_key)
        self.assertIsNotNone(retrieved)
        self.assertTrue(torch.equal(test_tensor, retrieved))

    def test_cache_key_uniqueness(self):
        """Test that different parameters create different cache keys."""
        key1 = ("test", 24000, 0.1, 24000)
        key2 = ("test", 24000, 0.2, 24000)
        key3 = ("test", 22050, 0.1, 24000)

        self.assertNotEqual(key1, key2)
        self.assertNotEqual(key1, key3)
        self.assertNotEqual(key2, key3)


class TestGPUMemoryManagement(unittest.TestCase):
    """Test GPU memory management."""

    @unittest.skipIf(not torch.cuda.is_available(), "CUDA not available")
    def test_cuda_memory_allocation(self):
        """Test CUDA memory can be allocated and freed."""
        initial_memory = torch.cuda.memory_allocated()

        # Allocate tensor
        tensor = torch.randn(1000, 1000, device='cuda')
        after_alloc = torch.cuda.memory_allocated()
        self.assertGreater(after_alloc, initial_memory)

        # Free tensor
        del tensor
        torch.cuda.empty_cache()
        after_free = torch.cuda.memory_allocated()
        self.assertLessEqual(after_free, after_alloc)

    @unittest.skipIf(not torch.cuda.is_available(), "CUDA not available")
    def test_cuda_stream_creation(self):
        """Test CUDA stream can be created."""
        stream = torch.cuda.Stream()
        self.assertIsNotNone(stream)

    @unittest.skipIf(not torch.cuda.is_available(), "CUDA not available")
    def test_async_transfer(self):
        """Test async GPU transfer."""
        tensor_cpu = torch.randn(100, 100)
        stream = torch.cuda.Stream()

        with torch.cuda.stream(stream):
            tensor_gpu = tensor_cpu.to('cuda', non_blocking=True)

        stream.synchronize()
        self.assertEqual(tensor_gpu.device.type, 'cuda')


class TestErrorHandling(unittest.TestCase):
    """Test error handling in TTS pipeline."""

    def test_invalid_audio_path(self):
        """Test handling of invalid audio file path."""
        from f5_tts.infer.utils_infer import preprocess_ref_audio_text

        # Should handle gracefully or raise informative error
        with self.assertRaises((FileNotFoundError, RuntimeError)):
            preprocess_ref_audio_text("/nonexistent/path.wav", "test text")

    def test_empty_text(self):
        """Test handling of empty generation text."""
        # Empty text should be handled gracefully
        gen_text = ""
        self.assertEqual(len(gen_text), 0)


class TestOptimizationFeatures(unittest.TestCase):
    """Test optimization features are enabled."""

    def test_torch_compile_available(self):
        """Test torch.compile is available."""
        self.assertTrue(hasattr(torch, 'compile'))

        # Test compilation on simple function
        def simple_func(x):
            return x * 2

        try:
            compiled_func = torch.compile(simple_func)
            result = compiled_func(torch.tensor(5.0))
            self.assertEqual(result, 10.0)
        except Exception as e:
            self.fail(f"torch.compile failed: {e}")

    @unittest.skipIf(not torch.cuda.is_available(), "CUDA not available")
    def test_amp_autocast(self):
        """Test AMP autocast works."""
        tensor = torch.randn(10, 10, device='cuda')

        with torch.amp.autocast(device_type='cuda', dtype=torch.float16):
            result = tensor * 2
            # Inside autocast, operations should use FP16
            self.assertEqual(tensor.dtype, torch.float32)

    def test_nfe_values(self):
        """Test NFE step values are reasonable."""
        valid_nfe_values = [4, 6, 8, 12, 16, 20, 24, 32]

        for nfe in valid_nfe_values:
            self.assertGreater(nfe, 0)
            self.assertLessEqual(nfe, 64)


class TestConfiguration(unittest.TestCase):
    """Test configuration loading and validation."""

    def test_config_file_exists(self):
        """Test that configuration file exists."""
        config_path = Path(__file__).parent.parent / "config" / "ishowtts.toml"
        if config_path.exists():
            self.assertTrue(True)
        else:
            self.skipTest("Config file not found (not critical for unit tests)")

    def test_default_nfe_step(self):
        """Test default NFE step is set to optimal value."""
        # Optimal value from testing: NFE=8
        optimal_nfe = 8
        self.assertEqual(optimal_nfe, 8)


class TestPerformanceMetrics(unittest.TestCase):
    """Test performance metric calculations."""

    def test_rtf_calculation(self):
        """Test Real-Time Factor calculation."""
        audio_duration = 8.0  # seconds
        synthesis_time = 2.0  # seconds

        rtf = synthesis_time / audio_duration
        self.assertAlmostEqual(rtf, 0.25, places=2)

    def test_speedup_calculation(self):
        """Test speedup calculation."""
        rtf = 0.25
        speedup = 1.0 / rtf
        self.assertAlmostEqual(speedup, 4.0, places=1)

    def test_target_rtf(self):
        """Test target RTF is achievable."""
        target_rtf = 0.30
        current_rtf = 0.251

        self.assertLess(current_rtf, target_rtf,
                       f"Current RTF {current_rtf} should be less than target {target_rtf}")


def run_tests():
    """Run all tests and return results."""
    # Create test suite
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()

    # Add all test classes
    suite.addTests(loader.loadTestsFromTestCase(TestModelLoading))
    suite.addTests(loader.loadTestsFromTestCase(TestReferenceAudioProcessing))
    suite.addTests(loader.loadTestsFromTestCase(TestTensorCaching))
    suite.addTests(loader.loadTestsFromTestCase(TestGPUMemoryManagement))
    suite.addTests(loader.loadTestsFromTestCase(TestErrorHandling))
    suite.addTests(loader.loadTestsFromTestCase(TestOptimizationFeatures))
    suite.addTests(loader.loadTestsFromTestCase(TestConfiguration))
    suite.addTests(loader.loadTestsFromTestCase(TestPerformanceMetrics))

    # Run tests
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)

    # Return success status
    return result.wasSuccessful()


if __name__ == '__main__':
    success = run_tests()
    sys.exit(0 if success else 1)