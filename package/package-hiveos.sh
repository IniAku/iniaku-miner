#!/bin/sh
# HiveOS custom-miner package from the Linux package:
#   package/package-hiveos.sh <iniaku-miner linux .tar.gz> <version> [out dir]
# Result: <CUSTOM_NAME>-<hive version>.tar.gz with one <CUSTOM_NAME>/ folder (HiveOS layout).
set -eu
src=${1:?usage: package-hiveos.sh <linux .tar.gz> <version> [out dir]}
version=${2:?}
root=$(cd "$(dirname "$0")/.." && pwd)
out=$(cd "${3:-$root/dist}" && pwd)
name=$(sed -n 's/^CUSTOM_NAME=//p' "$root/hiveos/h-manifest.conf")
hive_version=$(echo "$version" | sed 's/-/./g')

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
tar -C "$tmp" -xzf "$src"
mv "$tmp"/*/ "$tmp/$name"
for f in h-config.sh h-run.sh h-stats.sh; do
	install -m 755 "$root/hiveos/$f" "$tmp/$name/$f"
done
sed "s/@VERSION@/$hive_version/" "$root/hiveos/h-manifest.conf" > "$tmp/$name/h-manifest.conf"
tar -C "$tmp" --owner=0 --group=0 -czf "$out/$name-$hive_version.tar.gz" "$name"
(cd "$out" && sha256sum "$name-$hive_version.tar.gz" > "$name-$hive_version.tar.gz.sha256")
echo "$out/$name-$hive_version.tar.gz"
