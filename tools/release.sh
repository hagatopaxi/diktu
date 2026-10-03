#!/bin/sh
# Bumps the version, records the release in the AppStream metainfo, tags and pushes.
# The Flatpak workflow then builds the bundles and publishes the GitHub release.
# Usage: tools/release.sh 0.2.0 "One sentence describing the release."
set -eu

version=$1
notes=$2
metainfo=data/fr.gwenael_leger.Diktu.metainfo.xml

echo "$version" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' || { echo "version must be X.Y.Z" >&2; exit 1; }
[ "$(git branch --show-current)" = main ] || { echo "release from main" >&2; exit 1; }
git diff --quiet HEAD || { echo "commit or stash your changes first" >&2; exit 1; }
git rev-parse -q --verify "refs/tags/v$version" >/dev/null && { echo "v$version already exists" >&2; exit 1; }

sed -i "0,/^version = \".*\"/s//version = \"$version\"/" Cargo.toml
cargo update --workspace --quiet

# Newest release first, as AppStream expects; notes are XML-escaped.
escaped=$(printf '%s' "$notes" | sed 's/&/\&amp;/g; s/</\&lt;/g; s/>/\&gt;/g')
NOTES=$escaped VERSION=$version DATE=$(date +%F) awk '{ print }
  /^  <releases>$/ {
    printf "    <release version=\"%s\" date=\"%s\">\n      <description>\n        <p>%s</p>\n      </description>\n    </release>\n",
      ENVIRON["VERSION"], ENVIRON["DATE"], ENVIRON["NOTES"]
  }' "$metainfo" > "$metainfo.new"
mv "$metainfo.new" "$metainfo"
appstreamcli validate --no-net "$metainfo"

cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked

git commit -qam "chore: release $version"
git tag -a "v$version" -m "Diktu $version"
git push origin main "v$version"
