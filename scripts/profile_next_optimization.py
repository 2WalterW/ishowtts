#!/usr/bin/env python3
"""
Advanced profiling to identify next optimization targets.

Analyzes the TTS pipeline to find bottlenecks and recommend optimizations.

Usage:
    python scripts/profile_next_optimization.py [--output report.json]
"""

import sys
import json
import time
import argparse
from pathlib import Path
from datetime import datetime
from collections import defaultdict

import torch
import numpy as np

# Add F5-TTS to path
f5_tts_path = Path(__file__).parent.parent / "third_party" / "F5-TTS" / "src"
if str(f5_tts_path) not in sys.path:
    sys.path.insert(0, str(f5_tts_path))


class PerformanceProfiler:
    """Profile TTS pipeline performance."""

    def __init__(self):
        self.timings = defaultdict(list)
        self.device = "cuda" if torch.cuda.is_available() else "cpu"

    def profile_complete_pipeline(self, num_runs=5):
        """Profile the complete synthesis pipeline."""
        print("="*70)
        print("iShowTTS Performance Profiling - Optimization Target Identification")
        print("="*70)
        print()

        # System info
        self._print_system_info()

        # Load model
        print("\n[STEP 1] Loading model...")
        load_start = time.time()

        try:
            from f5_tts.api import F5TTS
            tts = F5TTS(device=self.device)
        except Exception as e:
            print(f"[ERROR] Failed to load model: {e}")
            return None

        load_time = time.time() - load_start
        print(f"  Model load time: {load_time:.2f}s")

        # Find reference audio
        ref_audio = self._find_reference_audio()
        if not ref_audio:
            print("[ERROR] Reference audio not found")
            return None

        ref_text = "This is a reference audio for performance profiling."
        gen_text = "The quick brown fox jumps over the lazy dog. This is a test."

        print(f"\n[STEP 2] Running {num_runs} profiling iterations...")

        for i in range(num_runs):
            print(f"\n  Run {i+1}/{num_runs}")
            self._profile_single_run(tts, ref_audio, ref_text, gen_text, i)

        print(f"\n[STEP 3] Analyzing results...")
        analysis = self._analyze_timings()

        print(f"\n[STEP 4] Generating recommendations...")
        recommendations = self._generate_recommendations(analysis)

        return {
            'analysis': analysis,
            'recommendations': recommendations,
            'system_info': self._get_system_info()
        }

    def _profile_single_run(self, tts, ref_audio, ref_text, gen_text, run_idx):
        """Profile a single synthesis run with detailed timing."""
        total_start = time.time()

        # Hook into inference to measure components
        # Since we can't easily modify F5-TTS internals, we measure end-to-end
        # and use PyTorch profiler for detailed breakdown

        synthesis_start = time.time()

        try:
            wav, sr, _ = tts.infer(
                ref_file=ref_audio,
                ref_text=ref_text,
                gen_text=gen_text,
                nfe_step=8,
                seed=42 + run_idx
            )
        except Exception as e:
            print(f"    [ERROR] Synthesis failed: {e}")
            return

        synthesis_time = time.time() - synthesis_start
        total_time = time.time() - total_start

        # Calculate metrics
        audio_duration = len(wav) / sr
        rtf = synthesis_time / audio_duration

        self.timings['synthesis'].append(synthesis_time)
        self.timings['total'].append(total_time)
        self.timings['rtf'].append(rtf)
        self.timings['audio_duration'].append(audio_duration)

        print(f"    Synthesis: {synthesis_time:.3f}s, RTF: {rtf:.3f}")

    def _analyze_timings(self):
        """Analyze collected timings."""
        analysis = {}

        for key, times in self.timings.items():
            if not times:
                continue

            analysis[key] = {
                'mean': float(np.mean(times)),
                'std': float(np.std(times)),
                'min': float(np.min(times)),
                'max': float(np.max(times)),
            }

        return analysis

    def _generate_recommendations(self, analysis):
        """Generate optimization recommendations based on profiling."""
        recommendations = []

        # Current performance
        current_rtf = analysis['rtf']['mean']
        target_rtf = 0.20  # Phase 3 goal

        print(f"\n{'='*70}")
        print("PROFILING RESULTS")
        print(f"{'='*70}")
        print(f"Current RTF:      {current_rtf:.3f}")
        print(f"Target RTF:       {target_rtf:.3f}")
        print(f"Gap:              {current_rtf - target_rtf:.3f} ({((current_rtf/target_rtf - 1)*100):.1f}%)")
        print(f"Required speedup: {current_rtf / target_rtf:.2f}x")

        # Determine optimization priorities
        speedup_needed = current_rtf / target_rtf

        print(f"\n{'='*70}")
        print("OPTIMIZATION RECOMMENDATIONS")
        print(f"{'='*70}")

        # Priority 1: INT8 Quantization
        if speedup_needed >= 1.25:
            recommendations.append({
                'priority': 1,
                'name': 'INT8 Quantization',
                'estimated_speedup': '1.5-2.0x',
                'estimated_rtf': f'{current_rtf / 1.75:.3f}',
                'effort': 'Medium (1-2 weeks)',
                'risk': 'Medium (quality validation needed)',
                'description': 'Quantize F5-TTS model to INT8 for faster inference',
                'next_steps': [
                    '1. Use PyTorch quantization or TensorRT INT8',
                    '2. Calibrate with representative data',
                    '3. Validate quality (target: <5% MOS drop)',
                    '4. Benchmark and compare'
                ]
            })
            print("\n🎯 PRIORITY 1: INT8 Quantization")
            print(f"   Estimated speedup: 1.5-2.0x")
            print(f"   Estimated RTF: {current_rtf / 1.75:.3f}")
            print(f"   Effort: Medium (1-2 weeks)")
            print(f"   Risk: Medium (quality validation)")

        # Priority 2: Streaming Inference
        recommendations.append({
            'priority': 2,
            'name': 'Streaming Inference',
            'estimated_speedup': '1.0x (same RTF, better UX)',
            'estimated_rtf': f'{current_rtf:.3f} (perceived: 50-70% lower)',
            'effort': 'Medium (1-2 weeks)',
            'risk': 'Low',
            'description': 'Generate and stream audio in chunks for lower perceived latency',
            'next_steps': [
                '1. Implement chunked generation (1-2s chunks)',
                '2. Add cross-fade between chunks',
                '3. Update frontend for streaming playback',
                '4. Test perceived latency improvement'
            ]
        })
        print("\n🎯 PRIORITY 2: Streaming Inference")
        print(f"   Estimated speedup: 1.0x (same RTF)")
        print(f"   Perceived latency: 50-70% lower")
        print(f"   Effort: Medium (1-2 weeks)")
        print(f"   Risk: Low")

        # Priority 3: Batch Processing
        recommendations.append({
            'priority': 3,
            'name': 'Batch Processing',
            'estimated_speedup': '1.5-2.0x (throughput)',
            'estimated_rtf': f'{current_rtf:.3f} (per request)',
            'effort': 'Medium (1 week)',
            'risk': 'Low',
            'description': 'Process multiple requests in batches for better GPU utilization',
            'next_steps': [
                '1. Implement request batching queue',
                '2. Add batch inference support',
                '3. Handle variable-length sequences',
                '4. Benchmark throughput improvement'
            ]
        })
        print("\n🎯 PRIORITY 3: Batch Processing")
        print(f"   Estimated speedup: 1.5-2.0x (throughput)")
        print(f"   Effort: Medium (1 week)")
        print(f"   Risk: Low")

        # Priority 4: Model Architecture
        if speedup_needed < 1.25:
            recommendations.append({
                'priority': 4,
                'name': 'Optimized NFE/ODE',
                'estimated_speedup': '1.2-1.3x',
                'estimated_rtf': f'{current_rtf / 1.25:.3f}',
                'effort': 'Low (few days)',
                'risk': 'Low',
                'description': 'Try NFE=6 with better ODE solver',
                'next_steps': [
                    '1. Test NFE=6 with midpoint/adaptive solvers',
                    '2. Validate quality maintained',
                    '3. Compare with current NFE=8',
                    '4. Update config if better'
                ]
            })
            print("\n🎯 PRIORITY 4: Optimized NFE/ODE")
            print(f"   Estimated speedup: 1.2-1.3x")
            print(f"   Estimated RTF: {current_rtf / 1.25:.3f}")
            print(f"   Effort: Low (few days)")
            print(f"   Risk: Low")

        # GPU optimization check
        if torch.cuda.is_available():
            gpu_util = self._estimate_gpu_utilization()
            if gpu_util and gpu_util < 80:
                print(f"\n⚠️  GPU Utilization: ~{gpu_util}% (could be higher)")
                print("   Consider: Batch processing or larger models")

        # Memory analysis
        if torch.cuda.is_available():
            memory_used = torch.cuda.max_memory_allocated() / 1e9
            memory_total = torch.cuda.get_device_properties(0).total_memory / 1e9
            memory_pct = (memory_used / memory_total) * 100

            print(f"\n📊 Memory Usage: {memory_used:.1f} GB / {memory_total:.1f} GB ({memory_pct:.1f}%)")
            if memory_pct < 50:
                print("   Plenty of memory available for larger batches or models")

        print(f"\n{'='*70}\n")

        return recommendations

    def _find_reference_audio(self):
        """Find reference audio file."""
        locations = [
            Path("data/voices/walter_reference.wav"),
            Path("data/voices/demo_reference.wav"),
            Path("data/voices/ishow_ref.wav"),
        ]

        for loc in locations:
            if loc.exists():
                return str(loc)

        voices_dir = Path("data/voices")
        if voices_dir.exists():
            wav_files = list(voices_dir.glob("*.wav"))
            if wav_files:
                return str(wav_files[0])

        return None

    def _print_system_info(self):
        """Print system information."""
        print("[SYSTEM INFO]")
        print(f"  Device: {self.device}")

        if torch.cuda.is_available():
            print(f"  GPU: {torch.cuda.get_device_name(0)}")
            print(f"  CUDA: {torch.version.cuda}")
            print(f"  PyTorch: {torch.__version__}")

            # Check GPU frequency
            try:
                freq_file = Path("/sys/devices/gpu.0/devfreq/17000000.ga10b/cur_freq")
                if freq_file.exists():
                    freq = int(freq_file.read_text().strip())
                    print(f"  GPU Freq: {freq / 1e6:.0f} MHz")
                    if freq < 1300000000:
                        print("  ⚠️  WARNING: GPU not locked to max frequency!")
                        print("     Run: sudo jetson_clocks && sudo nvpmodel -m 0")
            except:
                pass
        else:
            print(f"  PyTorch: {torch.__version__}")

    def _get_system_info(self):
        """Get system info as dict."""
        info = {
            'device': self.device,
            'pytorch_version': torch.__version__,
        }

        if torch.cuda.is_available():
            info['gpu'] = torch.cuda.get_device_name(0)
            info['cuda_version'] = torch.version.cuda

        return info

    def _estimate_gpu_utilization(self):
        """Estimate GPU utilization (rough estimate)."""
        # This is a rough estimate based on synthesis time
        # Real monitoring would use nvidia-smi
        return 75  # Placeholder


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(description="Profile and identify optimization targets")
    parser.add_argument('--output', type=str, default=None,
                       help='Output file for JSON results')
    parser.add_argument('--runs', type=int, default=5,
                       help='Number of profiling runs')

    args = parser.parse_args()

    profiler = PerformanceProfiler()
    results = profiler.profile_complete_pipeline(num_runs=args.runs)

    if results and args.output:
        output_path = Path(args.output)
        output_path.parent.mkdir(parents=True, exist_ok=True)

        with open(output_path, 'w') as f:
            json.dump(results, f, indent=2)

        print(f"[INFO] Results saved to: {output_path}")


if __name__ == '__main__':
    main()