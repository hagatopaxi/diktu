# Maintenance

## Updating sherpa-onnx and onnxruntime

The Flatpak builds sherpa-onnx from source (`sherpa-onnx` module of the manifest
`build-aux/fr.gwenael_leger.Diktu.json`) against the onnxruntime library published by
Microsoft (`onnxruntime` module), then links Diktu to `libsherpa-onnx-c-api.so` (see
[D24](DECISIONS.md#d24--sherpa-onnx-built-from-source-in-the-flatpak)). The native build
(`cargo build`), on the other hand, downloads the prebuilt static archive from k2-fsa.

**Mandatory coupling**: the version of the `sherpa-onnx` crate in `core/Cargo.toml` is
always equal to the sherpa-onnx tag built by the manifest. The FFI declarations of
`sherpa-onnx-sys` mirror the C API structures of the same version; a mismatch does not
cause a link error but structures misread at runtime.

The onnxruntime version follows the one sherpa-onnx expects, read from
`cmake/onnxruntime-linux-x86_64.cmake` at the chosen tag (1.28.2 for sherpa-onnx v1.13.8).

### Procedure

With `N` the new sherpa-onnx version (without the `v`):

1. **Crate**: in `core/Cargo.toml`, set `sherpa-onnx` to `N`, then run
   `cargo update -p sherpa-onnx --precise N` (this also updates `sherpa-onnx-sys`).
2. **Offline cargo sources**: regenerate `build-aux/cargo-sources.json` with
   [flatpak-cargo-generator](https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo),
   from a clone of flatpak-builder-tools:

   ```sh
   uv run flatpak-cargo-generator.py Cargo.lock -o build-aux/cargo-sources.json
   ```

3. **Tag commit**: resolve it rather than guess it.

   ```sh
   git ls-remote https://github.com/k2-fsa/sherpa-onnx refs/tags/vN 'refs/tags/vN^{}'
   ```

   For an annotated tag, take the `^{}` line (the commit); otherwise the only line. Copy
   `tag` and `commit` into the `git` source of the `sherpa-onnx` module.
4. **sherpa-onnx CMake dependencies**: the Flatpak build has no network; each
   `FetchContent` of sherpa-onnx finds its archive in the sources directory, where the
   `file` sources of the `sherpa-onnx` module put it. At the new tag, check them again in a clone:

   ```sh
   git clone --depth 1 --branch vN https://github.com/k2-fsa/sherpa-onnx
   cd sherpa-onnx
   grep -n '_URL \|_URL  \|_HASH\|CMAKE_SOURCE_DIR' cmake/kaldi-native-fbank.cmake \
       cmake/kaldi-decoder.cmake cmake/simple-sentencepiece.cmake cmake/json.cmake \
       cmake/eigen.cmake cmake/openfst.cmake
   ```

   Two archives are requested by the dependencies themselves: kissfft
   (`cmake/kissfft.cmake` in kaldi-native-fbank) and kaldifst (`cmake/kaldifst.cmake` in
   kaldi-decoder); read them from those archives. For each archive: `url` = the URL from the
   `.cmake` file, `dest-filename` = the name expected in `possible_file_locations`,
   `sha256` = the hash computed on the downloaded file (`curl -L <url> | sha256sum`),
   which must equal the `_HASH` in the `.cmake` file. If the build fails while attempting a
   download (`Downloading … from https://…`), add the missing archive the same way. The
   module's `-DSHERPA_ONNX_ENABLE_…` options disable what Diktu does not use (TTS,
   diarization, websocket, Python, executables) and therefore their dependencies.
5. **onnxruntime**: read the version from `cmake/onnxruntime-linux-x86_64.cmake`, and update
   both sources of the `onnxruntime` module (x86_64 and aarch64):
   `https://github.com/microsoft/onnxruntime/releases/download/vX/onnxruntime-linux-{x64,aarch64}-X.tgz`,
   with the `sha256` of each downloaded file.
6. **Build the Flatpak** and check the libraries:

   ```sh
   flatpak run org.flatpak.Builder --user --force-clean build-dir \
       build-aux/fr.gwenael_leger.Diktu.json
   flatpak build build-dir ldd /app/bin/diktu | grep "not found"   # must print nothing
   flatpak build build-dir /app/bin/diktu --help
   ```

7. **Test real dictation** (native build, same C API version):
   `cargo test -- --ignored`, then `cargo fmt --all --check`,
   `cargo clippy --all-targets -- -D warnings` and `cargo test`.
8. Record any behavior change in [DECISIONS.md](DECISIONS.md), then make a commit
   `build(deps): bump sherpa-onnx to N`.

### Automatic detection on Flathub

The Flathub bot runs
[flatpak-external-data-checker](https://github.com/flathub-infra/flatpak-external-data-checker)
on the application's Flathub repository and reads the `x-checker-data` blocks of the manifest:

- sherpa-onnx `git` source (`"type": "git"`, `tag-pattern` `^v([\d.]+)$`): for each
  new tag, it opens a pull request that updates `tag` and `commit`;
- onnxruntime sources (`"type": "json"` on the GitHub API for Microsoft's latest
  release): for each release, it updates `url` and `sha256` for both architectures.

The bot touches neither `core/Cargo.toml`, nor `cargo-sources.json`, nor the archives of the
CMake dependencies: its pull request serves as an alert and is completed with the procedure
above (steps 1, 2, 4 and 7). An onnxruntime release newer than the one expected by
sherpa-onnx is acceptable if the build and `cargo test -- --ignored` pass (the onnxruntime C
API stays backward compatible); otherwise close the pull request and wait for the matching
sherpa-onnx tag.

Check locally what the bot would see:

```sh
flatpak install --user flathub org.flathub.flatpak-external-data-checker
flatpak run org.flathub.flatpak-external-data-checker build-aux/fr.gwenael_leger.Diktu.json
```
