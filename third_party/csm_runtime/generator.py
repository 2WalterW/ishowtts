from __future__ import annotations

from dataclasses import dataclass
from typing import List, Sequence, Tuple

import torch
import torchaudio
from huggingface_hub import hf_hub_download
from moshi.models import loaders
from tokenizers.processors import TemplateProcessing
from transformers import AutoTokenizer

from .models import Model
from .watermarking import CSM_1B_GH_WATERMARK, load_watermarker, watermark


@dataclass
class Segment:
    speaker: int
    text: str
    audio: torch.Tensor  # (num_samples,) mono, sample_rate == generator.sample_rate


def _load_llama3_tokenizer() -> AutoTokenizer:
    """Load tokenizer with single-token prompt formatting."""
    tokenizer_name = "meta-llama/Llama-3.2-1B"
    tokenizer = AutoTokenizer.from_pretrained(tokenizer_name)
    bos = tokenizer.bos_token
    eos = tokenizer.eos_token
    tokenizer._tokenizer.post_processor = TemplateProcessing(
        single=f"{bos}:0 $A:0 {eos}:0",
        pair=f"{bos}:0 $A:0 {eos}:0 {bos}:1 $B:1 {eos}:1",
        special_tokens=[(bos, tokenizer.bos_token_id), (eos, tokenizer.eos_token_id)],
    )
    return tokenizer


class Generator:
    def __init__(self, model: Model, load_watermark: bool = True):
        self._model = model
        self._model.setup_caches(1)

        device = next(model.parameters()).device
        self._text_tokenizer = _load_llama3_tokenizer()

        mimi_weight = hf_hub_download(loaders.DEFAULT_REPO, loaders.MIMI_NAME)
        mimi = loaders.get_mimi(mimi_weight, device=device)
        mimi.set_num_codebooks(32)
        self._audio_tokenizer = mimi

        self._watermarker = load_watermarker(device=device) if load_watermark else None

        self.sample_rate = mimi.sample_rate
        self.device = device

    def _tokenize_text_segment(self, text: str, speaker: int) -> Tuple[torch.Tensor, torch.Tensor]:
        frame_tokens: List[torch.Tensor] = []
        frame_masks: List[torch.Tensor] = []

        text_tokens = self._text_tokenizer.encode(f"[{speaker}]{text}")
        text_frame = torch.zeros(len(text_tokens), 33, device=self.device, dtype=torch.long)
        text_frame_mask = torch.zeros(len(text_tokens), 33, device=self.device, dtype=torch.bool)
        text_frame[:, -1] = torch.tensor(text_tokens, device=self.device)
        text_frame_mask[:, -1] = True

        frame_tokens.append(text_frame)
        frame_masks.append(text_frame_mask)
        return torch.cat(frame_tokens, dim=0), torch.cat(frame_masks, dim=0)

    def _tokenize_audio(self, audio: torch.Tensor) -> Tuple[torch.Tensor, torch.Tensor]:
        assert audio.ndim == 1, "Audio must be single channel"

        audio = audio.to(self.device)
        audio_tokens = self._audio_tokenizer.encode(audio.unsqueeze(0).unsqueeze(0))[0]
        eos_frame = torch.zeros(audio_tokens.size(0), 1, device=self.device)
        audio_tokens = torch.cat([audio_tokens, eos_frame], dim=1)

        audio_frame = torch.zeros(audio_tokens.size(1), 33, device=self.device, dtype=torch.long)
        audio_frame_mask = torch.zeros(audio_tokens.size(1), 33, device=self.device, dtype=torch.bool)
        audio_frame[:, :-1] = audio_tokens.transpose(0, 1)
        audio_frame_mask[:, :-1] = True
        return audio_frame, audio_frame_mask

    def _tokenize_segment(self, segment: Segment) -> Tuple[torch.Tensor, torch.Tensor]:
        text_tokens, text_masks = self._tokenize_text_segment(segment.text, segment.speaker)
        audio_tokens, audio_masks = self._tokenize_audio(segment.audio)
        return torch.cat([text_tokens, audio_tokens], dim=0), torch.cat([text_masks, audio_masks], dim=0)

    @torch.inference_mode()
    def generate(
        self,
        text: str,
        speaker: int,
        context: Sequence[Segment],
        *,
        max_audio_length_ms: float = 90_000,
        temperature: float = 0.9,
        topk: int = 50,
    ) -> torch.Tensor:
        self._model.reset_caches()

        max_generation_len = int(max_audio_length_ms / 80)
        tokens: List[torch.Tensor] = []
        tokens_mask: List[torch.Tensor] = []
        for segment in context:
            segment_tokens, segment_tokens_mask = self._tokenize_segment(segment)
            tokens.append(segment_tokens)
            tokens_mask.append(segment_tokens_mask)

        gen_tokens, gen_tokens_mask = self._tokenize_text_segment(text, speaker)
        tokens.append(gen_tokens)
        tokens_mask.append(gen_tokens_mask)

        prompt_tokens = torch.cat(tokens, dim=0).long().to(self.device)
        prompt_tokens_mask = torch.cat(tokens_mask, dim=0).bool().to(self.device)

        samples: List[torch.Tensor] = []
        curr_tokens = prompt_tokens.unsqueeze(0)
        curr_tokens_mask = prompt_tokens_mask.unsqueeze(0)
        curr_pos = torch.arange(0, prompt_tokens.size(0), device=self.device).unsqueeze(0).long()

        max_seq_len = 2048
        max_context_len = max_seq_len - max_generation_len
        if curr_tokens.size(1) >= max_context_len:
            raise ValueError(
                f"Inputs too long, must be below max_seq_len - max_generation_len: {max_context_len}"
            )

        for _ in range(max_generation_len):
            sample = self._model.generate_frame(curr_tokens, curr_tokens_mask, curr_pos, temperature, topk)
            if torch.all(sample == 0):
                break

            samples.append(sample)

            curr_tokens = torch.cat(
                [sample, torch.zeros(1, 1, device=self.device, dtype=torch.long)],
                dim=1,
            ).unsqueeze(1)
            curr_tokens_mask = torch.cat(
                [torch.ones_like(sample, dtype=torch.bool), torch.zeros(1, 1, device=self.device, dtype=torch.bool)],
                dim=1,
            ).unsqueeze(1)
            curr_pos = curr_pos[:, -1:] + 1

        if not samples:
            return torch.zeros(0, device=self.device)

        audio_tokens = torch.stack(samples).permute(1, 2, 0)
        audio = self._audio_tokenizer.decode(audio_tokens).squeeze(0).squeeze(0)

        if self._watermarker is not None:
            audio, wm_sample_rate = watermark(
                self._watermarker,
                audio,
                self.sample_rate,
                CSM_1B_GH_WATERMARK,
            )
            audio = torchaudio.functional.resample(audio, orig_freq=wm_sample_rate, new_freq=self.sample_rate)

        return audio


def load_generator(
    model_path: str,
    *,
    device: str = "cuda",
    dtype: torch.dtype = torch.bfloat16,
    load_watermark: bool = True,
    cache_dir: str | None = None,
) -> Generator:
    model = Model.from_pretrained(model_path, cache_dir=cache_dir)
    model.to(device=device, dtype=dtype)
    generator = Generator(model, load_watermark=load_watermark)
    return generator
