#!/usr/bin/env python3
"""
Comprehensive TTS Performance Benchmark Script
Measures synthesis speed, RTF, and quality metrics
"""

import sys
import time
import json
import argparse
from pathlib import Path
from typing import Dict, List, Any

# Add F5-TTS to path
sys.path.insert(0, str(Path(__file__).parent.parent / "third_party" / "F5-TTS" / "src"))

try:
    import torch
    import numpy as np
    from f5_tts.api import F5TTS
except ImportError as e:
    print(f"Error importing dependencies: {e}")
    print("Make sure you have activated the ishowtts environment")
    sys.exit(1)


class TTSBenchmark:
    """Benchmark TTS performance with various configurations"""

    def __init__(self, config_path: Path = None):
        self.results = []
        self.config_path = config_path or Path("config/ishowtts.toml")

    def load_config(self) -> Dict[str, Any]:
        """Load configuration from TOML file"""
        try:
            import tomli
            with open(self.config_path, 'rb') as f:
                return tomli.load(f)
        except ImportError:
            print("Warning: tomli not installed, using default config")
            return {}

    def benchmark_synthesis(
        self,
        model: F5TTS,
        text: str,
        ref_audio: str,
        ref_text: str,
        nfe_steps: int = 16,
        num_runs: int = 5,
        warmup_runs: int = 1
    ) -> Dict[str, float]:
        """Benchmark synthesis with specific parameters"""

        print(f"\nBenchmarking with NFE={nfe_steps}, text_len={len(text)}")

        # Warmup runs
        for i in range(warmup_runs):
            print(f"  Warmup run {i+1}/{warmup_runs}...")
            try:
                _ = model.infer(
                    ref_file=ref_audio,
                    ref_text=ref_text,
                    gen_text=text,
                    nfe_step=nfe_steps,
                    show_info=print
                )
            except Exception as e:
                print(f"  Warmup failed: {e}")
                return {}

        # Actual benchmark runs
        times = []
        for i in range(num_runs):
            print(f"  Run {i+1}/{num_runs}...")

            if torch.cuda.is_available():
                torch.cuda.synchronize()

            start = time.perf_counter()

            try:
                wav, sr, _ = model.infer(
                    ref_file=ref_audio,
                    ref_text=ref_text,
                    gen_text=text,
                    nfe_step=nfe_steps,
                    show_info=print
                )
            except Exception as e:
                print(f"  Run failed: {e}")
                continue

            if torch.cuda.is_available():
                torch.cuda.synchronize()

            end = time.perf_counter()
            elapsed = end - start
            times.append(elapsed)

            # Calculate Real-Time Factor
            audio_duration = len(wav) / sr
            rtf = elapsed / audio_duration

            print(f"    Time: {elapsed:.3f}s, Audio: {audio_duration:.3f}s, RTF: {rtf:.3f}")

        if not times:
            return {}

        # Calculate statistics
        audio_duration = len(wav) / sr
        mean_time = np.mean(times)
        std_time = np.std(times)
        min_time = np.min(times)
        max_time = np.max(times)
        mean_rtf = mean_time / audio_duration

        return {
            "nfe_steps": nfe_steps,
            "text_length": len(text),
            "text_chars": len(text.encode("utf-8")),
            "audio_duration": audio_duration,
            "mean_time": mean_time,
            "std_time": std_time,
            "min_time": min_time,
            "max_time": max_time,
            "mean_rtf": mean_rtf,
            "speedup": 1.0 / mean_rtf,
            "num_runs": len(times)
        }

    def run_full_benchmark(self, ref_audio: str, ref_text: str):
        """Run comprehensive benchmark suite"""

        print("=" * 70)
        print("TTS Performance Benchmark")
        print("=" * 70)

        # System info
        print("\n[System Info]")
        print(f"PyTorch: {torch.__version__}")
        print(f"CUDA available: {torch.cuda.is_available()}")
        if torch.cuda.is_available():
            print(f"CUDA device: {torch.cuda.get_device_name(0)}")
            print(f"CUDA version: {torch.version.cuda}")
        print(f"torch.compile available: {hasattr(torch, 'compile')}")

        # Initialize model
        print("\n[Initializing Model]")
        start = time.perf_counter()
        model = F5TTS(model_type="F5-TTS")
        init_time = time.perf_counter() - start
        print(f"Model initialization: {init_time:.2f}s")

        # Test texts of varying lengths
        test_cases = [
            ("你好，欢迎使用语音合成系统。", "Short text (15 chars)"),
            ("今天天气真不错，阳光明媚，微风拂面，是个出门游玩的好日子。", "Medium text (30 chars)"),
            ("科技的发展日新月异，人工智能技术正在深刻地改变着我们的生活方式，从智能家居到自动驾驶，从语音助手到机器翻译，这些创新应用让我们的日常生活变得更加便捷高效。", "Long text (70 chars)")
        ]

        # NFE steps to test
        nfe_configs = [8, 16, 32]

        results = []

        for text, description in test_cases:
            print(f"\n{'='*70}")
            print(f"Test Case: {description}")
            print(f"Text: {text}")
            print(f"{'='*70}")

            for nfe in nfe_configs:
                result = self.benchmark_synthesis(
                    model=model,
                    text=text,
                    ref_audio=ref_audio,
                    ref_text=ref_text,
                    nfe_steps=nfe,
                    num_runs=3,
                    warmup_runs=1
                )

                if result:
                    result["test_case"] = description
                    result["text"] = text
                    results.append(result)

        # Print summary
        print("\n" + "=" * 70)
        print("BENCHMARK SUMMARY")
        print("=" * 70)

        print(f"\n{'Test Case':<25} {'NFE':<5} {'RTF':<8} {'Time(s)':<10} {'Speedup':<8}")
        print("-" * 70)

        for r in results:
            print(f"{r['test_case']:<25} {r['nfe_steps']:<5} "
                  f"{r['mean_rtf']:<8.3f} {r['mean_time']:<10.3f} "
                  f"{r['speedup']:<8.2f}x")

        # Save results
        output_file = Path("benchmark_results.json")
        with open(output_file, 'w') as f:
            json.dump({
                "system_info": {
                    "pytorch": torch.__version__,
                    "cuda": torch.cuda.is_available(),
                    "device": torch.cuda.get_device_name(0) if torch.cuda.is_available() else "cpu",
                    "torch_compile": hasattr(torch, 'compile')
                },
                "model_init_time": init_time,
                "results": results
            }, f, indent=2, ensure_ascii=False)

        print(f"\nResults saved to: {output_file}")

        # Performance analysis
        print("\n" + "=" * 70)
        print("PERFORMANCE ANALYSIS")
        print("=" * 70)

        for nfe in nfe_configs:
            nfe_results = [r for r in results if r['nfe_steps'] == nfe]
            if nfe_results:
                mean_rtf = np.mean([r['mean_rtf'] for r in nfe_results])
                print(f"\nNFE={nfe}: Average RTF = {mean_rtf:.3f} "
                      f"(Speedup: {1/mean_rtf:.2f}x real-time)")

                if mean_rtf < 0.3:
                    print(f"  ✓ Excellent! Faster than Whisper target (<0.3)")
                elif mean_rtf < 0.5:
                    print(f"  ✓ Good! Suitable for real-time streaming")
                elif mean_rtf < 1.0:
                    print(f"  ○ Acceptable for most use cases")
                else:
                    print(f"  ✗ Needs optimization (slower than real-time)")

        return results


def main():
    parser = argparse.ArgumentParser(description="Benchmark TTS performance")
    parser.add_argument("--ref-audio", required=True, help="Reference audio file path")
    parser.add_argument("--ref-text", required=True, help="Reference audio transcript")
    parser.add_argument("--config", help="Config file path", default="config/ishowtts.toml")

    args = parser.parse_args()

    # Validate inputs
    if not Path(args.ref_audio).exists():
        print(f"Error: Reference audio not found: {args.ref_audio}")
        sys.exit(1)

    # Run benchmark
    benchmark = TTSBenchmark(Path(args.config))
    benchmark.run_full_benchmark(args.ref_audio, args.ref_text)


if __name__ == "__main__":
    main()