#!/usr/bin/env python3
"""
Generate audio samples at different NFE values for quality comparison.

Generates WAV files for NFE = [6, 7, 8] for manual listening test.
"""

import time
import torch
import sys
import os
from pathlib import Path

# Add F5-TTS to Python path
sys.path.insert(0, str(Path(__file__).parent.parent / "third_party" / "F5-TTS" / "src"))

from f5_tts.api import F5TTS

def generate_sample(model, nfe_step, output_dir, test_texts, ref_audio, ref_text):
    """Generate audio samples for a specific NFE value."""
    output_dir = Path(output_dir)
    nfe_dir = output_dir / f"nfe_{nfe_step}"
    nfe_dir.mkdir(parents=True, exist_ok=True)

    print(f"\n{'='*70}")
    print(f"Generating samples for NFE={nfe_step}")
    print(f"Output directory: {nfe_dir}")
    print(f"{'='*70}")

    for i, text in enumerate(test_texts):
        print(f"\n[{i+1}/{len(test_texts)}] Generating: \"{text[:50]}...\"")

        torch.cuda.synchronize()
        start = time.time()

        wav, sr, _ = model.infer(
            ref_file=ref_audio,
            ref_text=ref_text,
            gen_text=text,
            show_info=lambda x: None,
            nfe_step=nfe_step,
        )

        torch.cuda.synchronize()
        elapsed = time.time() - start

        # Save audio file
        output_file = nfe_dir / f"sample_{i+1}.wav"
        model.export_wav(wav, str(output_file))

        audio_duration = len(wav) / sr
        rtf = elapsed / audio_duration

        print(f"  Saved: {output_file.name}")
        print(f"  Time: {elapsed:.3f}s | RTF: {rtf:.3f} | Audio: {audio_duration:.2f}s")

def main():
    print("="*70)
    print("Quality Comparison Sample Generator")
    print("="*70)

    # Configuration
    ref_audio = "/ssd/ishowtts/third_party/F5-TTS/src/f5_tts/infer/examples/basic/basic_ref_en.wav"
    ref_text = "No, you clearly don't know who you're talking to, so let me clue you in."
    output_dir = "/ssd/ishowtts/.agent/quality_samples"

    # Test texts - variety of lengths and complexity
    test_texts = [
        "Hello world, this is a test of the optimized TTS system.",
        "The quick brown fox jumps over the lazy dog near the riverbank.",
        "In the realm of artificial intelligence, text to speech synthesis has made remarkable progress in recent years.",
        "Performance optimization requires careful balancing between speed and quality to achieve the best user experience.",
    ]

    nfe_values = [6, 7, 8]

    print(f"Reference audio: {ref_audio}")
    print(f"Reference text: {ref_text}")
    print(f"Output directory: {output_dir}")
    print(f"NFE values to test: {nfe_values}")
    print(f"Number of test texts: {len(test_texts)}")

    print("\n[Init Model]")
    start = time.time()
    model = F5TTS()
    init_time = time.time() - start
    print(f"Init: {init_time:.2f}s")

    # Generate samples for each NFE value
    for nfe in nfe_values:
        generate_sample(model, nfe, output_dir, test_texts, ref_audio, ref_text)

    print("\n" + "="*70)
    print("Generation Complete!")
    print("="*70)
    print(f"\nSamples saved to: {output_dir}/")
    print("\nDirectory structure:")
    print("  nfe_6/")
    print("    sample_1.wav, sample_2.wav, sample_3.wav, sample_4.wav")
    print("  nfe_7/")
    print("    sample_1.wav, sample_2.wav, sample_3.wav, sample_4.wav")
    print("  nfe_8/")
    print("    sample_1.wav, sample_2.wav, sample_3.wav, sample_4.wav")

    print("\n📝 Quality Evaluation:")
    print("  1. Listen to samples in each directory")
    print("  2. Compare clarity, naturalness, and artifacts")
    print("  3. Rate on scale: 1 (poor) to 5 (excellent)")
    print("  4. Check if NFE=6 or NFE=7 maintains acceptable quality")

    print("\n🎯 Decision Criteria:")
    print("  - NFE=6: RTF 0.182 (31.6% faster than NFE=8)")
    print("  - NFE=7: RTF 0.213 (14.0% faster than NFE=8)")
    print("  - NFE=8: RTF 0.243 (current baseline)")
    print("\n  If NFE=7 quality ≈ NFE=8, use NFE=7 (good speedup, safer)")
    print("  If NFE=6 quality ≈ NFE=8, use NFE=6 (max speedup, phase 3 target)")

    print("="*70)

if __name__ == "__main__":
    main()