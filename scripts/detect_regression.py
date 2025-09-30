#!/usr/bin/env python3
"""
Performance regression detection for iShowTTS.

Monitors performance metrics and alerts on regressions:
- RTF > 0.35 (20% regression from target 0.30)
- Memory leaks
- GPU utilization issues
- Error rates

Usage:
    python scripts/detect_regression.py [--baseline BASELINE_RTF]
"""

import sys
import json
import time
import argparse
from pathlib import Path
from datetime import datetime

import torch
import numpy as np

# Add F5-TTS to path
f5_tts_path = Path(__file__).parent.parent / "third_party" / "F5-TTS" / "src"
if str(f5_tts_path) not in sys.path:
    sys.path.insert(0, str(f5_tts_path))


class RegressionDetector:
    """Detect performance regressions."""

    def __init__(self, baseline_rtf=0.30, threshold=0.20):
        """
        Initialize detector.

        Args:
            baseline_rtf: Target RTF value
            threshold: Regression threshold (e.g., 0.20 = 20% slower than baseline)
        """
        self.baseline_rtf = baseline_rtf
        self.threshold = threshold
        self.max_rtf = baseline_rtf * (1 + threshold)
        self.results = []

    def run_performance_test(self, num_runs=5):
        """Run performance test and collect metrics."""
        print(f"[INFO] Running performance test ({num_runs} runs)...")
        print(f"[INFO] Baseline RTF: {self.baseline_rtf:.3f}")
        print(f"[INFO] Max allowed RTF: {self.max_rtf:.3f} (+{self.threshold*100:.0f}%)")

        try:
            from f5_tts.api import F5TTS
        except ImportError as e:
            print(f"[ERROR] Failed to import F5TTS: {e}")
            return False

        # Load model
        device = "cuda" if torch.cuda.is_available() else "cpu"
        print(f"[INFO] Loading model on {device}...")

        try:
            tts = F5TTS(device=device)
        except Exception as e:
            print(f"[ERROR] Failed to load model: {e}")
            return False

        # Reference audio
        ref_audio = self._find_reference_audio()
        if not ref_audio:
            print("[ERROR] Reference audio not found")
            return False

        ref_text = "This is a reference audio for testing the text to speech system."
        gen_text = "The quick brown fox jumps over the lazy dog. This is a test."

        # Run tests
        rtfs = []
        synthesis_times = []
        memory_usage = []

        for i in range(num_runs):
            print(f"\n[RUN {i+1}/{num_runs}]")

            # Measure memory before
            if torch.cuda.is_available():
                torch.cuda.empty_cache()
                mem_before = torch.cuda.memory_allocated()

            # Run synthesis
            start_time = time.time()

            try:
                wav, sr, _ = tts.infer(
                    ref_file=ref_audio,
                    ref_text=ref_text,
                    gen_text=gen_text,
                    nfe_step=8,
                    seed=42 + i
                )
            except Exception as e:
                print(f"[ERROR] Synthesis failed: {e}")
                return False

            synthesis_time = time.time() - start_time

            # Measure memory after
            if torch.cuda.is_available():
                mem_after = torch.cuda.memory_allocated()
                mem_used = (mem_after - mem_before) / 1e6  # MB
                memory_usage.append(mem_used)

            # Calculate metrics
            audio_duration = len(wav) / sr
            rtf = synthesis_time / audio_duration

            rtfs.append(rtf)
            synthesis_times.append(synthesis_time)

            print(f"  Synthesis time: {synthesis_time:.3f}s")
            print(f"  Audio duration: {audio_duration:.3f}s")
            print(f"  RTF: {rtf:.3f}")
            if torch.cuda.is_available():
                print(f"  Memory used: {mem_used:.1f} MB")

        # Analyze results
        mean_rtf = np.mean(rtfs)
        std_rtf = np.std(rtfs)
        min_rtf = np.min(rtfs)
        max_rtf_measured = np.max(rtfs)

        print(f"\n{'='*60}")
        print("RESULTS")
        print(f"{'='*60}")
        print(f"Mean RTF:     {mean_rtf:.3f}")
        print(f"Std RTF:      {std_rtf:.3f}")
        print(f"Min RTF:      {min_rtf:.3f}")
        print(f"Max RTF:      {max_rtf_measured:.3f}")
        print(f"Variance:     ±{(std_rtf/mean_rtf)*100:.1f}%")

        if torch.cuda.is_available():
            print(f"Mean memory:  {np.mean(memory_usage):.1f} MB")

        # Check for regressions
        passed = True
        issues = []

        print(f"\n{'='*60}")
        print("REGRESSION CHECKS")
        print(f"{'='*60}")

        # Check 1: Mean RTF
        if mean_rtf > self.max_rtf:
            issues.append(f"Mean RTF {mean_rtf:.3f} exceeds threshold {self.max_rtf:.3f}")
            print(f"❌ FAIL: {issues[-1]}")
            passed = False
        else:
            print(f"✅ PASS: Mean RTF {mean_rtf:.3f} within threshold {self.max_rtf:.3f}")

        # Check 2: Max RTF (worst case)
        max_allowed_worst = self.baseline_rtf * 1.5  # Allow 50% worst case
        if max_rtf_measured > max_allowed_worst:
            issues.append(f"Max RTF {max_rtf_measured:.3f} exceeds worst-case {max_allowed_worst:.3f}")
            print(f"❌ FAIL: {issues[-1]}")
            passed = False
        else:
            print(f"✅ PASS: Max RTF {max_rtf_measured:.3f} within worst-case {max_allowed_worst:.3f}")

        # Check 3: Variance (should be < 20%)
        variance_pct = (std_rtf / mean_rtf) * 100
        if variance_pct > 20:
            issues.append(f"Variance {variance_pct:.1f}% too high (>20%)")
            print(f"⚠️  WARN: {issues[-1]}")
            print(f"         (May indicate GPU not locked or thermal throttling)")
        else:
            print(f"✅ PASS: Variance {variance_pct:.1f}% acceptable (<20%)")

        # Check 4: Memory stability
        if torch.cuda.is_available() and len(memory_usage) > 1:
            # Check if memory usage is increasing
            first_half = np.mean(memory_usage[:len(memory_usage)//2])
            second_half = np.mean(memory_usage[len(memory_usage)//2:])
            memory_increase = second_half - first_half

            if memory_increase > 50:  # > 50 MB increase
                issues.append(f"Memory increasing: {memory_increase:.1f} MB")
                print(f"⚠️  WARN: {issues[-1]}")
            else:
                print(f"✅ PASS: Memory stable (±{memory_increase:.1f} MB)")

        # Save results
        self._save_results(mean_rtf, std_rtf, min_rtf, max_rtf_measured, issues)

        print(f"\n{'='*60}")
        if passed:
            print("✅ ALL CHECKS PASSED")
        else:
            print("❌ REGRESSION DETECTED")
            print("\nIssues found:")
            for issue in issues:
                print(f"  - {issue}")
        print(f"{'='*60}\n")

        return passed

    def _find_reference_audio(self):
        """Find reference audio file."""
        # Try common locations
        locations = [
            Path("data/voices/walter_reference.wav"),
            Path("data/voices/demo_reference.wav"),
            Path("data/voices/ishow_ref.wav"),
        ]

        for loc in locations:
            if loc.exists():
                print(f"[INFO] Using reference audio: {loc}")
                return str(loc)

        # Try to find any .wav file in data/voices/
        voices_dir = Path("data/voices")
        if voices_dir.exists():
            wav_files = list(voices_dir.glob("*.wav"))
            if wav_files:
                print(f"[INFO] Using reference audio: {wav_files[0]}")
                return str(wav_files[0])

        return None

    def _save_results(self, mean_rtf, std_rtf, min_rtf, max_rtf, issues):
        """Save results to JSON file."""
        results_dir = Path("logs/regression")
        results_dir.mkdir(parents=True, exist_ok=True)

        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        result_file = results_dir / f"regression_{timestamp}.json"

        result = {
            "timestamp": datetime.now().isoformat(),
            "baseline_rtf": self.baseline_rtf,
            "max_allowed_rtf": self.max_rtf,
            "mean_rtf": float(mean_rtf),
            "std_rtf": float(std_rtf),
            "min_rtf": float(min_rtf),
            "max_rtf": float(max_rtf),
            "passed": len(issues) == 0,
            "issues": issues,
            "device": "cuda" if torch.cuda.is_available() else "cpu",
        }

        with open(result_file, 'w') as f:
            json.dump(result, f, indent=2)

        print(f"\n[INFO] Results saved to: {result_file}")


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(description="Detect performance regressions")
    parser.add_argument('--baseline', type=float, default=0.30,
                       help='Baseline RTF (target performance)')
    parser.add_argument('--threshold', type=float, default=0.20,
                       help='Regression threshold (0.20 = 20%% slower)')
    parser.add_argument('--runs', type=int, default=5,
                       help='Number of test runs')

    args = parser.parse_args()

    detector = RegressionDetector(
        baseline_rtf=args.baseline,
        threshold=args.threshold
    )

    print("="*60)
    print("iShowTTS Performance Regression Detection")
    print("="*60)
    print()

    # Check GPU
    if torch.cuda.is_available():
        print(f"[INFO] CUDA available: {torch.cuda.get_device_name(0)}")
        print(f"[INFO] PyTorch version: {torch.__version__}")
    else:
        print("[WARN] CUDA not available, using CPU")

    print()

    # Run detection
    passed = detector.run_performance_test(num_runs=args.runs)

    # Exit with appropriate code
    sys.exit(0 if passed else 1)


if __name__ == '__main__':
    main()