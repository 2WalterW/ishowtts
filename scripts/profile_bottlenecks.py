#!/usr/bin/env python3
"""
Profile F5-TTS inference to identify bottlenecks for Phase 3 optimizations.

This script uses PyTorch profiler to analyze where time is spent during inference.
Goal: Identify opportunities for further optimization beyond Phase 1 (RTF 0.251).

Usage:
    python scripts/profile_bottlenecks.py [--output profile_results.json]
"""

import argparse
import json
import time
from pathlib import Path

import torch
import numpy as np
from torch.profiler import profile, ProfilerActivity, record_function

# Add F5-TTS to path
import sys
f5_tts_path = Path(__file__).parent.parent / "third_party" / "F5-TTS" / "src"
sys.path.insert(0, str(f5_tts_path))

from f5_tts.api import F5TTS


def profile_inference(tts_model, text: str, ref_audio: str, ref_text: str, num_runs: int = 5):
    """Profile TTS inference with PyTorch profiler."""

    print(f"\n{'='*80}")
    print("PROFILING TTS INFERENCE")
    print(f"{'='*80}\n")

    # Warmup
    print("Warming up model...")
    for _ in range(2):
        _ = tts_model.infer(
            ref_file=ref_audio,
            ref_text=ref_text,
            gen_text=text,
        )
    print("Warmup complete.\n")

    # Profile with PyTorch profiler
    print(f"Running {num_runs} profiled inferences...")

    activities = [ProfilerActivity.CPU, ProfilerActivity.CUDA]

    with profile(
        activities=activities,
        record_shapes=True,
        profile_memory=True,
        with_stack=True,
        with_flops=True,
    ) as prof:
        with record_function("tts_inference"):
            for i in range(num_runs):
                with record_function(f"inference_run_{i}"):
                    _ = tts_model.infer(
                        ref_file=ref_audio,
                        ref_text=ref_text,
                        gen_text=text,
                    )

    print("Profiling complete.\n")

    # Print summary
    print(f"\n{'='*80}")
    print("TOP 10 TIME-CONSUMING OPERATIONS (by CUDA time)")
    print(f"{'='*80}\n")
    print(prof.key_averages().table(sort_by="cuda_time_total", row_limit=10))

    print(f"\n{'='*80}")
    print("TOP 10 TIME-CONSUMING OPERATIONS (by CPU time)")
    print(f"{'='*80}\n")
    print(prof.key_averages().table(sort_by="cpu_time_total", row_limit=10))

    print(f"\n{'='*80}")
    print("TOP 10 MEMORY-INTENSIVE OPERATIONS")
    print(f"{'='*80}\n")
    print(prof.key_averages().table(sort_by="cuda_memory_usage", row_limit=10))

    # Export detailed results
    return prof


def analyze_bottlenecks(prof, output_file: str = None):
    """Analyze profiling results and identify optimization opportunities."""

    print(f"\n{'='*80}")
    print("BOTTLENECK ANALYSIS & OPTIMIZATION OPPORTUNITIES")
    print(f"{'='*80}\n")

    # Get key averages
    key_averages = prof.key_averages()

    # Sort by CUDA time
    sorted_by_cuda = sorted(key_averages, key=lambda x: x.cuda_time_total, reverse=True)

    # Calculate total time
    total_cuda_time = sum(item.cuda_time_total for item in key_averages)
    total_cpu_time = sum(item.cpu_time_total for item in key_averages)

    print(f"Total CUDA time: {total_cuda_time/1e6:.2f} ms")
    print(f"Total CPU time: {total_cpu_time/1e6:.2f} ms\n")

    # Categorize operations
    categories = {
        'model': [],
        'vocoder': [],
        'audio_processing': [],
        'memory_ops': [],
        'other': []
    }

    for item in sorted_by_cuda[:20]:  # Top 20 operations
        name = item.key.lower()
        cuda_time_ms = item.cuda_time_total / 1e6
        cpu_time_ms = item.cpu_time_total / 1e6
        cuda_pct = 100 * item.cuda_time_total / total_cuda_time if total_cuda_time > 0 else 0

        op_info = {
            'name': item.key,
            'cuda_time_ms': cuda_time_ms,
            'cpu_time_ms': cpu_time_ms,
            'cuda_percent': cuda_pct,
            'count': item.count,
        }

        # Categorize
        if any(kw in name for kw in ['conv', 'linear', 'matmul', 'attention', 'transformer']):
            categories['model'].append(op_info)
        elif any(kw in name for kw in ['vocoder', 'decode', 'istft', 'griffin']):
            categories['vocoder'].append(op_info)
        elif any(kw in name for kw in ['resample', 'audio', 'spectrogram', 'mel']):
            categories['audio_processing'].append(op_info)
        elif any(kw in name for kw in ['copy', 'memcpy', 'to', 'clone']):
            categories['memory_ops'].append(op_info)
        else:
            categories['other'].append(op_info)

    # Print categorized results
    for category, ops in categories.items():
        if ops:
            print(f"\n## {category.upper().replace('_', ' ')} ({len(ops)} operations)")
            total_cat_time = sum(op['cuda_time_ms'] for op in ops)
            print(f"Total time: {total_cat_time:.2f} ms ({100*total_cat_time/(total_cuda_time/1e6):.1f}% of total)\n")

            for i, op in enumerate(ops[:5], 1):  # Top 5 per category
                print(f"{i}. {op['name']}")
                print(f"   Time: {op['cuda_time_ms']:.2f} ms ({op['cuda_percent']:.1f}%)")
                print(f"   Count: {op['count']}")
                print()

    # Recommendations
    print(f"\n{'='*80}")
    print("OPTIMIZATION RECOMMENDATIONS FOR PHASE 3")
    print(f"{'='*80}\n")

    model_time = sum(op['cuda_time_ms'] for op in categories['model'])
    vocoder_time = sum(op['cuda_time_ms'] for op in categories['vocoder'])
    audio_time = sum(op['cuda_time_ms'] for op in categories['audio_processing'])
    memory_time = sum(op['cuda_time_ms'] for op in categories['memory_ops'])

    total_categorized = model_time + vocoder_time + audio_time + memory_time

    recommendations = []

    # Model optimization
    if model_time > total_categorized * 0.5:
        recommendations.append({
            'priority': 'HIGH',
            'category': 'Model Optimization',
            'bottleneck': f'Model operations take {model_time:.2f} ms ({100*model_time/total_categorized:.1f}%)',
            'suggestions': [
                'INT8 quantization (1.5-2x speedup potential)',
                'Model TensorRT export (1.5-2x speedup potential)',
                'Model distillation (2-3x speedup potential)',
                'Reduce model size/layers',
            ]
        })

    # Vocoder optimization
    if vocoder_time > total_categorized * 0.2:
        recommendations.append({
            'priority': 'MEDIUM',
            'category': 'Vocoder Optimization',
            'bottleneck': f'Vocoder operations take {vocoder_time:.2f} ms ({100*vocoder_time/total_categorized:.1f}%)',
            'suggestions': [
                'Note: TensorRT vocoder already tested - slower E2E',
                'Try different vocoder architectures (HiFiGAN, WaveGlow)',
                'Reduce vocoder complexity',
            ]
        })

    # Audio processing optimization
    if audio_time > total_categorized * 0.1:
        recommendations.append({
            'priority': 'MEDIUM',
            'category': 'Audio Processing',
            'bottleneck': f'Audio processing takes {audio_time:.2f} ms ({100*audio_time/total_categorized:.1f}%)',
            'suggestions': [
                'Optimize resampling (already done in Rust)',
                'Cache preprocessed reference audio (already done)',
                'Use faster audio libraries (librosa → torchaudio)',
            ]
        })

    # Memory optimization
    if memory_time > total_categorized * 0.05:
        recommendations.append({
            'priority': 'LOW',
            'category': 'Memory Operations',
            'bottleneck': f'Memory operations take {memory_time:.2f} ms ({100*memory_time/total_categorized:.1f}%)',
            'suggestions': [
                'Reduce CPU-GPU transfers',
                'Use pinned memory for faster transfers',
                'Optimize tensor shapes to avoid copies',
            ]
        })

    # Sort by priority
    priority_order = {'HIGH': 0, 'MEDIUM': 1, 'LOW': 2}
    recommendations.sort(key=lambda x: priority_order[x['priority']])

    # Print recommendations
    for i, rec in enumerate(recommendations, 1):
        print(f"{i}. [{rec['priority']}] {rec['category']}")
        print(f"   Bottleneck: {rec['bottleneck']}")
        print(f"   Suggestions:")
        for suggestion in rec['suggestions']:
            print(f"   - {suggestion}")
        print()

    # Save to file
    if output_file:
        results = {
            'total_cuda_time_ms': total_cuda_time / 1e6,
            'total_cpu_time_ms': total_cpu_time / 1e6,
            'categories': {
                'model_ms': model_time,
                'vocoder_ms': vocoder_time,
                'audio_processing_ms': audio_time,
                'memory_ops_ms': memory_time,
            },
            'recommendations': recommendations,
        }

        with open(output_file, 'w') as f:
            json.dump(results, f, indent=2)

        print(f"\nResults saved to: {output_file}")

    return recommendations


def benchmark_components(tts_model, text: str, ref_audio: str, ref_text: str):
    """Benchmark individual components to isolate bottlenecks."""

    print(f"\n{'='*80}")
    print("COMPONENT-LEVEL BENCHMARKING")
    print(f"{'='*80}\n")

    # This would require modifying F5-TTS to expose internal components
    # For now, we'll do high-level timing

    num_runs = 5
    times = {
        'total': [],
        'model': [],  # Would need instrumentation
        'vocoder': [],  # Would need instrumentation
    }

    print(f"Running {num_runs} benchmarks...")
    for i in range(num_runs):
        start = time.perf_counter()

        _ = tts_model.infer(
            ref_file=ref_audio,
            ref_text=ref_text,
            gen_text=text,
        )

        end = time.perf_counter()
        total_time = (end - start) * 1000  # ms
        times['total'].append(total_time)

        print(f"  Run {i+1}: {total_time:.2f} ms")

    print(f"\nResults:")
    print(f"  Mean: {np.mean(times['total']):.2f} ms")
    print(f"  Std:  {np.std(times['total']):.2f} ms")
    print(f"  Min:  {np.min(times['total']):.2f} ms")
    print(f"  Max:  {np.max(times['total']):.2f} ms")

    # Estimate based on previous profiling
    # Model is typically 70-80% of time, vocoder 15-25%
    mean_total = np.mean(times['total'])
    estimated_model = mean_total * 0.75
    estimated_vocoder = mean_total * 0.20
    estimated_other = mean_total * 0.05

    print(f"\n## Estimated Component Breakdown:")
    print(f"  Model (est.):     {estimated_model:.2f} ms ({100*estimated_model/mean_total:.1f}%)")
    print(f"  Vocoder (est.):   {estimated_vocoder:.2f} ms ({100*estimated_vocoder/mean_total:.1f}%)")
    print(f"  Other (est.):     {estimated_other:.2f} ms ({100*estimated_other/mean_total:.1f}%)")

    return times


def main():
    parser = argparse.ArgumentParser(description="Profile F5-TTS inference bottlenecks")
    parser.add_argument("--output", default="logs/profile_results.json", help="Output file for results")
    parser.add_argument("--num-runs", type=int, default=5, help="Number of profiling runs")
    parser.add_argument("--text", default="这是一个用来测试F5-TTS性能的句子。我们需要找到主要的性能瓶颈。", help="Text to synthesize")
    parser.add_argument("--ref-audio", default="data/voices/demo_reference.wav", help="Reference audio file")
    parser.add_argument("--ref-text", default="这是参考音频的文本内容。", help="Reference text")
    args = parser.parse_args()

    # Check CUDA
    if not torch.cuda.is_available():
        print("ERROR: CUDA not available. This script requires GPU.")
        return 1

    print(f"PyTorch version: {torch.__version__}")
    print(f"CUDA version: {torch.version.cuda}")
    print(f"Device: {torch.cuda.get_device_name(0)}")
    print(f"CUDA capability: {torch.cuda.get_device_capability(0)}")

    # Initialize model
    print("\nInitializing F5-TTS model...")
    model = F5TTS(
        model="F5TTS_v1_Base",
        ckpt_file="",
        vocab_file="",
        device="cuda",
    )
    print("Model initialized.\n")

    # Profile inference
    prof = profile_inference(model, args.text, args.ref_audio, args.ref_text, args.num_runs)

    # Analyze bottlenecks
    recommendations = analyze_bottlenecks(prof, args.output)

    # Benchmark components
    benchmark_components(model, args.text, args.ref_audio, args.ref_text)

    # Summary
    print(f"\n{'='*80}")
    print("SUMMARY & NEXT STEPS")
    print(f"{'='*80}\n")

    print("Current Status:")
    print("  - Phase 1 Complete: RTF 0.251 (target < 0.30) ✅")
    print("  - Optimizations: torch.compile + FP16 + NFE=8")
    print("  - Speedup: 3.98x (best), 3.37x (mean)")
    print()

    print("Phase 3 Target:")
    print("  - RTF < 0.20 (need 25% more speedup)")
    print("  - Current: 0.251 RTF")
    print("  - Gap: ~0.05 RTF or ~500ms for 10s audio")
    print()

    print("Recommended Optimizations (in order):")
    for i, rec in enumerate(recommendations[:3], 1):
        print(f"  {i}. [{rec['priority']}] {rec['category']}")
        print(f"     Top suggestion: {rec['suggestions'][0]}")

    print(f"\nDetailed results saved to: {args.output}")
    print("Review the profiling results and choose the most impactful optimization.")

    return 0


if __name__ == "__main__":
    exit(main())