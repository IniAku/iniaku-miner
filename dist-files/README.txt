iniaku-miner — one miner for epic.iniaku.my.id (Epic Cash)
=========================================================

Mines RandomX on the CPU and ProgPoW on the GPU at the same time, from one config file:
  cpu/  XMRig iniaku      (RandomX rx/epic)        worker <worker>-cpu
  gpu/  epic-miner iniaku (ProgPoW, OpenCL/CUDA)   worker <worker>-gpu
Both show up as separate workers of your wallet on https://epic.iniaku.my.id and are paid
from the RandomX and the ProgPoW blocks of the pool. Pool fee as on the website; no extra fee
on epic.iniaku.my.id.

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
  "worker"   rig name; the pool shows <worker>-cpu and <worker>-gpu
  "region"   auto (lowest ping), id, us or au
  "cpu"      "enabled": true/false, "threads": 0 = automatic or a number,
             "huge-pages": true/false,
             "fallback": a pool for the time between Epic RandomX turns, e.g.
               {"url": "xmr-eu1.nanopool.org:14433", "user": "<XMR address>", "algo": "rx/0", "tls": true}
               (2% fee on the fallback time only, see cpu/README.txt)
             "extra-args": more XMRig options, e.g. ["--cpu-priority=1"]
  "gpu"      "enabled": true/false (false on computers without a GPU),
             "devices": GPU numbers, e.g. [0] or [0, 1, 2] for three GPUs,
             "driver": "opencl" (AMD, NVIDIA, Intel) or "cuda" (NVIDIA, Linux CUDA package)

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
