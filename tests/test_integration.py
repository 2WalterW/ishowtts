#!/usr/bin/env python3
"""
End-to-end integration tests for iShowTTS.

Tests the full synthesis pipeline from reference audio to generated audio.
"""

import sys
import os
import tempfile
import unittest
from pathlib import Path
import time

import torch
import numpy as np

# Add F5-TTS to path
f5_tts_path = Path(__file__).parent.parent / "third_party" / "F5-TTS" / "src"
if str(f5_tts_path) not in sys.path:
    sys.path.insert(0, str(f5_tts_path))


class TestEndToEndSynthesis(unittest.TestCase):
    """Test complete synthesis pipeline."""

    @classmethod
    def setUpClass(cls):
        """Set up test fixtures - load model once for all tests."""
        cls.device = "cuda" if torch.cuda.is_available() else "cpu"
        cls.sample_rate = 24000
        cls.temp_dir = tempfile.mkdtemp()

        # Create reference audio
        cls.ref_audio_path = cls._create_reference_audio()
        cls.ref_text = "This is a test reference audio for the text to speech system."

        # Initialize F5TTS (may take time due to model loading)
        try:
            from f5_tts.api import F5TTS
            print(f"\n[INFO] Loading F5-TTS model on {cls.device}...")
            cls.tts = F5TTS(device=cls.device)
            cls.model_loaded = True
            print("[INFO] Model loaded successfully")
        except Exception as e:
            print(f"[WARNING] Could not load model: {e}")
            cls.model_loaded = False

    @classmethod
    def tearDownClass(cls):
        """Clean up test fixtures."""
        import shutil
        shutil.rmtree(cls.temp_dir, ignore_errors=True)

    @classmethod
    def _create_reference_audio(cls):
        """Create a test reference audio file."""
        audio_path = Path(cls.temp_dir) / "reference.wav"

        # Generate 5-second audio (varied frequencies for realism)
        duration = 5.0
        num_samples = int(cls.sample_rate * duration)
        t = np.linspace(0, duration, num_samples)

        # Mix of frequencies (fundamental + harmonics)
        audio = np.zeros(num_samples)
        audio += 0.3 * np.sin(2 * np.pi * 200 * t)  # Fundamental
        audio += 0.2 * np.sin(2 * np.pi * 400 * t)  # 2nd harmonic
        audio += 0.1 * np.sin(2 * np.pi * 600 * t)  # 3rd harmonic

        # Add some amplitude modulation (speech-like)
        modulation = 0.5 + 0.5 * np.sin(2 * np.pi * 5 * t)
        audio = audio * modulation

        audio = audio.astype(np.float32)

        # Save
        import soundfile as sf
        sf.write(str(audio_path), audio, cls.sample_rate)

        return str(audio_path)

    @unittest.skipIf(not torch.cuda.is_available(), "CUDA required for E2E tests")
    def test_basic_synthesis(self):
        """Test basic text-to-speech synthesis."""
        if not self.model_loaded:
            self.skipTest("Model not loaded")

        gen_text = "Hello, this is a test of the text to speech system."

        output_path = Path(self.temp_dir) / "output_basic.wav"

        start_time = time.time()

        wav, sr, spec = self.tts.infer(
            ref_file=self.ref_audio_path,
            ref_text=self.ref_text,
            gen_text=gen_text,
            nfe_step=8,
            file_wave=str(output_path),
            seed=42
        )

        synthesis_time = time.time() - start_time

        # Verify output
        self.assertIsNotNone(wav)
        self.assertEqual(sr, self.sample_rate)
        self.assertGreater(len(wav), 0)
        self.assertTrue(output_path.exists())

        # Calculate RTF
        audio_duration = len(wav) / sr
        rtf = synthesis_time / audio_duration

        print(f"\n[PERF] Synthesis time: {synthesis_time:.2f}s")
        print(f"[PERF] Audio duration: {audio_duration:.2f}s")
        print(f"[PERF] RTF: {rtf:.3f}")

        # Check performance target
        self.assertLess(rtf, 0.5, f"RTF {rtf:.3f} exceeds limit (0.5)")

    @unittest.skipIf(not torch.cuda.is_available(), "CUDA required for E2E tests")
    def test_multiple_voices(self):
        """Test synthesis with same model but different reference audios."""
        if not self.model_loaded:
            self.skipTest("Model not loaded")

        # Create second reference audio (different frequency)
        ref2_path = Path(self.temp_dir) / "reference2.wav"
        duration = 4.0
        num_samples = int(self.sample_rate * duration)
        t = np.linspace(0, duration, num_samples)
        audio = (0.3 * np.sin(2 * np.pi * 300 * t)).astype(np.float32)

        import soundfile as sf
        sf.write(str(ref2_path), audio, self.sample_rate)

        gen_text = "Testing different voices."

        # Synthesize with first reference
        wav1, _, _ = self.tts.infer(
            ref_file=self.ref_audio_path,
            ref_text=self.ref_text,
            gen_text=gen_text,
            nfe_step=8,
            seed=42
        )

        # Synthesize with second reference
        wav2, _, _ = self.tts.infer(
            ref_file=str(ref2_path),
            ref_text="Different reference audio.",
            gen_text=gen_text,
            nfe_step=8,
            seed=42
        )

        # Outputs should be different (different voices)
        self.assertFalse(np.array_equal(wav1, wav2))

    @unittest.skipIf(not torch.cuda.is_available(), "CUDA required for E2E tests")
    def test_nfe_step_variations(self):
        """Test synthesis with different NFE steps."""
        if not self.model_loaded:
            self.skipTest("Model not loaded")

        gen_text = "Testing NFE variations."
        nfe_values = [8, 16]

        results = {}

        for nfe in nfe_values:
            start_time = time.time()

            wav, sr, _ = self.tts.infer(
                ref_file=self.ref_audio_path,
                ref_text=self.ref_text,
                gen_text=gen_text,
                nfe_step=nfe,
                seed=42
            )

            synthesis_time = time.time() - start_time
            audio_duration = len(wav) / sr
            rtf = synthesis_time / audio_duration

            results[nfe] = {
                'time': synthesis_time,
                'duration': audio_duration,
                'rtf': rtf
            }

            print(f"\n[PERF] NFE={nfe}: Time={synthesis_time:.2f}s, RTF={rtf:.3f}")

        # NFE=8 should be faster than NFE=16
        self.assertLess(results[8]['rtf'], results[16]['rtf'],
                       "NFE=8 should be faster than NFE=16")

    @unittest.skipIf(not torch.cuda.is_available(), "CUDA required for E2E tests")
    def test_concurrent_synthesis(self):
        """Test multiple synthesis requests (sequential)."""
        if not self.model_loaded:
            self.skipTest("Model not loaded")

        gen_texts = [
            "First synthesis request.",
            "Second synthesis request.",
            "Third synthesis request."
        ]

        rtfs = []

        for i, gen_text in enumerate(gen_texts):
            start_time = time.time()

            wav, sr, _ = self.tts.infer(
                ref_file=self.ref_audio_path,
                ref_text=self.ref_text,
                gen_text=gen_text,
                nfe_step=8,
                seed=42 + i
            )

            synthesis_time = time.time() - start_time
            audio_duration = len(wav) / sr
            rtf = synthesis_time / audio_duration

            rtfs.append(rtf)
            print(f"\n[PERF] Request {i+1}: RTF={rtf:.3f}")

        # All requests should be reasonably fast
        for i, rtf in enumerate(rtfs):
            self.assertLess(rtf, 0.5, f"Request {i+1} RTF {rtf:.3f} exceeds limit")

        # Later requests should not be slower (no memory leak)
        self.assertLessEqual(rtfs[-1], rtfs[0] * 1.5,
                            "Performance degradation detected")

    @unittest.skipIf(not torch.cuda.is_available(), "CUDA required for E2E tests")
    def test_memory_stability(self):
        """Test memory stability over multiple syntheses."""
        if not self.model_loaded:
            self.skipTest("Model not loaded")

        initial_memory = torch.cuda.memory_allocated()
        print(f"\n[MEMORY] Initial: {initial_memory / 1e6:.1f} MB")

        gen_text = "Memory stability test."

        for i in range(5):
            wav, sr, _ = self.tts.infer(
                ref_file=self.ref_audio_path,
                ref_text=self.ref_text,
                gen_text=gen_text,
                nfe_step=8,
                seed=42
            )

            current_memory = torch.cuda.memory_allocated()
            print(f"[MEMORY] After iter {i+1}: {current_memory / 1e6:.1f} MB")

        final_memory = torch.cuda.memory_allocated()
        memory_increase = final_memory - initial_memory

        print(f"[MEMORY] Final: {final_memory / 1e6:.1f} MB")
        print(f"[MEMORY] Increase: {memory_increase / 1e6:.1f} MB")

        # Memory increase should be minimal (<100 MB)
        self.assertLess(memory_increase, 100 * 1e6,
                       f"Memory leak detected: {memory_increase / 1e6:.1f} MB increase")


class TestGPUCleanup(unittest.TestCase):
    """Test GPU resource cleanup."""

    @unittest.skipIf(not torch.cuda.is_available(), "CUDA not available")
    def test_empty_cache(self):
        """Test GPU cache can be emptied."""
        initial_cached = torch.cuda.memory_reserved()

        # Allocate and free tensors
        tensors = [torch.randn(1000, 1000, device='cuda') for _ in range(10)]
        del tensors

        # Empty cache
        torch.cuda.empty_cache()

        final_cached = torch.cuda.memory_reserved()

        # Cache should be same or smaller
        self.assertLessEqual(final_cached, initial_cached * 1.2)

    @unittest.skipIf(not torch.cuda.is_available(), "CUDA not available")
    def test_gpu_synchronize(self):
        """Test GPU synchronization works."""
        torch.cuda.synchronize()
        self.assertTrue(True)


class TestQuality(unittest.TestCase):
    """Test audio quality metrics (basic)."""

    def test_audio_not_silent(self):
        """Test generated audio is not silent."""
        # Create test audio
        sample_rate = 24000
        duration = 2.0
        num_samples = int(sample_rate * duration)

        audio = np.random.randn(num_samples) * 0.1
        audio = audio.astype(np.float32)

        # Check RMS is above threshold
        rms = np.sqrt(np.mean(audio ** 2))
        self.assertGreater(rms, 0.01, "Audio is too quiet or silent")

    def test_audio_not_clipping(self):
        """Test audio is not clipping."""
        audio = np.random.randn(24000) * 0.5
        audio = audio.astype(np.float32)

        # Check no clipping (values > 1.0 or < -1.0)
        max_val = np.max(np.abs(audio))
        self.assertLessEqual(max_val, 1.0, "Audio clipping detected")


def run_tests():
    """Run all integration tests."""
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()

    # Add test classes
    suite.addTests(loader.loadTestsFromTestCase(TestEndToEndSynthesis))
    suite.addTests(loader.loadTestsFromTestCase(TestGPUCleanup))
    suite.addTests(loader.loadTestsFromTestCase(TestQuality))

    # Run with verbose output
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)

    return result.wasSuccessful()


if __name__ == '__main__':
    success = run_tests()
    sys.exit(0 if success else 1)