//! HTTP: metadata, files and the mirror fallback chain.
//!
//! Every download is written to `<name>.part` and renamed only after its SHA-1 matches, so an
//! interrupted run leaves nothing the next run would trust.

use sha1::{Digest, Sha1};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

#[derive(Clone)]
pub struct Net {
    agent: ureq::Agent,
}

#[derive(Clone, Debug)]
pub struct FileJob {
    /// Mirrors in order: the chosen source first, then the official host.
    pub urls: Vec<String>,
    pub dest: PathBuf,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

impl Net {
    pub fn new() -> Net {
        let agent = ureq::AgentBuilder::new()
            .user_agent(concat!(
                "LiteCraft-Launther/",
                env!("CARGO_PKG_VERSION"),
                " (+https://github.com/horizon-rust/horizon)"
            ))
            .timeout_connect(Duration::from_secs(25))
            .timeout_read(Duration::from_secs(180))
            .timeout_write(Duration::from_secs(180))
            .try_proxy_from_env(true)
            .build();
        Net { agent }
    }

    pub fn text(&self, url: &str) -> Result<String, String> {
        let response = self
            .agent
            .get(url)
            .call()
            .map_err(|e| format!("{url}: {e}"))?;
        response.into_string().map_err(|e| format!("{url}: {e}"))
    }

    /// Downloads one file if it is not already present with the right SHA-1.
    /// Returns whether bytes were fetched.
    pub fn file(
        &self,
        job: &FileJob,
        cancel: &AtomicBool,
        progress: &(dyn Fn(u64) + Sync),
    ) -> Result<bool, String> {
        if let Some(expected) = job.sha1.as_deref() {
            if job.dest.is_file() && sha1_file(&job.dest).ok().as_deref() == Some(expected) {
                return Ok(false);
            }
        } else if job.dest.is_file() && job.dest.metadata().map(|m| m.len()).unwrap_or(0) > 0 {
            return Ok(false);
        }

        if let Some(parent) = job.dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }

        let mut last = String::new();
        for url in &job.urls {
            if cancel.load(Ordering::Relaxed) {
                return Err("已取消".into());
            }
            match self.fetch_once(url, job, cancel, progress) {
                Ok(()) => return Ok(true),
                Err(e) => last = e,
            }
        }
        Err(if last.is_empty() {
            format!("{}: 没有可用的下载源", job.dest.display())
        } else {
            last
        })
    }

    fn fetch_once(
        &self,
        url: &str,
        job: &FileJob,
        cancel: &AtomicBool,
        progress: &(dyn Fn(u64) + Sync),
    ) -> Result<(), String> {
        let response = self
            .agent
            .get(url)
            .call()
            .map_err(|e| format!("{url}: {e}"))?;
        let mut part = job.dest.clone();
        let name = part
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "download".into());
        part.set_file_name(format!("{name}.part"));
        if part.exists() {
            let _ = std::fs::remove_file(&part);
        }
        let mut file =
            std::fs::File::create(&part).map_err(|e| format!("{}: {e}", part.display()))?;
        let mut reader = response.into_reader();
        let mut hasher = Sha1::new();
        let mut buffer = vec![0u8; 128 * 1024];
        let mut written = 0u64;
        loop {
            if cancel.load(Ordering::Relaxed) {
                let _ = std::fs::remove_file(&part);
                return Err("已取消".into());
            }
            let n = reader
                .read(&mut buffer)
                .map_err(|e| format!("{url}: {e}"))?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
            file.write_all(&buffer[..n])
                .map_err(|e| format!("{}: {e}", part.display()))?;
            written += n as u64;
            progress(n as u64);
        }
        file.flush().map_err(|e| format!("{e}"))?;
        drop(file);

        let digest = hex(&hasher.finalize());
        if let Some(expected) = job.sha1.as_deref() {
            if digest != expected {
                let _ = std::fs::remove_file(&part);
                return Err(format!(
                    "{url}: SHA-1 不符（期望 {expected}，实际 {digest}）"
                ));
            }
        }
        if let Some(size) = job.size {
            if written != size {
                let _ = std::fs::remove_file(&part);
                return Err(format!("{url}: 大小不符（期望 {size}，实际 {written}）"));
            }
        }
        std::fs::rename(&part, &job.dest).map_err(|e| format!("{}: {e}", job.dest.display()))?;
        Ok(())
    }

    /// Downloads files with `threads` workers. `progress(done, bytes)`.
    pub fn many(
        &self,
        jobs: Vec<FileJob>,
        threads: usize,
        cancel: &AtomicBool,
        progress: &(dyn Fn(usize, u64) + Sync),
    ) -> Result<(), String> {
        if jobs.is_empty() {
            return Ok(());
        }
        let total = jobs.len();
        let queue = Mutex::new(jobs.into_iter().enumerate());
        let done = AtomicUsize::new(0);
        let failures: Mutex<Vec<String>> = Mutex::new(Vec::new());
        let bytes = std::sync::atomic::AtomicU64::new(0);
        let threads = threads.clamp(1, 16).min(total);

        std::thread::scope(|scope| {
            for _ in 0..threads {
                scope.spawn(|| loop {
                    if cancel.load(Ordering::Relaxed) {
                        return;
                    }
                    let next = {
                        let mut queue = queue.lock().unwrap();
                        queue.next()
                    };
                    let Some((_, job)) = next else {
                        return;
                    };
                    let result = self.file(&job, cancel, &|add| {
                        bytes.fetch_add(add, Ordering::Relaxed);
                        progress(done.load(Ordering::Relaxed), bytes.load(Ordering::Relaxed));
                    });
                    match result {
                        Ok(_) => {
                            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                            progress(n, bytes.load(Ordering::Relaxed));
                        }
                        Err(e) => {
                            failures.lock().unwrap().push(e);
                            cancel.store(true, Ordering::Relaxed);
                            return;
                        }
                    }
                });
            }
        });

        let failures = failures.into_inner().unwrap();
        if let Some(first) = failures.first() {
            return Err(first.clone());
        }
        if cancel.load(Ordering::Relaxed) {
            return Err("已取消".into());
        }
        Ok(())
    }
}

pub fn sha1_file(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut hasher = Sha1::new();
    let mut buffer = vec![0u8; 256 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

pub fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

pub fn sha1_bytes(bytes: &[u8]) -> String {
    hex(&Sha1::digest(bytes))
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    hex(&sha2::Sha256::digest(bytes))
}
