import silentcipher
import torch
import torchaudio

CSM_1B_GH_WATERMARK = [212, 211, 146, 56, 201]


def load_watermarker(device: str = "cuda") -> silentcipher.server.Model:
    return silentcipher.get_model(model_type="44.1k", device=device)


@torch.inference_mode()
def watermark(
    watermarker: silentcipher.server.Model,
    audio_array: torch.Tensor,
    sample_rate: int,
    watermark_key: list[int],
) -> tuple[torch.Tensor, int]:
    audio_array_44khz = torchaudio.functional.resample(audio_array, orig_freq=sample_rate, new_freq=44100)
    encoded, _ = watermarker.encode_wav(audio_array_44khz, 44100, watermark_key, calc_sdr=False, message_sdr=36)

    output_sample_rate = min(44100, sample_rate)
    encoded = torchaudio.functional.resample(encoded, orig_freq=44100, new_freq=output_sample_rate)
    return encoded, output_sample_rate
