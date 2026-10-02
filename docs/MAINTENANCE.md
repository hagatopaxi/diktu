# Maintenance

## Mettre à jour sherpa-onnx et onnxruntime

Le Flatpak compile sherpa-onnx depuis les sources (module `sherpa-onnx` du manifeste
`build-aux/fr.gwenael_leger.Diktu.json`) contre la bibliothèque onnxruntime publiée par
Microsoft (module `onnxruntime`), puis lie Diktu à `libsherpa-onnx-c-api.so` (voir D24).
Le build natif (`cargo build`) télécharge lui l'archive statique précompilée de k2-fsa.

**Couplage obligatoire** : la version de la crate `sherpa-onnx` dans `core/Cargo.toml` est
toujours égale au tag sherpa-onnx compilé par le manifeste. Les déclarations FFI de
`sherpa-onnx-sys` reproduisent les structures de l'API C de la même version ; un écart
ne provoque pas d'erreur d'édition de liens mais des structures mal lues à l'exécution.

La version d'onnxruntime suit celle qu'attend sherpa-onnx, lue dans
`cmake/onnxruntime-linux-x86_64.cmake` au tag choisi (1.28.2 pour sherpa-onnx v1.13.8).

### Procédure

Avec `N` la nouvelle version de sherpa-onnx (sans le `v`) :

1. **Crate** : dans `core/Cargo.toml`, passer `sherpa-onnx` à `N`, puis
   `cargo update -p sherpa-onnx --precise N` (met aussi à jour `sherpa-onnx-sys`).
2. **Sources cargo hors ligne** : régénérer `build-aux/cargo-sources.json` avec
   [flatpak-cargo-generator](https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo),
   depuis un clone de flatpak-builder-tools :

   ```sh
   uv run flatpak-cargo-generator.py Cargo.lock -o build-aux/cargo-sources.json
   ```

3. **Commit du tag** : le résoudre sans le deviner.

   ```sh
   git ls-remote https://github.com/k2-fsa/sherpa-onnx refs/tags/vN 'refs/tags/vN^{}'
   ```

   Pour un tag annoté, prendre la ligne `^{}` (le commit) ; sinon l'unique ligne. Reporter
   `tag` et `commit` dans la source `git` du module `sherpa-onnx`.
4. **Dépendances CMake de sherpa-onnx** : le build Flatpak n'a pas de réseau ; chaque
   `FetchContent` de sherpa-onnx trouve son archive dans le dossier des sources, où la
   déposent les sources `file` du module `sherpa-onnx`. Au nouveau tag, relire dans un clone :

   ```sh
   git clone --depth 1 --branch vN https://github.com/k2-fsa/sherpa-onnx
   cd sherpa-onnx
   grep -n '_URL \|_URL  \|_HASH\|CMAKE_SOURCE_DIR' cmake/kaldi-native-fbank.cmake \
       cmake/kaldi-decoder.cmake cmake/simple-sentencepiece.cmake cmake/json.cmake \
       cmake/eigen.cmake cmake/openfst.cmake
   ```

   Deux archives sont demandées par les dépendances elles-mêmes : kissfft
   (`cmake/kissfft.cmake` de kaldi-native-fbank) et kaldifst (`cmake/kaldifst.cmake` de
   kaldi-decoder) ; les lire dans ces archives. Pour chaque archive : `url` = l'URL du
   fichier `.cmake`, `dest-filename` = le nom attendu dans `possible_file_locations`,
   `sha256` = le hash calculé sur le fichier téléchargé (`curl -L <url> | sha256sum`),
   qui doit être égal au `_HASH` du `.cmake`. Si le build échoue en tentant un
   téléchargement (`Downloading … from https://…`), ajouter l'archive manquante de la même
   façon. Les options `-DSHERPA_ONNX_ENABLE_…` du module désactivent ce que Diktu n'utilise
   pas (TTS, diarisation, websocket, Python, exécutables) et donc leurs dépendances.
5. **onnxruntime** : lire la version dans `cmake/onnxruntime-linux-x86_64.cmake`, mettre à
   jour les deux sources du module `onnxruntime` (x86_64 et aarch64) :
   `https://github.com/microsoft/onnxruntime/releases/download/vX/onnxruntime-linux-{x64,aarch64}-X.tgz`,
   avec le `sha256` de chaque fichier téléchargé.
6. **Construire le Flatpak** et vérifier les bibliothèques :

   ```sh
   flatpak run org.flatpak.Builder --user --force-clean build-dir \
       build-aux/fr.gwenael_leger.Diktu.json
   flatpak build build-dir ldd /app/bin/diktu | grep "not found"   # ne doit rien afficher
   flatpak build build-dir /app/bin/diktu --help
   ```

7. **Tester la dictée réelle** (build natif, même version de l'API C) :
   `cargo test -- --ignored`, puis `cargo fmt --all --check`,
   `cargo clippy --all-targets -- -D warnings` et `cargo test`.
8. Consigner tout changement de comportement dans `docs/DECISIONS.md`, puis un commit
   `build(deps): bump sherpa-onnx to N`.

### Détection automatique sur Flathub

Le bot de Flathub exécute
[flatpak-external-data-checker](https://github.com/flathub-infra/flatpak-external-data-checker)
sur le dépôt Flathub de l'application et lit les blocs `x-checker-data` du manifeste :

- source `git` de sherpa-onnx (`"type": "git"`, `tag-pattern` `^v([\d.]+)$`) : à chaque
  nouveau tag, il ouvre une pull request qui met à jour `tag` et `commit` ;
- sources d'onnxruntime (`"type": "json"` sur l'API GitHub de la dernière release de
  Microsoft) : à chaque release, il met à jour `url` et `sha256` des deux architectures.

Le bot ne touche ni à `core/Cargo.toml`, ni à `cargo-sources.json`, ni aux archives des
dépendances CMake : sa pull request sert d'alerte et se complète avec la procédure
ci-dessus (étapes 1, 2, 4 et 7). Une release d'onnxruntime plus récente que celle attendue
par sherpa-onnx est acceptable si le build et `cargo test -- --ignored` passent (l'API C
d'onnxruntime reste rétrocompatible) ; sinon fermer la pull request et attendre le tag
sherpa-onnx correspondant.

Vérifier localement ce que le bot verrait :

```sh
flatpak install --user flathub org.flathub.flatpak-external-data-checker
flatpak run org.flathub.flatpak-external-data-checker build-aux/fr.gwenael_leger.Diktu.json
```
