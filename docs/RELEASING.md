# Releasing Diktu

A release is a `vX.Y.Z` tag on `main`. Pushing the tag is the only manual step: GitHub Actions
builds the Flatpak bundles and publishes them.

## One command

```sh
tools/release.sh 0.2.0 "One sentence describing the release."
```

The script refuses to run outside `main`, with uncommitted changes, or if the tag exists. It then:

1. sets `version` in the workspace `Cargo.toml` and refreshes `Cargo.lock`;
2. adds a `<release>` entry, dated today, at the top of `<releases>` in the AppStream metainfo
   (GNOME Software and Flathub show these notes) and validates the file;
3. runs `cargo fmt --check`, `cargo clippy -D warnings` and `cargo test`;
4. commits `chore: release X.Y.Z`, creates the annotated tag `vX.Y.Z` and pushes both.

Versioning follows [Semantic Versioning](https://semver.org): bump the patch for fixes, the minor
for new features, the major for breaking changes (for example settings that no longer migrate).

## What the CI does with the tag

The `Flatpak` workflow (`.github/workflows/flatpak.yml`):

1. builds `diktu-x86_64.flatpak` and `diktu-aarch64.flatpak` on native runners, from source
   (sherpa-onnx included, see [MAINTENANCE.md](MAINTENANCE.md));
2. writes `SHA256SUMS`;
3. creates the GitHub release `vX.Y.Z` with these three files and notes generated from the commit
   messages since the previous tag.

Follow it in the repository's **Actions** tab; the release appears under **Releases** after about
20 minutes. If a build fails, fix it on `main`, delete the tag (`git push --delete origin vX.Y.Z`
and `git tag -d vX.Y.Z`) and release again.

## Check the release

```sh
flatpak install --user ./diktu-x86_64.flatpak
flatpak run fr.gwenael_leger.Diktu
```

Then go through [MANUAL_TESTS.md](MANUAL_TESTS.md) and record the result in its table.
