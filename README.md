# iniaku-miner

One program that mines Epic Cash on [epic.iniaku.my.id](https://epic.iniaku.my.id) with all three Epic algorithms:

| Hardware | Algorithm | Engine | Worker shown on the pool |
| --- | --- | --- | --- |
| CPU | RandomX | XMRig iniaku | `<worker>-cpu` |
| GPU | ProgPoW | epic-miner iniaku | `<worker>-gpu` |
| GPU | Cuckoo (Cuckatoo19) | epic-miner iniaku | `<worker>-cuckoo` |

Optionally, the CPU and GPU can mine another coin between their Epic turns (see [Dual mining](#dual-mining-with-fallback-pools)).

This is beta software. Please report problems in [Issues](https://github.com/IniAku/iniaku-miner/issues).

## Download

Get the latest build from [Releases](https://github.com/IniAku/iniaku-miner/releases) or the [Miners page](https://epic.iniaku.my.id/miners#iniaku-miner).

- Windows x64
- Linux x64 (Ubuntu 20.04 or newer)
- HiveOS custom miner package (see [HiveOS](#hiveos))

Windows Defender flags miners. Add exclusions for this folder and for `iniaku-miner.exe`, `cpu\xmrig.exe` and `gpu\epic-miner.exe`.

## Quick start

1. Open `iniaku-miner.json`. Set `wallet` to your Epic address and `worker` to a name for this computer (letters, digits, `-` and `_`).
2. Run `start.cmd` on Windows or `./start.sh` on Linux.
3. Press Ctrl+C to stop.

Engine output is prefixed with `[CPU]`, `[GPU]` and so on. Every minute a summary line is printed and written to `stats.json`.

Minimal config:

```json
{
    "wallet": "YOUR_EPIC_ADDRESS",
    "worker": "rig1",
    "region": "auto",
    "cpu": { "enabled": true, "threads": 0 },
    "gpu": { "enabled": true, "devices": "all", "driver": "opencl" }
}
```

`"devices": "all"` mines on every GPU (`iniaku-miner --list-gpus` shows what it finds). To use only some, list them: `"devices": [0, 2]`. GPUs are numbered from 0, so 4 GPUs are `[0, 1, 2, 3]`; `"0,1,2,3"`, `"0 1 2 3"` and `"0-3"` work too. A number the computer does not have is skipped with a warning. On a computer without a GPU, set `"enabled": false` in `gpu`.

Unknown keys in the config stop the miner at startup, so check the spelling if it refuses to start.

## Config reference

| Key | Meaning |
| --- | --- |
| `wallet` | Your Epic address (`es...`) |
| `worker` | Rig name |
| `region` | `auto` (lowest ping), `id`, `us`, `au` or `eu` |
| `cpu.enabled` | `true` or `false` |
| `cpu.threads` | `0` for automatic, or a number |
| `cpu.huge-pages` | `true` or `false` |
| `cpu.fallback` | Pool for the CPU between Epic RandomX turns |
| `cpu.extra-args` | More XMRig options, e.g. `["--cpu-priority=1"]` |
| `gpu.enabled` | `true` or `false` |
| `gpu.devices` | `"all"` (default) or GPU numbers counted from 0, e.g. `[0, 2]`, `"0,1,2,3"`, `"0-3"` |
| `gpu.driver` | `opencl` |
| `gpu.opencl-platform` | `auto` (default) picks the NVIDIA or AMD platform and skips an integrated Intel GPU. Or an index (`0` = first), or `NVIDIA`, `AMD`, `Intel` |
| `gpu.cuckoo` | `true` (default) also mines Cuckoo on the same GPUs during Cuckoo turns |
| `gpu.fallback` | Pool for the GPU between Epic ProgPoW turns |
| `gpu.fallback-backend` | Engine backend for the GPU fallback: `auto` (CUDA on NVIDIA when available), `cuda` or `opencl` |

## How the turns work

Epic takes turns per block: RandomX, ProgPoW, RandomX, ProgPoW, and Cuckoo every 25th block.

- The GPU mines ProgPoW during its turn and waits otherwise, so `[GPU]` waiting lines are normal.
- The first Cuckoo turn can take a while to arrive. Until then, `[CUCKOO] Waiting for Cuckoo turn` is normal.
- The CPU keeps mining RandomX when no fallback pool is set.

## Dual mining with fallback pools

A fallback pool puts the hardware to work outside its Epic turn. The miner switches to the fallback pool and comes back the moment the Epic turn starts.

### Quantus (QTC) on the GPU

Add `fallback` to the `gpu` section:

```json
"gpu": {
    "enabled": true,
    "devices": "all",
    "driver": "opencl",
    "fallback": {
        "url": "sg.lproute.com:5660",
        "user": "YOUR_QTC_ADDRESS.rig1",
        "algo": "quantus"
    }
}
```

Quantus has no DAG and needs little GPU memory. On AMD it uses OpenCL; if OpenCL fails it falls back to a Vulkan engine, which needs this once on HiveOS or Linux:

```sh
apt update && apt install -y libvulkan1 mesa-vulkan-drivers
```

### KawPow on the GPU

```json
"fallback": {
    "url": "gulf.moneroocean.stream:20128",
    "user": "YOUR_XMR_ADDRESS",
    "algo": "kawpow",
    "tls": true
}
```

The GPU keeps both DAGs in memory (ProgPoW about 2.2 GB, KawPow about 3 to 5 GB), so use GPUs with 8 GB or more.

### Monero on the CPU

Add `fallback` to the `cpu` section:

```json
"fallback": {
    "url": "xmr-eu1.nanopool.org:14433",
    "user": "YOUR_XMR_ADDRESS",
    "algo": "rx/0",
    "tls": true
}
```

## Fee

- Time on epic.iniaku.my.id: **0% dev fee**. The pool fee is listed on the website.
- Time on fallback pools: **2%** (1 minute after 49 minutes, same algorithm, never during an Epic turn).
- Without fallback pools there is no dev fee at all.

## HiveOS

Create a flight sheet with these values. The current Installation URL is on the [Miners page](https://epic.iniaku.my.id/miners#iniaku-miner).

| Field | Value |
| --- | --- |
| Coin | EPIC (add it as a custom coin if it is missing) |
| Wallet | Your Epic address |
| Pool | Configure in miner |
| Miner | Custom |
| Hash algorithm | `progpow` (dashboard label only) |
| Wallet and worker template | `%WAL%.%WORKER_NAME%` |
| Pool URL | `us.epic.iniaku.my.id` (or `id.`, `au.`, `eu.`); anything else picks the closest |
| Pass | `x` |

Optional arguments go in **Extra config arguments**, separated by spaces:

| Argument | Meaning |
| --- | --- |
| `cpu=0`, `gpu=0`, `cuckoo=0` | Turn off the CPU, GPU or Cuckoo part |
| `threads=6` | CPU threads (default automatic) |
| `devices=0,1` | GPUs (default: all NVIDIA and AMD GPUs HiveOS detected) |
| `platform=NVIDIA` | OpenCL platform, by index or vendor name (default automatic) |
| `gpu-fallback=host:port` | GPU fallback pool, requires `gpu-fallback-user=` |
| `gpu-fallback-user=`, `gpu-fallback-algo=`, `gpu-fallback-pass=` | GPU fallback settings (algo default `kawpow`) |
| `cpu-fallback=host:port` | CPU fallback pool, requires `cpu-fallback-user=` |
| `cpu-fallback-user=`, `cpu-fallback-algo=`, `cpu-fallback-pass=` | CPU fallback settings (algo default `rx/0`) |
| `fallback-tls=0` | No TLS to the fallback pools (default on) |
| `lan=192.168.1.139` | The pool node is on your own LAN; needs a regional Pool URL |

Quantus example:

```
gpu-fallback=sg.lproute.com:5660 gpu-fallback-user=YOUR_QTC_ADDRESS.rig1 gpu-fallback-algo=quantus
```

## Troubleshooting

| Symptom | What to check |
| --- | --- |
| The miner stops at startup with a config error | A key is misspelled or not supported by your version |
| Only the integrated GPU is listed | Use v1.0.8-beta or newer. The startup line `ProgPoW: OpenCL platform ...` shows the platform in use. Set `gpu.opencl-platform` to `NVIDIA` or `AMD` if it is wrong |
| A rig has both NVIDIA and AMD GPUs | ProgPoW uses one OpenCL platform, so only that vendor's GPUs mine it |
| An engine stops restarting | After 3 quick exits the miner stops restarting that engine and keeps the others running. Check the GPU driver |
| GPU memory errors | ProgPoW needs about 3 GB of GPU memory, and more with a KawPow fallback |

## Building from source

```sh
cargo build --release
```

This builds the launcher only. The engines are separate open-source projects, pinned in [`engines.env`](engines.env):

- [xmrig-iniaku](https://github.com/IniAku/xmrig-iniaku)
- [Epic-Miner-Iniaku](https://github.com/IniAku/Epic-Miner-Iniaku)
- [quantus-miner](https://github.com/IniAku/quantus-miner)
- [xmrig-cuda](https://github.com/IniAku/xmrig-cuda)

## License

[GPL-3.0](LICENSE)
