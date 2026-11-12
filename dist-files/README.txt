iniaku-miner — one miner for epic.iniaku.my.id (Epic Cash)
=========================================================

Mines RandomX on the CPU and ProgPoW on the GPU at the same time, from one config file:
  cpu/  XMRig iniaku      (RandomX rx/epic)        worker <worker>-cpu
  gpu/  epic-miner iniaku (ProgPoW, OpenCL/CUDA)   worker <worker>-gpu
Both show up as separate workers of your wallet on https://epic.iniaku.my.id and are paid
from the RandomX and the ProgPoW blocks of the pool. Pool fee as on the website; no extra fee
on epic.iniaku.my.id (2% only on time spent on fallback pools).

Quick start
-----------
1. Open iniaku-miner.json and set "wallet" to your Epic address and "worker" to a name for
   this computer (letters, digits, - and _).
2. Linux: ./start.sh      Windows: start.cmd
3. Output of both engines is shown with [CPU] / [GPU] in front; every minute a summary line
   "[iniaku] summary: CPU RandomX ... | GPU ProgPoW ..." is printed and written to stats.json.
   Ctrl+C stops both.

iniaku-miner.json
-----------------
  "wallet"   your Epic address (es...)
  "worker"   rig name; the pool shows <worker>-cpu, <worker>-gpu and <worker>-cuckoo
  "region"   auto (lowest ping), id, us, au or eu
  "cpu"      "enabled": true/false, "threads": 0 = automatic or a number,
             "huge-pages": true/false,
             "fallback": a pool for the CPU between Epic RandomX turns, e.g.
               {"url": "xmr-eu1.nanopool.org:14433", "user": "YOUR_XMR_ADDRESS", "algo": "rx/0", "tls": true}
             "extra-args": more XMRig options, e.g. ["--cpu-priority=1"]
  "gpu"      "enabled": true/false (false on computers without a GPU),
             "devices": "all" (default) = every GPU, or GPU numbers counted from 0,
               e.g. [0, 2]; 4 GPUs = [0, 1, 2, 3] (also "0,1,2,3" or "0-3");
               "iniaku-miner --list-gpus" shows the GPUs it finds,
             "driver": "opencl" (ProgPoW on AMD and NVIDIA),
             "opencl-platform": "auto" (default) picks the NVIDIA/AMD OpenCL platform, so an
               integrated Intel GPU is skipped; or an index (0 = first) or "NVIDIA"/"AMD"/"Intel",
             "cuckoo": true/false, default true: also mine Cuckoo (Cuckatoo19, OpenCL) on the
               same GPUs during the Cuckoo turns, shown as <worker>-cuckoo,
             "fallback": a pool for the GPU between Epic ProgPoW turns, mined by XMRig, e.g.
               {"url": "gulf.moneroocean.stream:20128", "user": "YOUR_XMR_ADDRESS", "algo": "kawpow", "tls": true}
             "fallback-backend": "auto" (CUDA on NVIDIA when cpu/xmrig-cuda is present), "cuda", "opencl"
             Quantus (QTC) instead of KawPow: "algo": "quantus", e.g.
               {"url": "sg.lproute.com:5660", "user": "YOUR_QTC_ADDRESS.rig1", "algo": "quantus"}
               (LuckyPool, Singapore; runs gpu/quantus-iniaku, no DAG, little GPU memory)

Fallback pools and fee
----------------------
Epic takes turns per block: RandomX, ProgPoW, RandomX, ProgPoW, ... and Cuckoo every 25th.
Without fallback pools the GPU waits outside the ProgPoW turn and the CPU keeps mining
RandomX. With fallback pools both switch to them outside their turn and come back the moment
the turn starts. Time on epic.iniaku.my.id: 0% fee. Time on fallback pools: 2% fee (1 minute
after 49 minutes, same algorithm, never during an Epic turn).
The GPU keeps both DAGs in memory (ProgPoW ~2.2 GB + KawPow ~3-5 GB): use 8 GB+ GPUs for a
KawPow fallback.

Notes
-----
- Epic takes turns per block (RandomX, ProgPoW, Cuckoo every 25th). The GPU mines ProgPoW in
  its turn and waits otherwise ("waiting" lines from [GPU] are normal).
- The GPU needs ~3 GB of memory for the ProgPoW DAG.
- If one engine keeps crashing (for example no GPU driver) iniaku-miner stops restarting it
  after 3 quick exits and keeps the other one running.
- Windows Defender flags miners: add exclusions for this folder and for iniaku-miner.exe,
  cpu\xmrig.exe and gpu\epic-miner.exe (see cpu\README.txt).

Source: https://github.com/IniAku  —  License: GPL-3.0

Quantus on AMD (HiveOS/Linux): uses our OpenCL engine (same driver as ProgPoW).
If OpenCL fails it falls back to the Vulkan engine, which needs once:
  apt update && apt install -y libvulkan1 mesa-vulkan-drivers
