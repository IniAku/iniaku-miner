#!/usr/bin/env bash
# Assemble an iniaku-miner package from the launcher binary and the engine releases:
#   package/package.sh <linux|windows> <version> <launcher binary> [out dir]
set -euo pipefail
os=${1:?usage: package.sh <linux|windows> <version> <launcher> [out]}
version=${2:?}
launcher=${3:?}
root=$(cd "$(dirname "$0")/.." && pwd)
out=$(mkdir -p "${4:-$root/dist}" && cd "${4:-$root/dist}" && pwd)
. "$root/engines.env"

name="iniaku-miner-$version-$os-x64"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
pkg="$work/$name"
mkdir -p "$pkg/cpu" "$pkg/gpu" "$work/dl"

fetch() { curl -fsSL --retry 3 -o "$work/dl/$2" "$1"; }
gh() { echo "https://github.com/$1/releases/download/v$2/$3"; }
unpack() { # archive -> dir (single top folder flattened)
	local a=$1 d=$2 t
	t=$(mktemp -d -p "$work")
	case "$a" in
	*.zip) unzip -q "$a" -d "$t" ;;
	*) tar -C "$t" -xzf "$a" ;;
	esac
	local n; n=$(find "$t" -mindepth 1 -maxdepth 1 | wc -l)
	if [ "$n" = 1 ] && [ -d "$(find "$t" -mindepth 1 -maxdepth 1)" ]; then t=$(find "$t" -mindepth 1 -maxdepth 1); fi
	cp -r "$t"/. "$d"/
}

if [ "$os" = windows ]; then
	fetch "$(gh "$XMRIG_REPO" "$XMRIG_VERSION" "xmrig-iniaku-$XMRIG_VERSION-windows-x64.zip")" xmrig.zip
	fetch "$(gh "$EPIC_MINER_REPO" "$EPIC_MINER_VERSION" "epic-miner-iniaku-$EPIC_MINER_VERSION-windows-x64-opencl.zip")" epic.zip
	fetch "$XMRIG_CUDA_WIN_URL" cuda.zip
	fetch "$(gh "$QUANTUS_REPO" "$QUANTUS_VERSION" "quantus-iniaku-$QUANTUS_VERSION-windows-x64.zip")" quantus.zip
	unpack "$work/dl/xmrig.zip" "$pkg/cpu"
	unpack "$work/dl/epic.zip" "$pkg/gpu"
	unpack "$work/dl/cuda.zip" "$pkg/cpu"
	mkdir -p "$work/q" && unpack "$work/dl/quantus.zip" "$work/q"
	cp "$work/q/quantus-iniaku.exe" "$pkg/gpu/"
	cp "$work/q/LICENSE" "$pkg/gpu/LICENSE-quantus-iniaku.txt"
	cp "$pkg"/cpu/nvrtc*.dll "$pkg/gpu/"
	install -m 755 "$launcher" "$pkg/iniaku-miner.exe"
	sed 's/$/\r/' "$root/dist-files/start.cmd" > "$pkg/start.cmd" 2>/dev/null || cp "$root/dist-files/start.cmd" "$pkg/"
else
	fetch "$(gh "$XMRIG_REPO" "$XMRIG_VERSION" "xmrig-iniaku-$XMRIG_VERSION-linux-x64.tar.gz")" xmrig.tgz
	fetch "$(gh "$EPIC_MINER_REPO" "$EPIC_MINER_VERSION" "epic-miner-iniaku-$EPIC_MINER_VERSION-linux-x64-opencl.tar.gz")" epic.tgz
	fetch "$(gh "$XMRIG_CUDA_LINUX_REPO" "$XMRIG_CUDA_LINUX_VERSION" "xmrig-cuda-$XMRIG_CUDA_LINUX_VERSION-linux-x64-cuda12.tar.gz")" cuda.tgz
	fetch "$(gh "$QUANTUS_REPO" "$QUANTUS_VERSION" "quantus-iniaku-$QUANTUS_VERSION-linux-x64.tar.gz")" quantus.tgz
	unpack "$work/dl/xmrig.tgz" "$pkg/cpu"
	unpack "$work/dl/epic.tgz" "$pkg/gpu"
	unpack "$work/dl/cuda.tgz" "$pkg/cpu"
	mkdir -p "$work/q" && unpack "$work/dl/quantus.tgz" "$work/q"
	install -m 755 "$work/q/quantus-iniaku" "$pkg/gpu/"
	cp "$work/q/LICENSE" "$pkg/gpu/LICENSE-quantus-iniaku.txt"
	cp -P "$pkg"/cpu/libnvrtc* "$pkg/gpu/"
	install -m 755 "$launcher" "$pkg/iniaku-miner"
	install -m 755 "$root/dist-files/start.sh" "$pkg/start.sh"
fi

# The engines' own start scripts/configs would confuse: iniaku-miner writes their configs.
rm -f "$pkg/cpu/start.sh" "$pkg/cpu/start.cmd" "$pkg/gpu/start.cmd" "$pkg/cpu/config.json" "$pkg/gpu/epic-miner.toml"
for f in README.txt LICENSE; do [ -f "$pkg/cpu/$f" ] && mv "$pkg/cpu/$f" "$pkg/cpu/${f%.txt}-xmrig.txt" || true; done
for f in README.txt LICENSE LICENSE.txt; do [ -f "$pkg/gpu/$f" ] && mv "$pkg/gpu/$f" "$pkg/gpu/${f%.txt}-epic-miner.txt" || true; done
[ -f "$pkg/cpu/README.md" ] && mv "$pkg/cpu/README.md" "$pkg/cpu/README-xmrig-cuda.md" || true
[ -f "$pkg/cpu/SHA256SUMS" ] && mv "$pkg/cpu/SHA256SUMS" "$pkg/cpu/SHA256SUMS-xmrig-cuda.txt" || true
rm -f "$pkg/cpu/SHA256SUMS.sig"
cp "$root/dist-files/iniaku-miner.json" "$pkg/"
cp "$root/dist-files/README.txt" "$pkg/"
cp "$root/LICENSE" "$pkg/LICENSE.txt"
printf 'iniaku-miner %s\nXMRig iniaku %s\nepic-miner iniaku %s\nquantus-iniaku %s\n' "$version" "$XMRIG_VERSION" "$EPIC_MINER_VERSION" "$QUANTUS_VERSION" > "$pkg/VERSIONS.txt"

if [ "$os" = windows ]; then
	for f in "$pkg/README.txt" "$pkg/iniaku-miner.json" "$pkg/VERSIONS.txt"; do sed -i 's/\r\?$/\r/' "$f"; done
	if command -v zip > /dev/null; then
		(cd "$work" && zip -qr "$out/$name.zip" "$name")
	else
		(cd "$work" && python3 -m zipfile -c "$out/$name.zip" "$name")
	fi
	(cd "$out" && sha256sum "$name.zip" > "$name.zip.sha256")
	echo "$out/$name.zip"
else
	tar -C "$work" --owner=0 --group=0 -czf "$out/$name.tar.gz" "$name"
	(cd "$out" && sha256sum "$name.tar.gz" > "$name.tar.gz.sha256")
	echo "$out/$name.tar.gz"
fi
