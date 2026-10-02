#!/usr/bin/env python3
"""Synthesises the start/stop sounds (CC0): two short sine notes with smooth fades."""

import math
import struct
import wave
from pathlib import Path

RATE = 48_000
NOTE_S = 0.07
FADE_S = 0.008


def note(freq: float) -> list[float]:
    n = int(RATE * NOTE_S)
    fade = int(RATE * FADE_S)
    out = []
    for i in range(n):
        env = min(1.0, i / fade, (n - 1 - i) / fade)
        out.append(0.6 * env * math.sin(2 * math.pi * freq * i / RATE))
    return out


def write(name: str, freqs: tuple[float, float]) -> None:
    samples = note(freqs[0]) + [0.0] * int(RATE * 0.02) + note(freqs[1])
    path = Path(__file__).resolve().parent.parent / "data" / "sounds" / name
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(RATE)
        w.writeframes(b"".join(struct.pack("<h", int(s * 32767)) for s in samples))


write("start.wav", (660.0, 880.0))  # rising: listening
write("stop.wav", (880.0, 660.0))  # falling: done
