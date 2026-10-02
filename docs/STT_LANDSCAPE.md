# Panorama de la reconnaissance vocale locale (STT), état au 2026-10-02

Objet : choisir un second modèle, optionnel, pour l'application, à côté de Kroko FR. Critères : exécution locale sur CPU (GPU en option), français d'abord, streaming de préférence (sinon un mode « par énoncé » après une courte pause), utilisable depuis Rust (sherpa-onnx en priorité), poids redistribuables, construction Flatpak depuis les sources.

Les sources sont données en références `[xx]` (liste en fin de document). Un « non trouvé » signifie qu'aucun chiffre primaire n'a été trouvé, pas que le modèle est mauvais.

## 0. Lire les WER avec prudence

- **Jeux de données différents.** FLEURS (Wikipédia lu), MLS (livres audio lus), CoVoST-2 et Common Voice (phrases lues, micros variés). Aucun ne ressemble à de la dictée spontanée. Les WER publiés sont calculés sur du texte normalisé (minuscules, sans ponctuation).
- **Banc homogène.** Le catalogue de [transcribe.cpp][tc-repo] mesure tous ses portages GGUF avec le même harnais (FLEURS fr, Q8_0) et donne aussi la vitesse CPU sur un **Ryzen 7 PRO 4750U** (portable de 2020, 8 cœurs), en multiple du temps réel (plus grand = plus rapide), sur des extraits anglais [tc-bench]. C'est la seule comparaison FR + CPU homogène trouvée. Ce n'est pas sherpa-onnx : la vitesse sous onnxruntime peut différer.
- **Notre mesure.** Le WER interne de Kroko (19,4 % brut, 22,2 % en bout de chaîne, D5/D6) porte sur 3 extraits Common Voice, soit 36 mots. On ne peut rien en conclure statistiquement. Avant tout choix, il faut constituer un petit corpus FR (≈ 100 extraits CV + enregistrements réels de dictée).

## 1. Synthèse

| Solution | Streaming | WER FR publié (jeu, source) | Taille | CPU temps réel | Licence code / poids | Intégration Rust | Maturité |
|---|---|---|---|---|---|---|---|
| **Kroko FR** (actuel) `csukuangfj/sherpa-onnx-streaming-zipformer-fr-kroko-2025-08-06` | oui, blocs de 1,28 s | non trouvé (Kroko ne publie rien pour les modèles community) [kroko] | 71 Mo | oui, RTF 0,044 (mesure interne) | Apache-2.0 / CC-BY-SA, version non précisée [kroko] | en place (`OnlineRecognizer`) | stable, petit modèle |
| Zipformer FR 2023 `shaojieli/sherpa-onnx-streaming-zipformer-fr-2023-04-14` | oui, 640 ms | CV12 test 10,57 % en glouton [icefall-fr] | 128 Mo (int8) | oui, RTF 0,089 (D5) | Apache-2.0 | en place (même moteur) | 2023 ; MAJUSCULES sans ponctuation |
| **Nemotron 3.5 ASR streaming 0.6B** `nvidia/nemotron-3.5-asr-streaming-0.6b` | **oui**, blocs de 80 à 1120 ms | FLEURS : 9,45 % (560 ms), 9,03 % (1,12 s), langue forcée [nemotron] ; 10,78 % [tc-nemotron] | 682 Mo (int8, sherpa) | oui : RTF ≈ 0,12 en int8 sur Mac série M [pr3671] ; 10,6× sur Ryzen 4750U [tc-nemotron] | Apache-2.0 / **OpenMDW-1.1** (permissive) [openmdw] | sherpa-onnx `OnlineRecognizer` + `set_option("language", …)` | juin 2026, récent |
| **Parakeet TDT 0.6B v3** `nvidia/parakeet-tdt-0.6b-v3` | non (hors ligne sous sherpa) | FLEURS 5,15 / MLS 4,97 / CoVoST-2 6,05 % [parakeet] ; moyenne FR de l'OAL 5,42 % [oal] ; 5,30 % [tc-cat] | 670 Mo (int8) | oui : 13,7× sur 4750U [tc-cat] ; ≈ 17 à 30× en int8 sous onnxruntime (tiers) [pk-fastapi] | Apache-2.0 / CC-BY-4.0 | sherpa-onnx `OfflineRecognizer`, `model_type = "nemo_transducer"` | août 2025, très répandu |
| **Canary-180M-Flash** `nvidia/canary-180m-flash` | non | MLS 4,75 / MCV-16.1 8,19 % [canary180] (5,88 % dans les métadonnées de la même carte) ; FLEURS 8,53 % [tc-cat] | 207 Mo (int8) | oui : 21,7× sur 4750U [tc-cat] | Apache-2.0 / CC-BY-4.0 | sherpa-onnx `OfflineCanaryModelConfig` (`src_lang = "fr"`, `use_pnc`) | 2025, stable |
| Canary-1B-v2 `nvidia/canary-1b-v2` | non | FLEURS 5,02 / MLS 3,36 / CoVoST-2 6,3 % [canary1b] ; OAL 4,83 % [oal] ; 5,09 % [tc-cat] | ≈ 1 milliard de paramètres | oui : 8,3× sur 4750U [tc-cat] | CC-BY-4.0 | pas de paquet sherpa officiel (conversion tierce `YaroslavGor/sherpa-onnx-nemo-canary-1b-v2-int`) ; transcribe.cpp | 2025 |
| Whisper large-v3-turbo `openai/whisper-large-v3-turbo` | non (fenêtres de 30 s ; `whisper-stream` en pseudo-streaming) | aucun chiffre FR d'OpenAI ; FLEURS 5,51 % [tc-cat] | 809 M de paramètres ; ggml q5_0 574 Mo, q8_0 874 Mo [ggml] | limite : 1,4× sur 4750U [tc-cat] | MIT / MIT | `whisper-rs` (Unlicense, compile whisper.cpp), sherpa `OfflineWhisper`, `ct2rs` | très mature |
| Whisper large-v3 | non | FLEURS 5,55 / MCV 11,33 / MLS 5,09 % [voxtral-paper] ; OAL 6,36 % [oal] | 1,55 milliard de paramètres ; 3,1 Go (ggml) | non : 0,97× sur 4750U [tc-cat] | MIT | idem | très mature |
| whisper-large-v3-french `bofenghuang/whisper-large-v3-french` | non | CV13 7,28 / MLS 3,98 / VoxPopuli 8,91 / FLEURS 4,84 % [bofeng] | taille de large-v3 ; variantes distillées `-distil-dec16/8/4/2` | non en taille pleine | MIT | ggml (whisper-rs), CTranslate2 (`ct2rs`), candle | un seul auteur |
| Vosk `vosk-model-fr-0.22` | oui | CV 14,72 / MLS 11,64 / mTEDx 13,10 / podcast 21,61 % [vosk] | 1,4 Go ; petit modèle 41 Mo (CV 23,95 %) | oui (« serveur ») | Apache-2.0 / Apache-2.0 | crate `vosk` 0.3.1 (MIT, 2024) + libvosk (Kaldi) | mature mais figé (modèles de 2022) ; pas de ponctuation |
| Kyutai `kyutai/stt-1b-en_fr` | **oui**, délai 0,5 s, VAD sémantique | non trouvé (ni sur la carte ni dans le papier DSM) [kyutai-hf][dsm-paper] | ≈ 1 milliard de paramètres ; candle : 1,98 Go + Mimi 385 Mo [kyutai-candle] | non documenté en natif ; WASM sur un seul thread 2 à 3× plus lent que le temps réel [kyutai-121] | MIT + Apache-2.0 / CC-BY-4.0 [dsm] | crate `moshi` 0.6.4 (candle) + exemple `stt-rs` | conçu pour GPU/serveur |
| Voxtral Mini 4B Realtime `mistralai/Voxtral-Mini-4B-Realtime-2602` | **oui**, 240 ms à 2,4 s | FLEURS : 8,00 % (240 ms), 6,42 % (480 ms), 5,68 % (960 ms) [voxtral-rt] ; 6,29 % [tc-cat] | 4 milliards de paramètres ; 8,9 Go BF16, ≈ 2,5 Go Q4 | **non** : 0,58× sur 4750U [tc-cat] | Apache-2.0 | `voxtral-mini-realtime-rs` (Burn/wgpu, GPU), `voxtral.c` (BLAS « lent ») [voxtral-c], transcribe.cpp | fév. 2026 |
| Voxtral Mini 3B `mistralai/Voxtral-Mini-3B-2507` | non | FLEURS 4,87 / MCV 8,92 / MLS 5,28 % [voxtral-paper] ; 4,51 % [tc-cat] | 3 milliards de paramètres | non : 0,56× [tc-cat] | Apache-2.0 | llama.cpp (mtmd) [llamacpp-vx], transcribe.cpp | 2025 |
| Cohere Transcribe `CohereLabs/cohere-transcribe-03-2026` | non | moyenne FR de l'OAL 4,05 % [oal] ; FLEURS 5,23 % [tc-cat] | 2 milliards de paramètres ; 2,9 Go (sherpa int8) | oui : 4,3× sur 4750U [tc-cat] ; 9 à 11× en q4f16 selon Voxtype [voxtype] | Apache-2.0 (dépôt amont soumis à une acceptation automatique) | sherpa-onnx `OfflineCohereTranscribeModelConfig` (`language`, `use_punct`) | mars 2026 ; hallucine sur du non-parole [cohere] |
| Qwen3-ASR 0.6B / 1.7B `Qwen/Qwen3-ASR-0.6B` | streaming via vLLM seulement ; hors ligne sous sherpa | 0.6B : MLS 8,55 / CV 12,25 % ; 1.7B : MLS 5,26 / CV 8,56 % [qwen3-report] | 0.6B : ≈ 985 Mo (sherpa int8) | 0.6B : 5,3× ; 1.7B : 2,5× [tc-cat] | Apache-2.0 | sherpa-onnx `OfflineQwen3ASRModelConfig` (avec hotwords) | janv. 2026 ; décodeur LLM |
| Omnilingual ASR CTC 300M / 1B (Meta) | non | non trouvé pour 300M et 1B (OAL : CTC 7B v2 à 7,63 %) [oal] | 365 Mo (300M int8) | probable (CTC), non mesuré | Apache-2.0 [omni] | sherpa-onnx `OfflineOmnilingualAsrCtcModelConfig` | nov. 2025 |
| Granite 4.0 1B speech `ibm-granite/granite-4.0-1b-speech` | non | FLEURS 8,44 % [tc-cat] | 1 milliard de paramètres | 2,75× [tc-cat] | Apache-2.0 | transcribe.cpp | 2026 |
| NeMo FastConformer hybride en/de/es/fr `sherpa-onnx-nemo-fast-conformer-transducer-en-de-es-fr-14288` | non | non trouvé | 107 Mo (archive int8) | probable | à vérifier | sherpa-onnx (hors ligne) | ancien (NeMo, 14 288 h) [ovos] |

OAL = Open ASR Leaderboard : moyenne de CoVoST-2, FLEURS et MLS pour le français, sur GPU A100 [oal].

## 2. sherpa-onnx (1.13.8, la version déjà utilisée)

La crate Rust 1.13.8 expose déjà tout le nécessaire : `OnlineStream::set_option`, et les configurations `Canary`, `Qwen3ASR`, `CohereTranscribe`, `OmnilingualAsrCtc` et `Whisper` de l'`OfflineRecognizer` (vérifié dans `~/.cargo/registry/.../sherpa-onnx-1.13.8/src/offline_asr.rs`, `online_asr.rs`). Le support de Nemotron 3.5 date de la 1.13.3, celui de Cohere de la 1.12.35, celui de Qwen3-ASR de la 1.12.34 [sherpa-cl].

Archives listées dans la release `asr-models` [sherpa-rel] :

| Famille | Archive / dépôt HF (révision relevée le 2026-10-02) | API | Français |
|---|---|---|---|
| Zipformer Kroko | `csukuangfj/sherpa-onnx-streaming-zipformer-fr-kroko-2025-08-06` (aussi en, de, es) | en ligne | oui |
| Zipformer icefall | `shaojieli/sherpa-onnx-streaming-zipformer-fr-2023-04-14` | en ligne | oui |
| Nemotron 3.5 | `csukuangfj2/sherpa-onnx-nemotron-3.5-asr-streaming-0.6b-{80,160,320,560,1120}ms-int8-2026-06-11` (rév. 560 ms : `ab43d895f5985b1bbab8b6eac8607fcdc05343f3` ; 1120 ms : `cba1c96ca5ef0e8393b50584ae153a79145dc492`) | en ligne | oui (40 locales, détection automatique ou forcée) |
| Parakeet TDT v3 | `csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8` (`2bda32ec70b097a55adaa07d9a7173915b43cc78`) | hors ligne | oui (25 langues, détection automatique, sans forçage) |
| Canary 180M Flash | `csukuangfj/sherpa-onnx-nemo-canary-180m-flash-en-es-de-fr-int8` (`9077164e0d3dd1d5353743e89ceaa1d3a770838c`) | hors ligne | oui (en, de, es, fr) |
| Cohere Transcribe | `csukuangfj2/sherpa-onnx-cohere-transcribe-14-lang-int8-2026-04-01` (`156a470cf08eefe706a0004f3c52d9ee567ca7a0`) | hors ligne | oui (14 langues, langue obligatoire) |
| Qwen3-ASR 0.6B | `csukuangfj2/sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25` (`68818b2313fe77bd06f6a7c5068ff3ef59d02b8a`) | hors ligne | oui (30 langues) |
| Whisper | `sherpa-onnx-whisper-{tiny…large-v3,turbo}` ; turbo : `csukuangfj/sherpa-onnx-whisper-turbo` (≈ 1 Go en int8) | hors ligne | oui |
| Omnilingual CTC | `csukuangfj/sherpa-onnx-omnilingual-asr-1600-languages-300M-ctc-int8-2025-11-12` | hors ligne | oui |
| FastConformer NeMo | `…-fast-conformer-{ctc,transducer}-en-de-es-fr-14288`, `…-be-de-en-es-fr-hr-it-pl-ru-uk-20k` | hors ligne | oui |

**Sans français** : SenseVoice (zh, en, ja, ko, yue) [sensevoice], Moonshine (streaming en ar, de, en, es, ja, zh, tl, vi ; pas de fr) [moonshine], Dolphin (40 langues orientales et 22 dialectes chinois) [dolphin], FunASR-Nano et Fun-ASR-MLT-Nano (fr absent de la liste) [funasr], Paraformer, FireRedASR, X-ASR (zh/en), Nemotron speech EN, Parakeet unified EN, MedASR (en).

Le dépôt `Banafo/Kroko-ASR` publie aussi une variante FR à blocs de 0,64 s (`Kroko-FR-Community-64-L-Streaming-001.data`), au format propre à Kroko et lue par `kroko-ai/kroko-onnx`, un dérivé de sherpa-onnx [kroko]. Elle n'existe pas en conversion sherpa officielle.

## 3. Familles hors sherpa : points saillants

- **NVIDIA Nemotron 3.5** [nemotron] : FastConformer à cache, streaming, décodeur RNN-T conditionné par la langue, ponctuation et casse natives. Le runtime sherpa filtre les balises `<fr-FR>` émises dans le texte [pr3671]. Les WER FR sont plus élevés que ceux de Parakeet v3 (≈ 9 % contre ≈ 5 % sur FLEURS). C'est le prix du streaming. Avec des blocs de 560 ms, la quantification des tokens (problème D9) tombe sous le seuil de 1,2 s.
- **Parakeet / Canary** [parakeet][canary180][canary1b] : la référence qualité/vitesse sur CPU en mode par énoncé. Parakeet v3 détecte la langue lui-même : sur un énoncé court ou truffé d'anglicismes, il peut basculer de langue (risque non mesuré). Canary permet de forcer `src_lang = "fr"`.
- **whisper.cpp / faster-whisper** : excellent écosystème (`whisper-rs` compile whisper.cpp depuis les sources, ce qui convient au Flatpak). Mais l'encodeur travaille sur des fenêtres fixes de 30 s, c'est le modèle le plus lent ici, et il hallucine sur les silences. Sur un CPU modeste, turbo tourne à 1,4× le temps réel, soit environ 3,5 s de traitement pour 5 s de parole [tc-cat]. faster-whisper est en Python ; depuis Rust, on passe par `ct2rs` (MIT, CTranslate2).
- **Vosk** [vosk] : vrai streaming léger, mais modèles Kaldi de 2022 nettement moins précis, et libvosk à fournir (Kaldi est lourd à construire dans un Flatpak). Le modèle `vosk-model-fr-0.6-linto-2.2.0` est sous AGPL, `vosk-model-small-fr-pguyot-0.3` sous CC-BY-NC-SA : tous deux sont à exclure.
- **Kyutai STT** [dsm][kyutai-hf] : le seul modèle streaming bilingue FR/EN conçu pour ça, avec une implémentation Rust officielle (candle, crate `moshi`). Mais : 2,4 Go de poids, aucun WER FR publié, et un déploiement pensé pour GPU (serveur websocket, `--features cuda`). Pas de chiffre CPU natif.
- **Voxtral Realtime** [voxtral-rt] : excellent en FR à 480 ms (6,42 %), mais 4 milliards de paramètres. Mistral recommande un GPU ≥ 16 Go ; sur un CPU portable, il tourne à 0,58× le temps réel [tc-cat]. Le portage Rust (Burn) cible wgpu [voxtral-rs]. Hors critères CPU.
- **Cohere Transcribe** [cohere][oal] : meilleur WER FR ouvert à taille raisonnable (OAL 4,05 %), hors ligne, 2,9 Go en int8. Il exige la langue et un VAD en amont (il transcrit le bruit). Sherpa a corrigé ces hallucinations sur le silence en 1.13.8 [sherpa-cl].
- **Qwen3-ASR** [qwen3-report] : le 0.6B est moins bon que Parakeet v3 en FR (MLS 8,55 contre 4,97 %), le 1.7B est bon mais lent sur CPU. Ses hotwords peuvent servir pour le vocabulaire métier.
- **transcribe.cpp** [tc-repo] (MIT, ggml, juin 2026) : bibliothèque C/C++ pour 16 familles (Parakeet, Canary, Nemotron 3.5, Cohere, Voxtral Realtime, Whisper, Granite…). Crate officielle `transcribe-cpp` 0.2.4 (MIT), **compilée depuis les sources** avec CMake, sans binaire précompilé : plus simple pour Flathub que l'archive précompilée de sherpa-onnx (D4). Le projet se déclare encore « en développement ».

## 4. Applications Linux comparables

| Application | Langage / licence | Moteurs | Remarques |
|---|---|---|---|
| Speech Note `mkiol/dsnote` [dsnote] | C++/Qt, MPL-2.0, sur Flathub | Vosk, whisper.cpp (+ Parakeet), faster-whisper, April-ASR… | notes, lecture et traduction plus que dictée système |
| nerd-dictation `ideasman42/nerd-dictation` | Python, GPL-3.0 | Vosk | minimaliste, peu actif (dernier push 2025-10) |
| Handy `cjpais/Handy` [handy] | Rust (Tauri), MIT, 32,6 k étoiles | Whisper (small à large, turbo), Parakeet v3 | tape via `wtype`/`dotool`, et indique que `wtype` ne fonctionne pas sous la session Wayland d'Ubuntu 26.04 (GNOME) : il faut passer par `ydotool` |
| Voxtype `peteonrails/voxtype` [voxtype] | Rust, MIT | Whisper, Parakeet, Moonshine, SenseVoice, Paraformer, Dolphin, Omnilingual, Cohere, OpenVINO | appui maintenu pour parler ; `wtype` puis `dotool` puis `ydotool` |
| hyprwhspr `goodroot/hyprwhspr` | Python, MIT | Cohere, Parakeet v3, Whisper, Qwen3-ASR, REST | orienté Hyprland |
| BlahST `QuantiusBenignus/BlahST` | shell, BSD-3 | whisper.cpp | presse-papiers et frappe |

Constats : le seul streaming FR local qu'on y trouve passe par Vosk (nerd-dictation, Speech Note). Les applications récentes (Handy, Voxtype, hyprwhspr) fonctionnent toutes par énoncé, avec Parakeet v3, Cohere ou Whisper. Handy signale que `wtype` ne fonctionne pas sous la session Wayland d'Ubuntu 26.04 : la frappe par le portail RemoteDesktop, utilisée ici, est un vrai différenciateur.

## 5. Recommandation

Trois candidats, par ordre de priorité :

| | Nemotron 3.5 streaming (560 ms ou 1120 ms) | Parakeet TDT 0.6B v3 | Canary-180M-Flash |
|---|---|---|---|
| Mode | streaming (le même que Kroko) | par énoncé, texte tapé après chaque pause | par énoncé |
| WER FR | FLEURS ≈ 9 à 11 % | FLEURS ≈ 5,2 %, MLS 5,0 % | MLS 4,75 %, MCV 8,19 %, FLEURS 8,5 % |
| Taille | 682 Mo | 670 Mo | 207 Mo |
| CPU | ≈ 10× le temps réel | ≈ 14× (≈ 0,4 s pour 5 s de parole) | ≈ 22× |
| Licence | OpenMDW-1.1 | CC-BY-4.0 | CC-BY-4.0 |
| Effort | **faible** : même `OnlineRecognizer` et mêmes fichiers (encoder, decoder, joiner, tokens) ; ajouter `stream.set_option("language", "fr")` et passer à 4 threads | **moyen** : nouveau moteur `OfflineRecognizer` (`model_type = "nemo_transducer"`), découpage par le VAD énergétique (D9) avec une pause courte (≈ 0,6 à 1 s), plusieurs segments par dictée (revoir D11) | idem Parakeet, avec `src_lang = tgt_lang = "fr"` et `use_pnc = true` |
| Risque | dépôt sous le compte secondaire `csukuangfj2`, modèle récent ; gain sur Kroko à mesurer, pas à supposer | basculement de langue automatique sur les énoncés courts | 4 langues seulement |

1. **Nemotron 3.5**, premier choix : on garde le streaming et un code quasi identique, et on ajoute le multilingue. C'est le seul streaming FR moderne, sur CPU, déjà intégré à sherpa. Vérifier la chaîne de langue exacte (`"fr"` ou `"fr-FR"`, codes dans les métadonnées `prompt_dictionary` de l'encodeur [sherpa-nemotron]).
2. **Parakeet v3**, pour la meilleure précision par énoncé à coût CPU raisonnable. C'est l'option qui corrigerait le plus probablement les mots inventés du type « arporescence ». Si la précision prime sur la taille, **Cohere Transcribe** (2,9 Go, 4 à 11× le temps réel) est l'étape au-dessus.
3. **Canary-180M-Flash**, en variante légère du mode par énoncé, avec la langue forcée.

À écarter pour le CPU : Voxtral (Realtime et Mini), Kyutai (pas de chemin CPU documenté), Whisper large (trop lent), Vosk (précision).

**Ce qu'il faut pour le registre `data/models.toml`.** Pour chaque modèle : `repo`, `revision` (commit relevé par `https://huggingface.co/api/models/{repo}`, champ `sha`), et pour chaque fichier `path`, `size`, `sha256`. Le sha256 des fichiers LFS (`*.onnx`) figure dans `https://huggingface.co/api/models/{repo}/revision/{sha}?blobs=true` (`lfs.sha256`). Les petits fichiers non LFS (`tokens.txt`) sont à télécharger à cette révision puis passer à `sha256sum`. Côté code, il faut aussi :
- des valeurs d'`engine` nouvelles (par exemple `sherpa-onnx-offline-nemo-transducer`, `sherpa-onnx-offline-canary`). Le test `registry.rs` qui impose `sherpa-onnx-transducer` est à assouplir ;
- un champ optionnel de langue (option de flux pour Nemotron, `src_lang` pour Canary) ;
- pour Nemotron, ne lister que les fichiers int8 (`encoder.int8.onnx`, `decoder.int8.onnx`, `joiner.int8.onnx`, `tokens.txt`). Pour Parakeet : `encoder.int8.onnx`, `decoder.int8.onnx`, `joiner.int8.onnx`, `tokens.txt`, sans `test_wavs/` ;
- la licence exacte dans `license` (`OpenMDW-1.1`, `CC-BY-4.0`), à attribuer dans la fenêtre « À propos » ou les préférences.

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

- kroko : <https://huggingface.co/Banafo/Kroko-ASR>
- icefall-fr : <https://huggingface.co/shaojieli/icefall-asr-commonvoice-fr-pruned-transducer-stateless7-streaming-2023-04-02>
- nemotron : <https://huggingface.co/nvidia/nemotron-3.5-asr-streaming-0.6b>
- tc-nemotron : <https://huggingface.co/handy-computer/nemotron-3.5-asr-streaming-0.6b-gguf>
- pr3671 : <https://github.com/k2-fsa/sherpa-onnx/pull/3671>
- sherpa-nemotron : <https://github.com/k2-fsa/sherpa-onnx/blob/master/scripts/nemo/nemotron-3.5-asr-streaming-0.6b/README.md>
- openmdw : <https://openmdw.ai/license/1-1/>
- parakeet : <https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3>
- pk-fastapi : <https://github.com/groxaxo/parakeet-tdt-0.6b-v3-fastapi-openai>
- canary180 : <https://huggingface.co/nvidia/canary-180m-flash>
- canary1b : <https://huggingface.co/nvidia/canary-1b-v2>
- oal : <https://arxiv.org/html/2510.06961v4> (tableau 4)
- tc-repo : <https://github.com/handy-computer/transcribe.cpp>
- tc-bench : <https://github.com/handy-computer/transcribe.cpp/blob/main/catalog/_benchmark_profiles.json>
- tc-cat : <https://huggingface.co/handy-computer> (métadonnées `transcribe_cpp` de chaque dépôt `*-gguf` : `wer_fleurs_fr.q8_0`, `rtf_ryzen_4750u.cpu`)
- ggml : <https://huggingface.co/ggerganov/whisper.cpp>
- voxtral-paper : <https://arxiv.org/html/2507.13264> (tableaux 4 à 6)
- bofeng : <https://huggingface.co/bofenghuang/whisper-large-v3-french>
- vosk : <https://alphacephei.com/vosk/models>
- kyutai-hf : <https://huggingface.co/kyutai/stt-1b-en_fr>
- kyutai-candle : <https://huggingface.co/kyutai/stt-1b-en_fr-candle>
- kyutai-121 : <https://github.com/kyutai-labs/delayed-streams-modeling/issues/121>
- dsm : <https://github.com/kyutai-labs/delayed-streams-modeling>
- dsm-paper : <https://arxiv.org/html/2509.08753>
- voxtral-rt : <https://huggingface.co/mistralai/Voxtral-Mini-4B-Realtime-2602>
- voxtral-rs : <https://github.com/TrevorS/voxtral-mini-realtime-rs>
- voxtral-c : <https://github.com/antirez/voxtral.c>
- llamacpp-vx : <https://huggingface.co/ggml-org/Voxtral-Mini-3B-2507-GGUF>
- cohere : <https://huggingface.co/CohereLabs/cohere-transcribe-03-2026>
- qwen3-report : <https://arxiv.org/html/2601.21337v1> (tableau A.2)
- omni : <https://huggingface.co/csukuangfj/sherpa-onnx-omnilingual-asr-1600-languages-300M-ctc-int8-2025-11-12>
- ovos : <https://github.com/OpenVoiceOS/ovos-stt-plugin-sherpa-onnx>
- sherpa-cl : <https://github.com/k2-fsa/sherpa-onnx/blob/master/CHANGELOG.md>
- sherpa-rel : <https://github.com/k2-fsa/sherpa-onnx/releases/tag/asr-models>
- sensevoice : <https://huggingface.co/FunAudioLLM/SenseVoiceSmall>
- moonshine : <https://moonshine-voice.readthedocs.io/en/latest/models/available-models/>
- dolphin : <https://github.com/DataoceanAI/Dolphin>
- funasr : <https://huggingface.co/FunAudioLLM/Fun-ASR-MLT-Nano-2512>
- dsnote : <https://github.com/mkiol/dsnote>
- handy : <https://github.com/cjpais/Handy>
- voxtype : <https://github.com/peteonrails/voxtype>
- Crates : `sherpa-onnx` 1.13.8 (Apache-2.0), `whisper-rs` 0.16.0 (Unlicense), `ct2rs` 0.10.1 (MIT), `vosk` 0.3.1 (MIT), `moshi` 0.6.4 (MIT/Apache-2.0), `transcribe-cpp` 0.2.4 (MIT), `transcribe-rs` 0.3.11 (MIT), `parakeet-rs` 0.3.8 (MIT/Apache-2.0), d'après <https://crates.io> au 2026-10-02.
