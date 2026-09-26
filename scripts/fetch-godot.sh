#!/usr/bin/env bash
# Download the pinned Godot editor (Linux x86_64) into .tools/ (gitignored).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VER="${GODOT_VERSION:-4.7.2-stable}"
mkdir -p "$ROOT/.tools" && cd "$ROOT/.tools"
curl -sSL -o godot.zip "https://github.com/godotengine/godot/releases/download/${VER}/Godot_v${VER}_linux.x86_64.zip"
python3 -c "import zipfile; zipfile.ZipFile('godot.zip').extractall('.')"
rm godot.zip
chmod +x "Godot_v${VER}_linux.x86_64"
ln -sf "Godot_v${VER}_linux.x86_64" godot
./godot --version
