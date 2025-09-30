#!/usr/bin/env python3
"""
NFE=6 Quality Evaluation Script

Generates audio samples with NFE=6 and NFE=7 for quality comparison.
Saves samples to .agent/quality_samples/nfe6_vs_nfe7/ for manual evaluation.

Usage:
    python scripts/test_nfe6_quality.py
"""

import time
import torch
import sys
import os
from pathlib import Path
from datetime import datetime

# Add F5-TTS to Python path
sys.path.insert(0, str(Path(__file__).parent.parent / "third_party" / "F5-TTS" / "src"))

from f5_tts.api import F5TTS

# Test sentences covering various scenarios
TEST_SENTENCES = [
    # Short (1-2s)
    ("short_1", "Hello world!"),
    ("short_2", "How are you doing today?"),
    ("short_3", "This is a test."),
    ("short_4", "Thanks for watching!"),
    ("short_5", "See you next time!"),

    # Medium (3-5s)
    ("medium_1", "The quick brown fox jumps over the lazy dog."),
    ("medium_2", "Machine learning is transforming the world of technology."),
    ("medium_3", "Welcome to the livestream, thanks for joining us today!"),
    ("medium_4", "Please subscribe and hit the notification bell."),
    ("medium_5", "Let me know what you think in the comments below."),

    # Long (6-10s)
    ("long_1", "In the field of artificial intelligence and machine learning, "
               "we are witnessing rapid advancements that are reshaping our future."),
    ("long_2", "Text to speech technology has come a long way in recent years, "
               "and now we can generate natural sounding voices in real time."),
    ("long_3", "The optimization process involved multiple iterations and careful "
               "tuning to achieve the best balance between speed and quality."),

    # Technical (numbers, acronyms)
    ("tech_1", "The model achieved an RTF of 0.213 with NFE equals 7."),
    ("tech_2", "Version 2.5.0 includes PyTorch CUDA optimization."),
    ("tech_3", "GPU utilization reached 85% during peak load."),

    # Emotional/Varied intonation
    ("emotion_1", "Wow, that's absolutely amazing!"),
    ("emotion_2", "I'm sorry to hear that happened."),
    ("emotion_3", "Congratulations on your success!"),
    ("emotion_4", "Oh no, that's terrible!"),
    ("emotion_5", "Yes! We did it! Great job everyone!"),

    # Livestream/Gaming context
    ("stream_1", "Welcome raider! Thanks for the raid!"),
    ("stream_2", "Let's check out this new game mode."),
    ("stream_3", "What should we do next? Let me know in chat."),
    ("stream_4", "That was clutch! Nice play!"),
    ("stream_5", "GG everyone, that was a great match!"),
]

def main():
    print("="*70)
    print("NFE=6 vs NFE=7 Quality Comparison Test")
    print("="*70)
    print(f"PyTorch: {torch.__version__}")
    print(f"CUDA: {torch.cuda.is_available()}")
    print(f"Number of test cases: {len(TEST_SENTENCES)}")
    print()

    # Setup output directory
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    output_dir = Path(__file__).parent.parent / ".agent" / "quality_samples" / f"nfe6_vs_nfe7_{timestamp}"
    output_dir.mkdir(parents=True, exist_ok=True)

    nfe6_dir = output_dir / "nfe6"
    nfe7_dir = output_dir / "nfe7"
    nfe6_dir.mkdir(exist_ok=True)
    nfe7_dir.mkdir(exist_ok=True)

    print(f"Output directory: {output_dir}")
    print()

    # Reference audio (use project reference)
    ref_audio = "/ssd/ishowtts/data/voices/walter_reference.wav"
    ref_text = "No, you clearly don't know who you're talking to, so let me clue you in. I am not in danger, Skyler. I am the danger. A guy opens his door and gets shot, and you think that of me? No, I am."

    # Initialize model
    print("[Init Model]")
    start = time.time()
    model = F5TTS()
    init_time = time.time() - start
    print(f"Init: {init_time:.2f}s")
    print()

    # Warmup
    print("[Warmup]")
    _ = model.infer(
        ref_file=ref_audio,
        ref_text=ref_text,
        gen_text="Warmup test.",
        show_info=lambda x: None,
        nfe_step=7,
    )
    print("Warmup complete")
    print()

    # Generate samples for each test case
    nfe6_times = []
    nfe7_times = []

    print("[Generating Samples]")
    print("-"*70)

    for i, (name, text) in enumerate(TEST_SENTENCES, 1):
        print(f"\n{i}/{len(TEST_SENTENCES)}: {name}")
        print(f"Text: {text}")

        # NFE=7 (baseline)
        torch.cuda.synchronize()
        start_time = time.time()

        wav_nfe7, sr, _ = model.infer(
            ref_file=ref_audio,
            ref_text=ref_text,
            gen_text=text,
            show_info=lambda x: None,
            nfe_step=7,
        )

        torch.cuda.synchronize()
        time_nfe7 = time.time() - start_time
        nfe7_times.append(time_nfe7)

        duration_nfe7 = len(wav_nfe7) / sr
        rtf_nfe7 = time_nfe7 / duration_nfe7

        # Save NFE=7 sample
        output_path_nfe7 = nfe7_dir / f"{name}_nfe7.wav"
        import scipy.io.wavfile
        scipy.io.wavfile.write(str(output_path_nfe7), sr, wav_nfe7)

        print(f"  NFE=7: {time_nfe7:.3f}s | RTF: {rtf_nfe7:.3f} | Duration: {duration_nfe7:.2f}s")

        # NFE=6 (test)
        torch.cuda.synchronize()
        start_time = time.time()

        wav_nfe6, sr, _ = model.infer(
            ref_file=ref_audio,
            ref_text=ref_text,
            gen_text=text,
            show_info=lambda x: None,
            nfe_step=6,
        )

        torch.cuda.synchronize()
        time_nfe6 = time.time() - start_time
        nfe6_times.append(time_nfe6)

        duration_nfe6 = len(wav_nfe6) / sr
        rtf_nfe6 = time_nfe6 / duration_nfe6

        # Save NFE=6 sample
        output_path_nfe6 = nfe6_dir / f"{name}_nfe6.wav"
        scipy.io.wavfile.write(str(output_path_nfe6), sr, wav_nfe6)

        speedup = time_nfe7 / time_nfe6
        print(f"  NFE=6: {time_nfe6:.3f}s | RTF: {rtf_nfe6:.3f} | Duration: {duration_nfe6:.2f}s")
        print(f"  Speedup: {speedup:.2f}x ({(speedup-1)*100:.1f}% faster)")

    # Summary statistics
    print("\n" + "="*70)
    print("SUMMARY")
    print("="*70)

    mean_time_nfe7 = sum(nfe7_times) / len(nfe7_times)
    mean_time_nfe6 = sum(nfe6_times) / len(nfe6_times)
    overall_speedup = mean_time_nfe7 / mean_time_nfe6

    print(f"\nNFE=7 Mean Time: {mean_time_nfe7:.3f}s")
    print(f"NFE=6 Mean Time: {mean_time_nfe6:.3f}s")
    print(f"Overall Speedup: {overall_speedup:.2f}x ({(overall_speedup-1)*100:.1f}% faster)")
    print()
    print(f"Samples saved to: {output_dir}")
    print()
    print("="*70)
    print("QUALITY EVALUATION INSTRUCTIONS")
    print("="*70)
    print()
    print("1. Listen to each pair of samples:")
    print(f"   - {nfe6_dir}")
    print(f"   - {nfe7_dir}")
    print()
    print("2. Evaluate each sample on:")
    print("   - Naturalness (1-5): How human-like does it sound?")
    print("   - Clarity (1-5): Is speech clear and intelligible?")
    print("   - Artifacts (Y/N): Any clicks, pops, robotic sounds?")
    print("   - Prosody (1-5): Is intonation and rhythm natural?")
    print()
    print("3. Overall comparison:")
    print("   - Is NFE=6 quality acceptable for production?")
    print("   - Is the quality difference noticeable?")
    print("   - Would you accept 14% speed gain for this quality?")
    print()
    print("4. Decision criteria:")
    print("   ✅ ACCEPT NFE=6 if:")
    print("      - No obvious artifacts")
    print("      - Quality drop < 10% subjectively")
    print("      - Naturalness >= 4/5 for most samples")
    print()
    print("   ❌ REJECT NFE=6 if:")
    print("      - Frequent artifacts (clicks, pops)")
    print("      - Quality drop > 15% subjectively")
    print("      - Naturalness < 3/5 for many samples")
    print()
    print("="*70)

    # Create evaluation template
    eval_template_path = output_dir / "EVALUATION_TEMPLATE.txt"
    with open(eval_template_path, "w") as f:
        f.write("NFE=6 Quality Evaluation\n")
        f.write("="*70 + "\n\n")
        f.write("Evaluator: _______________\n")
        f.write("Date: _______________\n\n")
        f.write("Rating Scale:\n")
        f.write("5 = Excellent, 4 = Good, 3 = Acceptable, 2 = Poor, 1 = Unacceptable\n\n")
        f.write("-"*70 + "\n\n")

        for name, text in TEST_SENTENCES:
            f.write(f"Test: {name}\n")
            f.write(f"Text: {text}\n")
            f.write(f"NFE=7: Naturalness __ | Clarity __ | Artifacts (Y/N) __ | Prosody __\n")
            f.write(f"NFE=6: Naturalness __ | Clarity __ | Artifacts (Y/N) __ | Prosody __\n")
            f.write(f"Notes: _________________________________________________\n")
            f.write("\n")

        f.write("-"*70 + "\n\n")
        f.write("Overall Assessment:\n\n")
        f.write("Quality difference noticeable? (Y/N) __\n")
        f.write("NFE=6 acceptable for production? (Y/N) __\n")
        f.write("Accept 14% speedup for this quality? (Y/N) __\n\n")
        f.write("Final Decision: ACCEPT / REJECT NFE=6\n\n")
        f.write("Recommendation:\n")
        f.write("_________________________________________________________________\n")
        f.write("_________________________________________________________________\n")

    print(f"Evaluation template saved to: {eval_template_path}")
    print()

if __name__ == "__main__":
    main()