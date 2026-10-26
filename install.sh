#!/bin/sh
# Installs the `slml` compiler from the latest GitHub release.
#
#   curl -fsSL https://raw.githubusercontent.com/KubeKoslaw/slml/main/install.sh | sh
#
# Set SLML_INSTALL_DIR to choose the folder (default: ~/.local/bin).
# While the repository is private, install and log in to the GitHub CLI
# (`gh auth login`) first; the script then downloads through it.
set -eu

REPO="KubeKoslaw/slml"
DEST="${SLML_INSTALL_DIR:-$HOME/.local/bin}"

os=$(uname -s)
arch=$(uname -m)
case "$os-$arch" in
  Linux-x86_64) target=x86_64-unknown-linux-gnu ;;
  Darwin-arm64) target=aarch64-apple-darwin ;;
  Darwin-x86_64) target=x86_64-apple-darwin ;;
  *)
    echo "slml: no prebuilt compiler for $os $arch." >&2
    echo "Build it from source instead: git clone https://github.com/$REPO && cd slml && make install" >&2
    exit 1
    ;;
esac

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
  gh release download --repo "$REPO" --pattern "slml-*-$target.tar.gz" --dir "$tmp"
else
  url=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | grep -o "https://[^\"]*-$target\.tar\.gz" | head -n 1)
  if [ -z "$url" ]; then
    echo "slml: could not find a release for $target (is the repository private? use 'gh auth login')" >&2
    exit 1
  fi
  curl -fsSL "$url" -o "$tmp/slml.tar.gz"
fi

for archive in "$tmp"/*.tar.gz; do tar xzf "$archive" -C "$tmp"; done
mkdir -p "$DEST"
install -m 755 "$tmp"/slml-*/slml "$DEST/slml"
echo "installed $("$DEST/slml" --version) to $DEST/slml"
case ":$PATH:" in
  *":$DEST:"*) ;;
  *) echo "note: $DEST is not on your PATH; add it in your shell profile" ;;
esac
