//! One background job: download, prepare, check, or install a resource pack.

use crate::events::Emitter;
use crate::minecraft::{self, Job};
use crate::net::Net;
use crate::paths::Paths;
use crate::prepare;
use crate::settings::Settings;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum Mode {
    /// Download the official files and prepare the resources.
    All,
    /// Download the official files only.
    Download,
    /// Prepare from files already downloaded.
    Prepare,
    /// Report what the prepared folder lacks.
    Check,
    /// Install a prepared-resource zip (a GitHub release asset, for example).
    InstallPack(String),
}

pub fn run(
    mode: Mode,
    paths: &Paths,
    settings: &Settings,
    net: &Net,
    cancel: &AtomicBool,
    emit: &Arc<Emitter>,
) -> Result<(), String> {
    let log = |line: &str| emit.log(line);
    let phase = |line: &str| emit.phase(line);
    let fraction = |value: f32| emit.fraction(value);
    let job = Job {
        net,
        paths,
        settings,
        cancel,
        log: &log,
        phase: &phase,
        fraction: &fraction,
    };

    match mode {
        Mode::All => {
            prepare::run(&job)?;
            Ok(())
        }
        Mode::Download => {
            let (meta, _) = minecraft::ensure_version_json(&job)?;
            minecraft::ensure_game_files(&job, &meta)?;
            minecraft::ensure_assets(&job, &meta)?;
            let pins = minecraft::pins();
            let component = meta
                .java_version
                .as_ref()
                .and_then(|value| value.get("component"))
                .and_then(|value| value.as_str());
            minecraft::ensure_java(&job, pins.java_major, component)?;
            emit.log("原版文件已齐备");
            Ok(())
        }
        Mode::Prepare => {
            prepare::run(&job)?;
            Ok(())
        }
        Mode::Check => {
            prepare::materialize(paths, &log)?;
            let gaps = prepare::gaps(paths)?;
            if gaps.is_empty() {
                emit.log("资源完整，可以启动游戏");
                Ok(())
            } else {
                for gap in &gaps {
                    emit.log(&format!("缺少：{gap}"));
                }
                Err(format!("资源还不完整（{} 项）", gaps.len()))
            }
        }
        Mode::InstallPack(url) => prepare::install_pack(paths, &url, net, emit),
    }
}
