#!/usr/bin/env bash
cd "$(dirname "$0")" || exit 1
. h-manifest.conf

# lan= from the flight sheet: pin the region hostname to the LAN IP (and drop an old pin).
read -r lan_ip lan_host < .lan_host 2>/dev/null
sed -i '/# iniaku-lan-'"$CUSTOM_NAME"'$/d' /etc/hosts 2>/dev/null
[[ -n $lan_ip ]] && echo "$lan_ip $lan_host # iniaku-lan-$CUSTOM_NAME" >> /etc/hosts

# Quantus on AMD/Intel runs on Vulkan; HiveOS AMD images ship OpenCL only.
if grep -q '"algo": *"\(quantus\|qpow\)"' "$CUSTOM_CONFIG_FILENAME" 2>/dev/null && [[ ! -e /proc/driver/nvidia/version ]] \
	&& ! ls /usr/share/vulkan/icd.d/*.json /etc/vulkan/icd.d/*.json >/dev/null 2>&1; then
	echo "[iniaku] Quantus needs a Vulkan driver on AMD/Intel, none found. Install it once with:"
	echo "[iniaku]   apt update && apt install -y libvulkan1 mesa-vulkan-drivers"
fi
mkdir -p "$(dirname "$CUSTOM_LOG_BASENAME")"
./iniaku-miner -c "$CUSTOM_CONFIG_FILENAME" 2>&1 | tee --append "$CUSTOM_LOG_BASENAME.log"
