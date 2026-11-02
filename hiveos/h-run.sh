#!/usr/bin/env bash
cd "$(dirname "$0")" || exit 1
. h-manifest.conf
mkdir -p "$(dirname "$CUSTOM_LOG_BASENAME")"
./iniaku-miner -c "$CUSTOM_CONFIG_FILENAME" 2>&1 | tee --append "$CUSTOM_LOG_BASENAME.log"
