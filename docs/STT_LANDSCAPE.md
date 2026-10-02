# Survey of local speech recognition (STT), as of 2026-10-02

Purpose: choose a second, optional model for the application, alongside Kroko FR. Criteria: local execution on CPU (GPU optional), French first, streaming preferred (otherwise a "per utterance" mode after a short pause), usable from Rust (sherpa-onnx first), redistributable weights, Flatpak build from source.

Sources are given as `[xx]` references (list at the end of the document). "Not found" means that no primary figure was found, not that the model is bad.

## 0. Read WERs with caution

- **Different datasets.** FLEURS (read Wikipedia), MLS (read audiobooks), CoVoST-2 and Common Voice (read sentences, various microphones). None resembles spontaneous dictation. Published WERs are computed on normalized text (lowercase, no punctuation).
- **Homogeneous benchmark.** The [transcribe.cpp][tc-repo] catalog measures all its GGUF ports with the same harness (FLEURS fr, Q8_0) and also gives CPU speed on a **Ryzen 7 PRO 4750U** (2020 laptop, 8 cores), as a multiple of real time (higher = faster), on English clips [tc-bench]. It is the only homogeneous FR + CPU comparison found. It is not sherpa-onnx: speed under onnxruntime may differ.
- **Our measurement.** Kroko's internal WER (19.4% raw, 22.2% end to end, D5/D6) covers 3 Common Voice clips, i.e. 36 words. Nothing can be concluded from it statistically. Before any choice, a small FR corpus must be built (≈ 100 CV clips + real dictation recordings).

## 1. Summary

| Solution | Streaming | Published FR WER (set, source) | Size | CPU real time | Code / weights license | Rust integration | Maturity |
|---|---|---|---|---|---|---|---|
| **Kroko FR** (current) `csukuangfj/sherpa-onnx-streaming-zipformer-fr-kroko-2025-08-06` | yes, 1.28 s blocks | not found (Kroko publishes nothing for the community models) [kroko] | 71 MB | yes, RTF 0.044 (internal measurement) | Apache-2.0 / CC-BY-SA, version not specified [kroko] | in place (`OnlineRecognizer`) | stable, small model |
| Zipformer FR 2023 `shaojieli/sherpa-onnx-streaming-zipformer-fr-2023-04-14` | yes, 640 ms | CV12 test 10.57% greedy [icefall-fr] | 128 MB (int8) | yes, RTF 0.089 (D5) | Apache-2.0 | in place (same engine) | 2023; UPPERCASE without punctuation |
| **Nemotron 3.5 ASR streaming 0.6B** `nvidia/nemotron-3.5-asr-streaming-0.6b` | **yes**, 80 to 1120 ms blocks | FLEURS: 9.45% (560 ms), 9.03% (1.12 s), forced language [nemotron]; 10.78% [tc-nemotron] | 682 MB (int8, sherpa) | yes: RTF ≈ 0.12 in int8 on M-series Mac [pr3671]; 10.6× on Ryzen 4750U [tc-nemotron] | Apache-2.0 / **OpenMDW-1.1** (permissive) [openmdw] | sherpa-onnx `OnlineRecognizer` + `set_option("language", …)` | June 2026, recent |
| **Parakeet TDT 0.6B v3** `nvidia/parakeet-tdt-0.6b-v3` | no (offline under sherpa) | FLEURS 5.15 / MLS 4.97 / CoVoST-2 6.05% [parakeet]; OAL FR average 5.42% [oal]; 5.30% [tc-cat] | 670 MB (int8) | yes: 13.7× on 4750U [tc-cat]; ≈ 17 to 30× in int8 under onnxruntime (third party) [pk-fastapi] | Apache-2.0 / CC-BY-4.0 | sherpa-onnx `OfflineRecognizer`, `model_type = "nemo_transducer"` | August 2025, very widespread |
| **Canary-180M-Flash** `nvidia/canary-180m-flash` | no | MLS 4.75 / MCV-16.1 8.19% [canary180] (5.88% in the metadata of the same card); FLEURS 8.53% [tc-cat] | 207 MB (int8) | yes: 21.7× on 4750U [tc-cat] | Apache-2.0 / CC-BY-4.0 | sherpa-onnx `OfflineCanaryModelConfig` (`src_lang = "fr"`, `use_pnc`) | 2025, stable |
| Canary-1B-v2 `nvidia/canary-1b-v2` | no | FLEURS 5.02 / MLS 3.36 / CoVoST-2 6.3% [canary1b]; OAL 4.83% [oal]; 5.09% [tc-cat] | ≈ 1 billion parameters | yes: 8.3× on 4750U [tc-cat] | CC-BY-4.0 | no official sherpa package (third-party conversion `YaroslavGor/sherpa-onnx-nemo-canary-1b-v2-int`); transcribe.cpp | 2025 |
| Whisper large-v3-turbo `openai/whisper-large-v3-turbo` | no (30 s windows; `whisper-stream` for pseudo-streaming) | no FR figure from OpenAI; FLEURS 5.51% [tc-cat] | 809 M parameters; ggml q5_0 574 MB, q8_0 874 MB [ggml] | borderline: 1.4× on 4750U [tc-cat] | MIT / MIT | `whisper-rs` (Unlicense, builds whisper.cpp), sherpa `OfflineWhisper`, `ct2rs` | very mature |
| Whisper large-v3 | no | FLEURS 5.55 / MCV 11.33 / MLS 5.09% [voxtral-paper]; OAL 6.36% [oal] | 1.55 billion parameters; 3.1 GB (ggml) | no: 0.97× on 4750U [tc-cat] | MIT | same | very mature |
| whisper-large-v3-french `bofenghuang/whisper-large-v3-french` | no | CV13 7.28 / MLS 3.98 / VoxPopuli 8.91 / FLEURS 4.84% [bofeng] | size of large-v3; distilled variants `-distil-dec16/8/4/2` | not at full size | MIT | ggml (whisper-rs), CTranslate2 (`ct2rs`), candle | single author |
| Vosk `vosk-model-fr-0.22` | yes | CV 14.72 / MLS 11.64 / mTEDx 13.10 / podcast 21.61% [vosk] | 1.4 GB; small model 41 MB (CV 23.95%) | yes ("server") | Apache-2.0 / Apache-2.0 | `vosk` crate 0.3.1 (MIT, 2024) + libvosk (Kaldi) | mature but frozen (2022 models); no punctuation |
| Kyutai `kyutai/stt-1b-en_fr` | **yes**, 0.5 s delay, semantic VAD | not found (neither on the card nor in the DSM paper) [kyutai-hf][dsm-paper] | ≈ 1 billion parameters; candle: 1.98 GB + Mimi 385 MB [kyutai-candle] | not documented natively; single-thread WASM 2 to 3× slower than real time [kyutai-121] | MIT + Apache-2.0 / CC-BY-4.0 [dsm] | `moshi` crate 0.6.4 (candle) + `stt-rs` example | designed for GPU/server |
| Voxtral Mini 4B Realtime `mistralai/Voxtral-Mini-4B-Realtime-2602` | **yes**, 240 ms to 2.4 s | FLEURS: 8.00% (240 ms), 6.42% (480 ms), 5.68% (960 ms) [voxtral-rt]; 6.29% [tc-cat] | 4 billion parameters; 8.9 GB BF16, ≈ 2.5 GB Q4 | **no**: 0.58× on 4750U [tc-cat] | Apache-2.0 | `voxtral-mini-realtime-rs` (Burn/wgpu, GPU), `voxtral.c` ("slow" BLAS) [voxtral-c], transcribe.cpp | Feb. 2026 |
| Voxtral Mini 3B `mistralai/Voxtral-Mini-3B-2507` | no | FLEURS 4.87 / MCV 8.92 / MLS 5.28% [voxtral-paper]; 4.51% [tc-cat] | 3 billion parameters | no: 0.56× [tc-cat] | Apache-2.0 | llama.cpp (mtmd) [llamacpp-vx], transcribe.cpp | 2025 |
| Cohere Transcribe `CohereLabs/cohere-transcribe-03-2026` | no | OAL FR average 4.05% [oal]; FLEURS 5.23% [tc-cat] | 2 billion parameters; 2.9 GB (sherpa int8) | yes: 4.3× on 4750U [tc-cat]; 9 to 11× in q4f16 according to Voxtype [voxtype] | Apache-2.0 (upstream repository subject to automatic acceptance) | sherpa-onnx `OfflineCohereTranscribeModelConfig` (`language`, `use_punct`) | March 2026; hallucinates on non-speech [cohere] |
| Qwen3-ASR 0.6B / 1.7B `Qwen/Qwen3-ASR-0.6B` | streaming via vLLM only; offline under sherpa | 0.6B: MLS 8.55 / CV 12.25%; 1.7B: MLS 5.26 / CV 8.56% [qwen3-report] | 0.6B: ≈ 985 MB (sherpa int8) | 0.6B: 5.3×; 1.7B: 2.5× [tc-cat] | Apache-2.0 | sherpa-onnx `OfflineQwen3ASRModelConfig` (with hotwords) | Jan. 2026; LLM decoder |
| Omnilingual ASR CTC 300M / 1B (Meta) | no | not found for 300M and 1B (OAL: CTC 7B v2 at 7.63%) [oal] | 365 MB (300M int8) | likely (CTC), not measured | Apache-2.0 [omni] | sherpa-onnx `OfflineOmnilingualAsrCtcModelConfig` | Nov. 2025 |
| Granite 4.0 1B speech `ibm-granite/granite-4.0-1b-speech` | no | FLEURS 8.44% [tc-cat] | 1 billion parameters | 2.75× [tc-cat] | Apache-2.0 | transcribe.cpp | 2026 |
| NeMo FastConformer hybrid en/de/es/fr `sherpa-onnx-nemo-fast-conformer-transducer-en-de-es-fr-14288` | no | not found | 107 MB (int8 archive) | likely | to be checked | sherpa-onnx (offline) | old (NeMo, 14,288 h) [ovos] |

OAL = Open ASR Leaderboard: average of CoVoST-2, FLEURS and MLS for French, on an A100 GPU [oal].

## 2. sherpa-onnx (1.13.8, the version already in use)

The Rust crate 1.13.8 already exposes everything needed: `OnlineStream::set_option`, and the `Canary`, `Qwen3ASR`, `CohereTranscribe`, `OmnilingualAsrCtc` and `Whisper` configurations of the `OfflineRecognizer` (checked in `~/.cargo/registry/.../sherpa-onnx-1.13.8/src/offline_asr.rs`, `online_asr.rs`). Nemotron 3.5 support dates from 1.13.3, Cohere from 1.12.35, Qwen3-ASR from 1.12.34 [sherpa-cl].

Archives listed in the `asr-models` release [sherpa-rel]:

| Family | Archive / HF repository (revision recorded on 2026-10-02) | API | French |
|---|---|---|---|
| Zipformer Kroko | `csukuangfj/sherpa-onnx-streaming-zipformer-fr-kroko-2025-08-06` (also en, de, es) | online | yes |
| Zipformer icefall | `shaojieli/sherpa-onnx-streaming-zipformer-fr-2023-04-14` | online | yes |
| Nemotron 3.5 | `csukuangfj2/sherpa-onnx-nemotron-3.5-asr-streaming-0.6b-{80,160,320,560,1120}ms-int8-2026-06-11` (rev. 560 ms: `ab43d895f5985b1bbab8b6eac8607fcdc05343f3`; 1120 ms: `cba1c96ca5ef0e8393b50584ae153a79145dc492`) | online | yes (40 locales, automatic detection or forced) |
| Parakeet TDT v3 | `csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8` (`2bda32ec70b097a55adaa07d9a7173915b43cc78`) | offline | yes (25 languages, automatic detection, cannot be forced) |
| Canary 180M Flash | `csukuangfj/sherpa-onnx-nemo-canary-180m-flash-en-es-de-fr-int8` (`9077164e0d3dd1d5353743e89ceaa1d3a770838c`) | offline | yes (en, de, es, fr) |
| Cohere Transcribe | `csukuangfj2/sherpa-onnx-cohere-transcribe-14-lang-int8-2026-04-01` (`156a470cf08eefe706a0004f3c52d9ee567ca7a0`) | offline | yes (14 languages, language required) |
| Qwen3-ASR 0.6B | `csukuangfj2/sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25` (`68818b2313fe77bd06f6a7c5068ff3ef59d02b8a`) | offline | yes (30 languages) |
| Whisper | `sherpa-onnx-whisper-{tiny…large-v3,turbo}`; turbo: `csukuangfj/sherpa-onnx-whisper-turbo` (≈ 1 GB in int8) | offline | yes |
| Omnilingual CTC | `csukuangfj/sherpa-onnx-omnilingual-asr-1600-languages-300M-ctc-int8-2025-11-12` | offline | yes |
| NeMo FastConformer | `…-fast-conformer-{ctc,transducer}-en-de-es-fr-14288`, `…-be-de-en-es-fr-hr-it-pl-ru-uk-20k` | offline | yes |

**Without French**: SenseVoice (zh, en, ja, ko, yue) [sensevoice], Moonshine (streaming in ar, de, en, es, ja, zh, tl, vi; no fr) [moonshine], Dolphin (40 Eastern languages and 22 Chinese dialects) [dolphin], FunASR-Nano and Fun-ASR-MLT-Nano (fr missing from the list) [funasr], Paraformer, FireRedASR, X-ASR (zh/en), Nemotron speech EN, Parakeet unified EN, MedASR (en).

The `Banafo/Kroko-ASR` repository also publishes a FR variant with 0.64 s blocks (`Kroko-FR-Community-64-L-Streaming-001.data`), in Kroko's own format and read by `kroko-ai/kroko-onnx`, a sherpa-onnx derivative [kroko]. It does not exist as an official sherpa conversion.

## 3. Families outside sherpa: highlights

- **NVIDIA Nemotron 3.5** [nemotron]: cache-aware FastConformer, streaming, language-conditioned RNN-T decoder, native punctuation and casing. The sherpa runtime filters out the `<fr-FR>` tags emitted in the text [pr3671]. FR WERs are higher than Parakeet v3's (≈ 9% versus ≈ 5% on FLEURS). That is the price of streaming. With 560 ms blocks, token quantization (the D9 problem) falls below the 1.2 s threshold.
- **Parakeet / Canary** [parakeet][canary180][canary1b]: the quality/speed reference on CPU in per-utterance mode. Parakeet v3 detects the language by itself: on a short utterance or one full of anglicisms, it may switch language (risk not measured). Canary allows forcing `src_lang = "fr"`.
- **whisper.cpp / faster-whisper**: an excellent ecosystem (`whisper-rs` builds whisper.cpp from source, which suits Flatpak). But the encoder works on fixed 30 s windows, it is the slowest model here, and it hallucinates on silence. On a modest CPU, turbo runs at 1.4× real time, i.e. about 3.5 s of processing for 5 s of speech [tc-cat]. faster-whisper is in Python; from Rust, it goes through `ct2rs` (MIT, CTranslate2).
- **Vosk** [vosk]: real lightweight streaming, but 2022 Kaldi models that are clearly less accurate, and libvosk to provide (Kaldi is heavy to build in a Flatpak). The `vosk-model-fr-0.6-linto-2.2.0` model is under AGPL, `vosk-model-small-fr-pguyot-0.3` under CC-BY-NC-SA: both are to be excluded.
- **Kyutai STT** [dsm][kyutai-hf]: the only bilingual FR/EN streaming model designed for this, with an official Rust implementation (candle, `moshi` crate). But: 2.4 GB of weights, no published FR WER, and a deployment designed for GPU (websocket server, `--features cuda`). No native CPU figure.
- **Voxtral Realtime** [voxtral-rt]: excellent in FR at 480 ms (6.42%), but 4 billion parameters. Mistral recommends a GPU with ≥ 16 GB; on a laptop CPU, it runs at 0.58× real time [tc-cat]. The Rust port (Burn) targets wgpu [voxtral-rs]. Outside the CPU criteria.
- **Cohere Transcribe** [cohere][oal]: best open FR WER at a reasonable size (OAL 4.05%), offline, 2.9 GB in int8. It requires the language and an upstream VAD (it transcribes noise). Sherpa fixed these hallucinations on silence in 1.13.8 [sherpa-cl].
- **Qwen3-ASR** [qwen3-report]: the 0.6B is worse than Parakeet v3 in FR (MLS 8.55 versus 4.97%), the 1.7B is good but slow on CPU. Its hotwords can be used for domain vocabulary.
- **transcribe.cpp** [tc-repo] (MIT, ggml, June 2026): a C/C++ library for 16 families (Parakeet, Canary, Nemotron 3.5, Cohere, Voxtral Realtime, Whisper, Granite…). Official crate `transcribe-cpp` 0.2.4 (MIT), **built from source** with CMake, with no prebuilt binary: simpler for Flathub than sherpa-onnx's prebuilt archive (D4). The project still describes itself as "in development".

## 4. Comparable Linux applications

| Application | Language / license | Engines | Notes |
|---|---|---|---|
| Speech Note `mkiol/dsnote` [dsnote] | C++/Qt, MPL-2.0, on Flathub | Vosk, whisper.cpp (+ Parakeet), faster-whisper, April-ASR… | notes, reading and translation rather than system-wide dictation |
| nerd-dictation `ideasman42/nerd-dictation` | Python, GPL-3.0 | Vosk | minimalist, not very active (last push 2025-10) |
| Handy `cjpais/Handy` [handy] | Rust (Tauri), MIT, 32.6k stars | Whisper (small to large, turbo), Parakeet v3 | types via `wtype`/`dotool`, and states that `wtype` does not work in the Ubuntu 26.04 Wayland session (GNOME): `ydotool` must be used |
| Voxtype `peteonrails/voxtype` [voxtype] | Rust, MIT | Whisper, Parakeet, Moonshine, SenseVoice, Paraformer, Dolphin, Omnilingual, Cohere, OpenVINO | push to talk; `wtype` then `dotool` then `ydotool` |
| hyprwhspr `goodroot/hyprwhspr` | Python, MIT | Cohere, Parakeet v3, Whisper, Qwen3-ASR, REST | Hyprland-oriented |
| BlahST `QuantiusBenignus/BlahST` | shell, BSD-3 | whisper.cpp | clipboard and typing |

Findings: the only local FR streaming found among them goes through Vosk (nerd-dictation, Speech Note). Recent applications (Handy, Voxtype, hyprwhspr) all work per utterance, with Parakeet v3, Cohere or Whisper. Handy reports that `wtype` does not work in the Ubuntu 26.04 Wayland session: typing through the RemoteDesktop portal, as used here, is a real differentiator.

## 5. Recommendation

Three candidates, in order of priority:

| | Nemotron 3.5 streaming (560 ms or 1120 ms) | Parakeet TDT 0.6B v3 | Canary-180M-Flash |
|---|---|---|---|
| Mode | streaming (the same as Kroko) | per utterance, text typed after each pause | per utterance |
| FR WER | FLEURS ≈ 9 to 11% | FLEURS ≈ 5.2%, MLS 5.0% | MLS 4.75%, MCV 8.19%, FLEURS 8.5% |
| Size | 682 MB | 670 MB | 207 MB |
| CPU | ≈ 10× real time | ≈ 14× (≈ 0.4 s for 5 s of speech) | ≈ 22× |
| License | OpenMDW-1.1 | CC-BY-4.0 | CC-BY-4.0 |
| Effort | **low**: same `OnlineRecognizer` and same files (encoder, decoder, joiner, tokens); add `stream.set_option("language", "fr")` and switch to 4 threads | **medium**: new `OfflineRecognizer` engine (`model_type = "nemo_transducer"`), segmentation by the energy VAD (D9) with a short pause (≈ 0.6 to 1 s), several segments per dictation (revisit D11) | same as Parakeet, with `src_lang = tgt_lang = "fr"` and `use_pnc = true` |
| Risk | repository under the secondary account `csukuangfj2`, recent model; gain over Kroko to be measured, not assumed | automatic language switching on short utterances | only 4 languages |

1. **Nemotron 3.5**, first choice: streaming and nearly identical code are kept, and multilingual support is added. It is the only modern FR streaming model, on CPU, already integrated into sherpa. Check the exact language string (`"fr"` or `"fr-FR"`, codes in the encoder's `prompt_dictionary` metadata [sherpa-nemotron]).
2. **Parakeet v3**, for the best per-utterance accuracy at a reasonable CPU cost. It is the option most likely to fix invented words such as "arporescence". If accuracy matters more than size, **Cohere Transcribe** (2.9 GB, 4 to 11× real time) is the next step up.
3. **Canary-180M-Flash**, as a lightweight variant of the per-utterance mode, with the language forced.

To rule out for CPU: Voxtral (Realtime and Mini), Kyutai (no documented CPU path), Whisper large (too slow), Vosk (accuracy).

**What the `data/models.toml` registry needs.** For each model: `repo`, `revision` (commit read from `https://huggingface.co/api/models/{repo}`, `sha` field), and for each file `path`, `size`, `sha256`. The sha256 of LFS files (`*.onnx`) is in `https://huggingface.co/api/models/{repo}/revision/{sha}?blobs=true` (`lfs.sha256`). Small non-LFS files (`tokens.txt`) must be downloaded at that revision and then run through `sha256sum`. On the code side, the following are also needed:
- new `engine` values (for example `sherpa-onnx-offline-nemo-transducer`, `sherpa-onnx-offline-canary`). The `registry.rs` test that requires `sherpa-onnx-transducer` must be relaxed;
- an optional language field (stream option for Nemotron, `src_lang` for Canary);
- for Nemotron, list only the int8 files (`encoder.int8.onnx`, `decoder.int8.onnx`, `joiner.int8.onnx`, `tokens.txt`). For Parakeet: `encoder.int8.onnx`, `decoder.int8.onnx`, `joiner.int8.onnx`, `tokens.txt`, without `test_wavs/`;
- the exact license in `license` (`OpenMDW-1.1`, `CC-BY-4.0`), to be credited in the "About" window or the preferences.

## Sources

[kroko]: https://huggingface.co/Banafo/Kroko-ASR
[icefall-fr]: https://huggingface.co/shaojieli/icefall-asr-commonvoice-fr-pruned-transducer-stateless7-streaming-2023-04-02
[nemotron]: https://huggingface.co/nvidia/nemotron-3.5-asr-streaming-0.6b
[tc-nemotron]: https://huggingface.co/handy-computer/nemotron-3.5-asr-streaming-0.6b-gguf
[pr3671]: https://github.com/k2-fsa/sherpa-onnx/pull/3671
[sherpa-nemotron]: https://github.com/k2-fsa/sherpa-onnx/blob/master/scripts/nemo/nemotron-3.5-asr-streaming-0.6b/README.md
[openmdw]: https://openmdw.ai/license/1-1/
[parakeet]: https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3
[pk-fastapi]: https://github.com/groxaxo/parakeet-tdt-0.6b-v3-fastapi-openai
[canary180]: https://huggingface.co/nvidia/canary-180m-flash
[canary1b]: https://huggingface.co/nvidia/canary-1b-v2
[oal]: https://arxiv.org/html/2510.06961v4
[tc-repo]: https://github.com/handy-computer/transcribe.cpp
[tc-bench]: https://github.com/handy-computer/transcribe.cpp/blob/main/catalog/_benchmark_profiles.json
[tc-cat]: https://huggingface.co/handy-computer
[ggml]: https://huggingface.co/ggerganov/whisper.cpp
[voxtral-paper]: https://arxiv.org/html/2507.13264
[bofeng]: https://huggingface.co/bofenghuang/whisper-large-v3-french
[vosk]: https://alphacephei.com/vosk/models
[kyutai-hf]: https://huggingface.co/kyutai/stt-1b-en_fr
[kyutai-candle]: https://huggingface.co/kyutai/stt-1b-en_fr-candle
[kyutai-121]: https://github.com/kyutai-labs/delayed-streams-modeling/issues/121
[dsm]: https://github.com/kyutai-labs/delayed-streams-modeling
[dsm-paper]: https://arxiv.org/html/2509.08753
[voxtral-rt]: https://huggingface.co/mistralai/Voxtral-Mini-4B-Realtime-2602
[voxtral-rs]: https://github.com/TrevorS/voxtral-mini-realtime-rs
[voxtral-c]: https://github.com/antirez/voxtral.c
[llamacpp-vx]: https://huggingface.co/ggml-org/Voxtral-Mini-3B-2507-GGUF
[cohere]: https://huggingface.co/CohereLabs/cohere-transcribe-03-2026
[qwen3-report]: https://arxiv.org/html/2601.21337v1
[omni]: https://huggingface.co/csukuangfj/sherpa-onnx-omnilingual-asr-1600-languages-300M-ctc-int8-2025-11-12
[ovos]: https://github.com/OpenVoiceOS/ovos-stt-plugin-sherpa-onnx
[sherpa-cl]: https://github.com/k2-fsa/sherpa-onnx/blob/master/CHANGELOG.md
[sherpa-rel]: https://github.com/k2-fsa/sherpa-onnx/releases/tag/asr-models
[sensevoice]: https://huggingface.co/FunAudioLLM/SenseVoiceSmall
[moonshine]: https://moonshine-voice.readthedocs.io/en/latest/models/available-models/
[dolphin]: https://github.com/DataoceanAI/Dolphin
[funasr]: https://huggingface.co/FunAudioLLM/Fun-ASR-MLT-Nano-2512
[dsnote]: https://github.com/mkiol/dsnote
[handy]: https://github.com/cjpais/Handy
[voxtype]: https://github.com/peteonrails/voxtype

- kroko: <https://huggingface.co/Banafo/Kroko-ASR>
- icefall-fr: <https://huggingface.co/shaojieli/icefall-asr-commonvoice-fr-pruned-transducer-stateless7-streaming-2023-04-02>
- nemotron: <https://huggingface.co/nvidia/nemotron-3.5-asr-streaming-0.6b>
- tc-nemotron: <https://huggingface.co/handy-computer/nemotron-3.5-asr-streaming-0.6b-gguf>
- pr3671: <https://github.com/k2-fsa/sherpa-onnx/pull/3671>
- sherpa-nemotron: <https://github.com/k2-fsa/sherpa-onnx/blob/master/scripts/nemo/nemotron-3.5-asr-streaming-0.6b/README.md>
- openmdw: <https://openmdw.ai/license/1-1/>
- parakeet: <https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3>
- pk-fastapi: <https://github.com/groxaxo/parakeet-tdt-0.6b-v3-fastapi-openai>
- canary180: <https://huggingface.co/nvidia/canary-180m-flash>
- canary1b: <https://huggingface.co/nvidia/canary-1b-v2>
- oal: <https://arxiv.org/html/2510.06961v4> (table 4)
- tc-repo: <https://github.com/handy-computer/transcribe.cpp>
- tc-bench: <https://github.com/handy-computer/transcribe.cpp/blob/main/catalog/_benchmark_profiles.json>
- tc-cat: <https://huggingface.co/handy-computer> (`transcribe_cpp` metadata of each `*-gguf` repository: `wer_fleurs_fr.q8_0`, `rtf_ryzen_4750u.cpu`)
- ggml: <https://huggingface.co/ggerganov/whisper.cpp>
- voxtral-paper: <https://arxiv.org/html/2507.13264> (tables 4 to 6)
- bofeng: <https://huggingface.co/bofenghuang/whisper-large-v3-french>
- vosk: <https://alphacephei.com/vosk/models>
- kyutai-hf: <https://huggingface.co/kyutai/stt-1b-en_fr>
- kyutai-candle: <https://huggingface.co/kyutai/stt-1b-en_fr-candle>
- kyutai-121: <https://github.com/kyutai-labs/delayed-streams-modeling/issues/121>
- dsm: <https://github.com/kyutai-labs/delayed-streams-modeling>
- dsm-paper: <https://arxiv.org/html/2509.08753>
- voxtral-rt: <https://huggingface.co/mistralai/Voxtral-Mini-4B-Realtime-2602>
- voxtral-rs: <https://github.com/TrevorS/voxtral-mini-realtime-rs>
- voxtral-c: <https://github.com/antirez/voxtral.c>
- llamacpp-vx: <https://huggingface.co/ggml-org/Voxtral-Mini-3B-2507-GGUF>
- cohere: <https://huggingface.co/CohereLabs/cohere-transcribe-03-2026>
- qwen3-report: <https://arxiv.org/html/2601.21337v1> (table A.2)
- omni: <https://huggingface.co/csukuangfj/sherpa-onnx-omnilingual-asr-1600-languages-300M-ctc-int8-2025-11-12>
- ovos: <https://github.com/OpenVoiceOS/ovos-stt-plugin-sherpa-onnx>
- sherpa-cl: <https://github.com/k2-fsa/sherpa-onnx/blob/master/CHANGELOG.md>
- sherpa-rel: <https://github.com/k2-fsa/sherpa-onnx/releases/tag/asr-models>
- sensevoice: <https://huggingface.co/FunAudioLLM/SenseVoiceSmall>
- moonshine: <https://moonshine-voice.readthedocs.io/en/latest/models/available-models/>
- dolphin: <https://github.com/DataoceanAI/Dolphin>
- funasr: <https://huggingface.co/FunAudioLLM/Fun-ASR-MLT-Nano-2512>
- dsnote: <https://github.com/mkiol/dsnote>
- handy: <https://github.com/cjpais/Handy>
- voxtype: <https://github.com/peteonrails/voxtype>
- Crates: `sherpa-onnx` 1.13.8 (Apache-2.0), `whisper-rs` 0.16.0 (Unlicense), `ct2rs` 0.10.1 (MIT), `vosk` 0.3.1 (MIT), `moshi` 0.6.4 (MIT/Apache-2.0), `transcribe-cpp` 0.2.4 (MIT), `transcribe-rs` 0.3.11 (MIT), `parakeet-rs` 0.3.8 (MIT/Apache-2.0), according to <https://crates.io> as of 2026-10-02.
