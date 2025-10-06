from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional

import torch
import torchaudio

from .generator import Generator, Segment, load_generator


@dataclass
class SegmentContext:
    speaker: int
    text: str
    audio_path: Path


class CsmRuntime:
    def __init__(
        self,
        model_path: str,
        *,
        device: str = "cuda",
        dtype: str = "bfloat16",
        cache_dir: Optional[str] = None,
        load_watermark: bool = True,
    ) -> None:
        dtype_obj = getattr(torch, dtype)
        self._generator = load_generator(
            model_path,
            device=device,
            dtype=dtype_obj,
            cache_dir=cache_dir,
            load_watermark=load_watermark,
        )
        self.sample_rate = self._generator.sample_rate
        self.device = device

    def _build_context(self, segments: Iterable[Dict[str, Any]]) -> List[Segment]:
        ctx: List[Segment] = []
        for segment in segments:
            audio_path = Path(segment["audio_path"]).expanduser()
            audio, sample_rate = torchaudio.load(str(audio_path))
            audio = audio.mean(dim=0)
            if sample_rate != self.sample_rate:
                audio = torchaudio.functional.resample(audio, sample_rate, self.sample_rate)
            ctx.append(
                Segment(
                    speaker=int(segment["speaker"]),
                    text=str(segment["text"]),
                    audio=audio,
                )
            )
        return ctx

    @torch.inference_mode()
    def generate(
        self,
        *,
        text: str,
        speaker: int,
        context: Optional[Iterable[Dict[str, Any]]] = None,
        max_audio_length_ms: float,
        temperature: float,
        topk: int,
    ) -> Dict[str, Any]:
        segments = self._build_context(context or [])
        audio = self._generator.generate(
            text,
            speaker,
            segments,
            max_audio_length_ms=max_audio_length_ms,
            temperature=temperature,
            topk=topk,
        )
        if audio.ndim == 0:
            audio = torch.zeros(1, device=audio.device)
        return {
            "audio": audio.cpu().numpy().astype("float32"),
            "sample_rate": self.sample_rate,
        }

    def export_state(self) -> str:
        payload = {"sample_rate": self.sample_rate, "device": self.device}
        return json.dumps(payload)
