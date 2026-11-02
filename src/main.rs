// iniaku-miner — one miner for epic.iniaku.my.id
//
// Runs XMRig iniaku (RandomX rx/epic on the CPU) and epic-miner iniaku (ProgPoW on the GPU)
// from one config file, shows their output with [CPU]/[GPU] prefixes, restarts them when they
// crash and prints one combined summary every minute (also written to stats.json).
//
// This program is free software: you can redistribute it and/or modify it under the terms of
// the GNU General Public License as published by the Free Software Foundation, either version 3
// of the License, or (at your option) any later version.

use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const CPU_API_PORT: u16 = 60061;
const SUMMARY_EVERY: Duration = Duration::from_secs(60);
const RESTART_DELAY: Duration = Duration::from_secs(10);
const QUICK_EXIT: Duration = Duration::from_secs(20);
const MAX_QUICK_EXITS: u32 = 3;

#[cfg(windows)]
const EXE: &str = ".exe";
#[cfg(not(windows))]
const EXE: &str = "";

// ---------------------------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct Config {
	wallet: String,
	#[serde(default = "default_worker")]
	worker: String,
	#[serde(default = "default_region")]
	region: String,
	#[serde(default = "default_pass")]
	pass: String,
	#[serde(default)]
	cpu: CpuConfig,
	#[serde(default)]
	gpu: GpuConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct CpuConfig {
	#[serde(default = "yes")]
	enabled: bool,
	/// 0 = automatic
	#[serde(default)]
	threads: u32,
	#[serde(default = "yes")]
	huge_pages: bool,
	/// Pool for the ProgPoW/Cuckoo turns (see XMRig iniaku README), e.g.
	/// {"url": "xmr-eu1.nanopool.org:14433", "user": "<XMR address>", "algo": "rx/0", "tls": true}
	#[serde(default)]
	fallback: Option<Value>,
	#[serde(default)]
	extra_args: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct GpuConfig {
	#[serde(default = "yes")]
	enabled: bool,
	/// GPU indexes, e.g. [0, 1, 2]
	#[serde(default = "default_devices")]
	devices: Vec<u32>,
	/// "opencl" (AMD, NVIDIA, Intel) or "cuda" (NVIDIA, Linux CUDA package only)
	#[serde(default = "default_driver")]
	driver: String,
}

impl Default for CpuConfig {
	fn default() -> Self {
		CpuConfig { enabled: true, threads: 0, huge_pages: true, fallback: None, extra_args: vec![] }
	}
}

impl Default for GpuConfig {
	fn default() -> Self {
		GpuConfig { enabled: true, devices: default_devices(), driver: default_driver() }
	}
}

fn yes() -> bool {
	true
}
fn default_worker() -> String {
	"rig1".into()
}
fn default_region() -> String {
	"auto".into()
}
fn default_pass() -> String {
	"x".into()
}
fn default_devices() -> Vec<u32> {
	vec![0]
}
fn default_driver() -> String {
	"opencl".into()
}

fn load_config(path: &Path) -> Result<Config, String> {
	let text = fs::read_to_string(path).map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
	let config: Config = serde_json::from_str(&text).map_err(|e| format!("{}: {}", path.display(), e))?;

	if config.wallet.trim().is_empty() || config.wallet.starts_with("YOUR_") {
		return Err(format!("set your Epic address in {}: \"wallet\": \"es...\"", path.display()));
	}
	if config.wallet.contains(' ') || config.worker.contains(' ') || config.worker.contains('.') {
		return Err("wallet and worker must not contain spaces, worker must not contain '.'".into());
	}
	if !["auto", "id", "us", "au"].contains(&config.region.as_str()) {
		return Err(format!("unknown region \"{}\" (valid: auto, id, us, au)", config.region));
	}
	if !["opencl", "cuda"].contains(&config.gpu.driver.as_str()) {
		return Err(format!("unknown gpu driver \"{}\" (valid: opencl, cuda)", config.gpu.driver));
	}
	if !config.cpu.enabled && !config.gpu.enabled {
		return Err("both cpu and gpu are disabled".into());
	}

	Ok(config)
}

// ---------------------------------------------------------------------------------------------
// Configs of the two engines
// ---------------------------------------------------------------------------------------------

fn write_cpu_config(config: &Config, dir: &Path) -> Result<PathBuf, String> {
	let mut pools = vec![json!({
		"user": format!("{}.{}-cpu", config.wallet, config.worker),
		"pass": config.pass,
	})];
	if let Some(fallback) = &config.cpu.fallback {
		if !fallback.is_object() || fallback.get("url").is_none() || fallback.get("user").is_none() {
			return Err("cpu.fallback needs at least \"url\" and \"user\"".into());
		}
		pools.push(fallback.clone());
	}

	let xmrig = json!({
		"autosave": false,
		"donate-level": 0,
		"region": config.region,
		"colors": false,
		"print-time": 60,
		"cpu": { "enabled": true, "huge-pages": config.cpu.huge_pages, "max-threads-hint": 100 },
		"randomx": { "init": -1, "mode": "auto", "1gb-pages": false },
		"http": { "enabled": true, "host": "127.0.0.1", "port": CPU_API_PORT, "access-token": null, "restricted": true },
		"pools": pools,
	});

	let path = dir.join("config.json");
	fs::write(&path, serde_json::to_string_pretty(&xmrig).unwrap()).map_err(|e| format!("{}: {}", path.display(), e))?;
	Ok(path)
}

fn write_gpu_config(config: &Config, dir: &Path) -> Result<PathBuf, String> {
	let driver = if config.gpu.driver == "cuda" { 1 } else { 2 };
	let mut gpus = String::new();
	for device in &config.gpu.devices {
		gpus += &format!("\n[[mining.gpu_config]]\ndevice = {}\ndriver = {}\n", device, driver);
	}

	let toml = format!(
		r#"# Generated by iniaku-miner from iniaku-miner.json; edits here are overwritten.
[logging]
log_to_stdout = true
stdout_log_level = "Info"
log_to_file = true
file_log_level = "Info"
log_file_path = "epic-miner.log"
log_file_append = false

[mining]
algorithm = "ProgPow"
run_tui = false
stratum_server_addr = "{region}.epic.iniaku.my.id:3334"
stratum_server_tls_enabled = true
stratum_server_login = "{wallet}.{worker}-gpu"
stratum_server_password = "{pass}"
miner_plugin_dir = "plugins"

[mining.randomx_config]
threads = 1
jit = true
large_pages = false
hard_aes = true
{gpus}
# Required by the config format, not used for ProgPoW.
[[mining.miner_plugin_config]]
plugin_name = "cuckatoo_lean_cpu_compat_19"
[mining.miner_plugin_config.parameters]
nthreads = 1
"#,
		region = if config.region == "auto" { "id" } else { &config.region },
		wallet = config.wallet,
		worker = config.worker,
		pass = config.pass.replace('"', ""),
		gpus = gpus,
	);

	let path = dir.join("epic-miner.toml");
	fs::write(&path, toml).map_err(|e| format!("{}: {}", path.display(), e))?;
	Ok(path)
}

// ---------------------------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------------------------

#[derive(Default, Clone)]
struct GpuStats {
	hashrate: f64,
	accepted: u64,
	rejected: u64,
	waiting: bool,
}

#[derive(Default, Clone)]
struct CpuStats {
	hashrate: f64,
	accepted: u64,
	rejected: u64,
	pool: String,
}

fn parse_gpu_line(line: &str, stats: &Mutex<GpuStats>) {
	let mut s = stats.lock().unwrap();
	if let Some(i) = line.find("Mining: ProgPow at ") {
		let rest = &line[i + "Mining: ProgPow at ".len()..];
		if let Some(n) = rest.split_whitespace().next().and_then(|n| n.parse::<f64>().ok()) {
			s.hashrate = n;
			s.waiting = false;
		}
	} else if line.contains("Share Accepted") {
		s.accepted += 1;
	} else if line.contains("Failed to submit a solution") {
		s.rejected += 1;
	}
}

fn query_cpu(stats: &Mutex<CpuStats>) {
	let body = (|| -> Option<Value> {
		let mut stream = TcpStream::connect_timeout(&([127, 0, 0, 1], CPU_API_PORT).into(), Duration::from_secs(2)).ok()?;
		stream.set_read_timeout(Some(Duration::from_secs(3))).ok()?;
		stream.write_all(b"GET /2/summary HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n").ok()?;
		let mut response = String::new();
		stream.read_to_string(&mut response).ok()?;
		let start = response.find("\r\n\r\n")? + 4;
		serde_json::from_str(&response[start..]).ok()
	})();

	let mut s = stats.lock().unwrap();
	match body {
		Some(v) => {
			let total = &v["hashrate"]["total"];
			s.hashrate = total[0].as_f64().or_else(|| total[1].as_f64()).unwrap_or(0.0);
			s.accepted = v["results"]["shares_good"].as_u64().unwrap_or(0);
			s.rejected = v["results"]["shares_total"].as_u64().unwrap_or(0).saturating_sub(s.accepted);
			s.pool = v["connection"]["pool"].as_str().unwrap_or("").to_string();
		}
		None => s.hashrate = 0.0,
	}
}

fn format_rate(hs: f64) -> String {
	match hs {
		x if x >= 1e9 => format!("{:.2} GH/s", x / 1e9),
		x if x >= 1e6 => format!("{:.2} MH/s", x / 1e6),
		x if x >= 1e3 => format!("{:.2} kH/s", x / 1e3),
		x => format!("{:.0} H/s", x),
	}
}

// ---------------------------------------------------------------------------------------------
// Engines (child processes)
// ---------------------------------------------------------------------------------------------

struct Engine {
	tag: &'static str,
	dir: PathBuf,
	program: PathBuf,
	args: Vec<String>,
	child: Option<Child>,
	started: Instant,
	quick_exits: u32,
	restart_at: Option<Instant>,
	gave_up: bool,
}

impl Engine {
	fn new(tag: &'static str, dir: PathBuf, program: PathBuf, args: Vec<String>) -> Engine {
		Engine { tag, dir, program, args, child: None, started: Instant::now(), quick_exits: 0, restart_at: None, gave_up: false }
	}

	fn start(&mut self, gpu: Option<Arc<Mutex<GpuStats>>>) {
		log(&format!("[{}] starting {}", self.tag, self.program.display()));

		let spawned = Command::new(&self.program)
			.args(&self.args)
			.current_dir(&self.dir)
			.stdin(Stdio::null())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn();

		match spawned {
			Ok(mut child) => {
				let tag = self.tag;
				for reader in [
					child.stdout.take().map(|s| Box::new(s) as Box<dyn Read + Send>),
					child.stderr.take().map(|s| Box::new(s) as Box<dyn Read + Send>),
				]
				.into_iter()
				.flatten()
				{
					let gpu = gpu.clone();
					thread::spawn(move || {
						for line in BufReader::new(reader).lines() {
							let Ok(line) = line else { break };
							let line = strip_ansi(&line);
							if line.trim().is_empty() {
								continue;
							}
							if let Some(stats) = &gpu {
								parse_gpu_line(&line, stats);
							}
							println!("[{}] {}", tag, line);
						}
					});
				}
				self.child = Some(child);
				self.started = Instant::now();
			}
			Err(e) => {
				log(&format!("[{}] cannot start {}: {}", self.tag, self.program.display(), e));
				self.exited();
			}
		}
	}

	fn exited(&mut self) {
		self.child = None;
		if self.started.elapsed() < QUICK_EXIT {
			self.quick_exits += 1;
		} else {
			self.quick_exits = 0;
		}

		if self.quick_exits >= MAX_QUICK_EXITS {
			self.gave_up = true;
			log(&format!(
				"[{}] stopped after {} quick exits in a row — check the messages above{}",
				self.tag,
				self.quick_exits,
				if self.tag == "GPU" { " (no GPU / driver? set \"gpu\": {\"enabled\": false} to mine CPU only)" } else { "" }
			));
		} else {
			self.restart_at = Some(Instant::now() + RESTART_DELAY);
			log(&format!("[{}] exited, restarting in {} s", self.tag, RESTART_DELAY.as_secs()));
		}
	}

	fn poll(&mut self, gpu: Option<Arc<Mutex<GpuStats>>>) {
		if self.gave_up {
			return;
		}
		if let Some(child) = &mut self.child {
			if let Ok(Some(status)) = child.try_wait() {
				log(&format!("[{}] exited with {}", self.tag, status));
				self.exited();
			}
		} else if let Some(at) = self.restart_at {
			if Instant::now() >= at {
				self.restart_at = None;
				self.start(gpu);
			}
		}
	}

	fn stop(&mut self) {
		if let Some(mut child) = self.child.take() {
			let _ = child.kill();
			let _ = child.wait();
		}
	}

	fn running(&self) -> bool {
		self.child.is_some()
	}
}

fn strip_ansi(line: &str) -> String {
	let mut out = String::with_capacity(line.len());
	let mut chars = line.chars().peekable();
	while let Some(c) = chars.next() {
		if c == '\u{1b}' && chars.peek() == Some(&'[') {
			chars.next();
			for c in chars.by_ref() {
				if c.is_ascii_alphabetic() {
					break;
				}
			}
		} else {
			out.push(c);
		}
	}
	out
}

fn log(message: &str) {
	println!("[iniaku] {}", message);
}

fn now() -> u64 {
	SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

// ---------------------------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------------------------

fn usage() -> ! {
	println!("iniaku-miner {} — RandomX (CPU) + ProgPoW (GPU) for epic.iniaku.my.id", VERSION);
	println!("usage: iniaku-miner [-c iniaku-miner.json]");
	std::process::exit(0);
}

fn main() {
	let base = std::env::current_exe()
		.ok()
		.and_then(|p| p.parent().map(Path::to_path_buf))
		.unwrap_or_else(|| PathBuf::from("."));

	let mut config_path = base.join("iniaku-miner.json");
	let mut args = std::env::args().skip(1);
	while let Some(arg) = args.next() {
		match arg.as_str() {
			"-c" | "--config" => config_path = PathBuf::from(args.next().unwrap_or_else(|| usage())),
			"-V" | "--version" => {
				println!("iniaku-miner {}", VERSION);
				return;
			}
			_ => usage(),
		}
	}

	log(&format!("iniaku-miner {} — RandomX on the CPU + ProgPoW on the GPU, pool epic.iniaku.my.id", VERSION));

	let config = match load_config(&config_path) {
		Ok(c) => c,
		Err(e) => {
			log(&format!("config error: {}", e));
			std::process::exit(1);
		}
	};

	let cpu_dir = base.join("cpu");
	let gpu_dir = base.join("gpu");
	let gpu_stats = Arc::new(Mutex::new(GpuStats { waiting: true, ..Default::default() }));
	let cpu_stats = Arc::new(Mutex::new(CpuStats::default()));

	let mut engines: Vec<Engine> = vec![];

	if config.cpu.enabled {
		let cfg = write_cpu_config(&config, &cpu_dir).unwrap_or_else(|e| {
			log(&format!("cpu: {}", e));
			std::process::exit(1)
		});
		let mut args = vec!["-c".to_string(), cfg.display().to_string()];
		if config.cpu.threads > 0 {
			args.push(format!("--threads={}", config.cpu.threads));
		}
		args.extend(config.cpu.extra_args.iter().cloned());
		engines.push(Engine::new("CPU", cpu_dir.clone(), cpu_dir.join(format!("xmrig{}", EXE)), args));
		log(&format!("CPU: RandomX as {}.{}-cpu{}", short(&config.wallet), config.worker,
			if config.cpu.fallback.is_some() { " (+ fallback pool between RandomX turns)" } else { "" }));
	}

	if config.gpu.enabled {
		let cfg = write_gpu_config(&config, &gpu_dir).unwrap_or_else(|e| {
			log(&format!("gpu: {}", e));
			std::process::exit(1)
		});
		let args = vec!["-c".to_string(), cfg.display().to_string(), "--region".to_string(), config.region.clone()];
		engines.push(Engine::new("GPU", gpu_dir.clone(), gpu_dir.join(format!("epic-miner{}", EXE)), args));
		log(&format!("GPU: ProgPoW as {}.{}-gpu, {} device(s) {:?}", short(&config.wallet), config.worker,
			config.gpu.driver, config.gpu.devices));
	}

	let stop = Arc::new(AtomicBool::new(false));
	{
		let stop = stop.clone();
		let _ = ctrlc::set_handler(move || stop.store(true, Ordering::SeqCst));
	}

	for engine in engines.iter_mut() {
		let gpu = if engine.tag == "GPU" { Some(gpu_stats.clone()) } else { None };
		engine.start(gpu);
	}

	let stats_path = base.join("stats.json");
	let mut last_summary = Instant::now();

	while !stop.load(Ordering::SeqCst) {
		thread::sleep(Duration::from_millis(500));

		for engine in engines.iter_mut() {
			let gpu = if engine.tag == "GPU" { Some(gpu_stats.clone()) } else { None };
			engine.poll(gpu);
		}

		if engines.iter().all(|e| e.gave_up) {
			log("nothing left running, exiting");
			std::process::exit(1);
		}

		if last_summary.elapsed() >= SUMMARY_EVERY {
			last_summary = Instant::now();
			if config.cpu.enabled {
				query_cpu(&cpu_stats);
			}
			let cpu = cpu_stats.lock().unwrap().clone();
			let gpu = gpu_stats.lock().unwrap().clone();
			let cpu_on = engines.iter().any(|e| e.tag == "CPU" && e.running());
			let gpu_on = engines.iter().any(|e| e.tag == "GPU" && e.running());

			let mut parts = vec![];
			if config.cpu.enabled {
				parts.push(format!("CPU RandomX {} ({}/{} shares){}", format_rate(cpu.hashrate), cpu.accepted,
					cpu.accepted + cpu.rejected, if cpu_on { "" } else { " [stopped]" }));
			}
			if config.gpu.enabled {
				parts.push(format!("GPU ProgPoW {} ({}/{} shares){}", format_rate(gpu.hashrate), gpu.accepted,
					gpu.accepted + gpu.rejected, if gpu_on { "" } else { " [stopped]" }));
			}
			log(&format!("summary: {}", parts.join(" | ")));

			let stats = json!({
				"version": VERSION,
				"time": now(),
				"cpu": { "enabled": config.cpu.enabled, "running": cpu_on, "hashrate": cpu.hashrate,
					"accepted": cpu.accepted, "rejected": cpu.rejected, "pool": cpu.pool },
				"gpu": { "enabled": config.gpu.enabled, "running": gpu_on, "hashrate": gpu.hashrate,
					"accepted": gpu.accepted, "rejected": gpu.rejected, "devices": config.gpu.devices },
			});
			let _ = fs::write(&stats_path, serde_json::to_string_pretty(&stats).unwrap());
		}
	}

	log("stopping");
	for engine in engines.iter_mut() {
		engine.stop();
	}
}

fn short(wallet: &str) -> String {
	if wallet.len() > 16 {
		format!("{}…{}", &wallet[..8], &wallet[wallet.len() - 6..])
	} else {
		wallet.to_string()
	}
}
