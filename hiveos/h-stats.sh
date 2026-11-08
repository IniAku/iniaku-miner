#!/usr/bin/env bash
# Sourced by the HiveOS agent: $khs and $stats from iniaku-miner's stats.json (written every minute).
# HiveOS shows one algorithm per miner: the GPU (ProgPoW, or the fallback algo between turns) is
# reported per GPU (the total split evenly), the CPU RandomX hashrate is added to the total and
# shown as an extra "GPU".

dir=$(dirname "${BASH_SOURCE[0]}")
. "$dir/h-manifest.conf"
s="$dir/stats.json"

if [[ ! -f $s ]] || (( $(date +%s) - $(jq -r '.time // 0' "$s" 2>/dev/null || echo 0) > 180 )); then
	khs=0
	stats=null
	return 0 2>/dev/null || exit 0
fi

stats=$(jq -c '
	(.gpu.devices // [0]) as $dev
	| (if (.gpu_fallback.running // false) and (((.gpu.running // false) | not) or (.gpu.turn != "progpow"))
		then (.gpu_fallback.hashrate // 0) else (.gpu.hashrate // 0) end) as $gpu_rate
	| (if .gpu.enabled then ($gpu_rate / ($dev | length)) else 0 end) as $per
	| {
		hs: ([$dev[] | $per] + (if .cpu.enabled then [.cpu.hashrate // 0] else [] end)),
		hs_units: "hs",
		temp: [], fan: [],
		uptime: 0,
		ver: .version,
		ar: [((.cpu.accepted // 0) + (.gpu.accepted // 0) + (.gpu_cuckoo.accepted // 0) + (.gpu_fallback.accepted // 0)),
		     ((.cpu.rejected // 0) + (.gpu.rejected // 0) + (.gpu_cuckoo.rejected // 0) + (.gpu_fallback.rejected // 0))],
		algo: "progpow+randomx"
	}' "$s")
khs=$(jq -r '[.hs[]] | add / 1000' <<< "$stats")
