#!/usr/bin/env bash
# Builds iniaku-miner.json from the HiveOS flight sheet.
#   Wallet and worker template: %WAL%.%WORKER_NAME%
#   Pool URL:                   us.epic.iniaku.my.id (or id./au.) — anything else = auto
#   Extra config arguments (optional, space separated):
#     cpu=0 | gpu=0              disable the CPU (RandomX) or GPU (ProgPoW) part
#     threads=6                  CPU threads (default automatic)
#     devices=0,1                GPUs (default: all GPUs HiveOS detected)
#     cpu-fallback=host:port cpu-fallback-user=<wallet> [cpu-fallback-algo=rx/0] [cpu-fallback-pass=x]
#     gpu-fallback=host:port gpu-fallback-user=<wallet> [gpu-fallback-algo=kawpow] [gpu-fallback-pass=x]
#       pools used between the Epic RandomX / ProgPoW turns (2% fee on that time only)
#     fallback-tls=0             no TLS to the fallback pools (default TLS on)
#     lan=192.168.1.139          the pool node is in your own LAN (router without NAT loopback):
#                                the region's hostname is pointed at that IP in /etc/hosts.
#                                Needs a regional Pool URL (id./us./au./eu.epic.iniaku.my.id).

dir=$(dirname "${BASH_SOURCE[0]}")
. "$dir/h-manifest.conf"

[[ -z $CUSTOM_TEMPLATE ]] && echo "CUSTOM_TEMPLATE is empty (set it to %WAL%.%WORKER_NAME%)" && return 1
wallet=${CUSTOM_TEMPLATE%.*}
worker=${CUSTOM_TEMPLATE##*.}
[[ $wallet == "$CUSTOM_TEMPLATE" ]] && worker=$(hostname)

region=auto
case "$CUSTOM_URL" in
	*id.epic*) region=id ;;
	*us.epic*) region=us ;;
	*au.epic*) region=au ;;
	*eu.epic*) region=eu ;;
esac

cpu=true gpu=true cuckoo=true threads=0 devices= tls=true
cf= cfu= cfa=rx/0 cfp=x gf= gfu= gfa=kawpow gfp=x lan=
for a in $CUSTOM_USER_CONFIG; do
	case "$a" in
		cpu=0) cpu=false ;;
		gpu=0) gpu=false ;;
		cuckoo=0) cuckoo=false ;;
		threads=*) threads=${a#*=} ;;
		devices=*) devices=${a#*=} ;;
		cpu-fallback=*) cf=${a#*=} ;;
		cpu-fallback-user=*) cfu=${a#*=} ;;
		cpu-fallback-algo=*) cfa=${a#*=} ;;
		cpu-fallback-pass=*) cfp=${a#*=} ;;
		gpu-fallback=*) gf=${a#*=} ;;
		gpu-fallback-user=*) gfu=${a#*=} ;;
		gpu-fallback-algo=*) gfa=${a#*=} ;;
		gpu-fallback-pass=*) gfp=${a#*=} ;;
		fallback-tls=0) tls=false ;;
		lan=*) lan=${a#*=} ;;
		*) echo "unknown extra config argument: $a" ;;
	esac
done

if [[ -z $devices ]]; then
	n=$(jq '[.[] | select(.brand == "nvidia" or .brand == "amd")] | length' /run/hive/gpu-detect.json 2>/dev/null || echo 1)
	[[ -z $n || $n == 0 ]] && n=1
	devices=$(seq -s, 0 $((n - 1)))
fi
driver=opencl

if [[ -n $lan && $region == auto ]]; then
	echo "lan= needs a regional Pool URL (id./us./au./eu.epic.iniaku.my.id) — ignored"
	lan=
fi
echo "${lan:+$lan $region.epic.iniaku.my.id}" > "$dir/.lan_host"

[[ -n $cf && -z $cfu ]] && echo "cpu-fallback needs cpu-fallback-user — ignored" && cf=
[[ -n $gf && -z $gfu ]] && echo "gpu-fallback needs gpu-fallback-user — ignored" && gf=

jq -n --arg wallet "$wallet" --arg worker "$worker" --arg region "$region" --arg pass "${CUSTOM_PASS:-x}" \
	--argjson cpu "$cpu" --argjson gpu "$gpu" --argjson cuckoo "$cuckoo" --argjson threads "$threads" --arg devices "$devices" --arg driver "$driver" \
	--arg cf "$cf" --arg cfu "$cfu" --arg cfa "$cfa" --arg cfp "$cfp" \
	--arg gf "$gf" --arg gfu "$gfu" --arg gfa "$gfa" --arg gfp "$gfp" --argjson tls "$tls" '{
	wallet: $wallet, worker: $worker, region: $region, pass: $pass,
	cpu: ({enabled: $cpu, threads: $threads}
		+ (if $cf == "" then {} else {fallback: {url: $cf, user: $cfu, pass: $cfp, algo: $cfa, tls: $tls}} end)),
	gpu: ({enabled: $gpu, cuckoo: $cuckoo, devices: ($devices | split(",") | map(tonumber)), driver: $driver}
		+ (if $gf == "" then {} else {fallback: {url: $gf, user: $gfu, pass: $gfp, algo: $gfa, tls: $tls}} end))
}' > "$CUSTOM_CONFIG_FILENAME"
