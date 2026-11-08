//! Preparing the game's resources, in process.
//!
//! `horizon-prepare` imports the downloaded official files, runs the pinned exporters and copies
//! the asset-store extras. A packaged launcher ships the prepared folder already, so this runs
//! only on a fresh install or when the player asks for a refresh.

use crate::events::Emitter;
use crate::minecraft::{self, Job};
use crate::net::{sha256_bytes, Net};
use crate::paths::{Paths, GAME_VERSION};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Copies the packaged `prepare/` tree (research lock, exporter sources, precompiled exporters)
/// into the writable data folder. `.local/` is left alone: it holds the imported jars.
pub fn materialize(paths: &Paths, log: &(dyn Fn(&str) + Sync)) -> Result<(), String> {
    let source = &paths.prepare_assets;
    let lock = source.join("research-lock.json");
    if !lock.is_file() {
        return Err(format!(
            "安装目录缺少资源准备文件：{}\n请使用完整的 LiteCraft Launther 发布包。",
            lock.display()
        ));
    }
    let destination = &paths.prepare_root;
    std::fs::create_dir_all(destination).map_err(|e| format!("{}: {e}", destination.display()))?;

    // The exporter bundle is part of the stamp too: replacing the shipped classes must replace
    // the copies already under the data folder, even when the lock and version did not change.
    let exporters =
        std::fs::read(source.join("exporters").join("manifest.json")).unwrap_or_default();
    let stamp = format!(
        "{}
{}
{}
",
        env!("CARGO_PKG_VERSION"),
        sha256_bytes(&std::fs::read(&lock).map_err(|e| e.to_string())?),
        sha256_bytes(&exporters)
    );
    let marker = destination.join(".materials");
    let current = std::fs::read_to_string(&marker).unwrap_or_default();
    if current == stamp && destination.join("research-lock.json").is_file() {
        return Ok(());
    }

    log("正在安装资源准备组件…");
    copy_tree(source, destination)?;
    std::fs::write(&marker, stamp).map_err(|e| format!("{}: {e}", marker.display()))?;
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    let entries = std::fs::read_dir(source).map_err(|e| format!("{}: {e}", source.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if name == ".local" {
            continue;
        }
        let target = destination.join(&name);
        if path.is_dir() {
            std::fs::create_dir_all(&target).map_err(|e| format!("{}: {e}", target.display()))?;
            copy_tree(&path, &target)?;
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("{}: {e}", parent.display()))?;
            }
            std::fs::copy(&path, &target).map_err(|e| format!("{}: {e}", target.display()))?;
        }
    }
    Ok(())
}

/// What the prepared folder still lacks; empty when the game can start.
pub fn gaps(paths: &Paths) -> Result<Vec<String>, String> {
    if !paths.prepare_root.join("research-lock.json").is_file() {
        return Ok(vec!["资源准备组件尚未安装".into()]);
    }
    horizon_prepare::check(&paths.prepare_root, &paths.resources)
}

pub fn is_ready(paths: &Paths, sounds: bool) -> bool {
    if !paths.resources.join("horizon-resources.json").is_file() {
        return false;
    }
    // A packaged resource folder is usable without the local prepare tree.
    if !paths.prepare_root.join("research-lock.json").is_file() {
        return true;
    }
    match gaps(paths) {
        Ok(list) => list
            .iter()
            .all(|gap| sounds || !gap.to_lowercase().contains("sound")),
        Err(_) => true,
    }
}

/// Prepare (or refresh) the resources. `job` supplies the network, the paths and the progress
/// callbacks; the version's Java is found or downloaded first.
pub fn run(job: &Job) -> Result<PathBuf, String> {
    let paths = job.paths;
    let pins = minecraft::pins();

    let (meta, _) = minecraft::ensure_version_json(job)?;
    let server_jar = minecraft::ensure_game_files(job, &meta)?;
    minecraft::ensure_assets(job, &meta)?;
    let java = minecraft::ensure_java(job, pins.java_major, java_component(&meta))?;

    materialize(paths, job.log)?;

    (job.phase)("准备游戏资源（导出原版数据，可能需要几分钟）");
    (job.fraction)(0.0);

    let options = horizon_prepare::Options {
        root: paths.prepare_root.clone(),
        import: true,
        minecraft_dir: Some(paths.minecraft.clone()),
        server_jar: Some(server_jar),
        java_home: Some(java),
        inputs: Default::default(),
        output: Some(paths.resources.clone()),
    };
    let log = |line: &str| (job.log)(line);
    let progress = |value: f32| (job.fraction)(value);
    horizon_prepare::prepare_with_progress(&options, &log, &progress).map_err(|e| {
        format!(
            "{e}\n\n如果提示缺少 javac，请安装 JDK 25；发布包已附带编译好的导出器，通常不需要。"
        )
    })?;

    (job.phase)("复制音效、全景图、字体与语言");
    for kind in ["panorama", "fonts", "packs", "languages"] {
        if let Err(e) = horizon_prepare::add_assets(
            kind,
            &paths.prepare_root,
            Some(&paths.resources),
            Some(&paths.minecraft),
            &Default::default(),
            &log,
        ) {
            (job.log)(&format!("附加资源 {kind} 未完成：{e}"));
        }
    }
    if job.settings.download_sounds {
        if let Err(e) = horizon_prepare::add_assets(
            "sounds",
            &paths.prepare_root,
            Some(&paths.resources),
            Some(&paths.minecraft),
            &Default::default(),
            &log,
        ) {
            (job.log)(&format!("音效未复制，游戏将静音：{e}"));
        }
    }

    match gaps(paths) {
        Ok(list) if list.is_empty() => (job.log)("资源准备完成"),
        Ok(list) => {
            for gap in list {
                (job.log)(&format!("资源检查：{gap}"));
            }
        }
        Err(e) => (job.log)(&format!("资源检查失败：{e}")),
    }
    (job.fraction)(1.0);
    Ok(paths.resources.clone())
}

fn java_component(meta: &minecraft::VersionMeta) -> Option<&str> {
    meta.java_version
        .as_ref()
        .and_then(|value| value.get("component"))
        .and_then(|value| value.as_str())
}

/// Install a prepared-resource zip (for example a GitHub release asset) into the resource folder.
pub fn install_pack(paths: &Paths, url: &str, net: &Net, emitter: &Emitter) -> Result<(), String> {
    if url.trim().is_empty() {
        return Err("请填写资源包地址（.zip）".into());
    }
    let cache = paths.data.join("cache");
    std::fs::create_dir_all(&cache).map_err(|e| format!("{}: {e}", cache.display()))?;
    let archive_path = cache.join("resource-pack.zip");
    emitter.phase("下载资源包");
    let job = crate::net::FileJob {
        urls: vec![url.trim().to_owned()],
        dest: archive_path.clone(),
        sha1: None,
        size: None,
    };
    let cancel = std::sync::atomic::AtomicBool::new(false);
    net.file(&job, &cancel, &|_| {})?;

    emitter.phase("解压资源包");
    let extract = cache.join("resource-pack");
    if extract.exists() {
        std::fs::remove_dir_all(&extract).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(&extract).map_err(|e| e.to_string())?;
    let file = std::fs::File::open(&archive_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("这个文件不是可用的 zip：{e}"))?;
    let total = zip.len().max(1);
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let Some(relative) = entry.enclosed_name() else {
            continue;
        };
        let target = extract.join(&relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = std::fs::File::create(&target).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
        if i % 200 == 0 {
            emitter.fraction(i as f32 / total as f32);
        }
    }

    let root = find_root(&extract).ok_or("资源包里没有 horizon-resources.json")?;
    emitter.phase("安装资源包");
    if paths.resources.exists() {
        // Keep the old one for a single rollback if the copy fails halfway.
        let backup = paths
            .resources
            .with_file_name(format!("{GAME_VERSION}.old"));
        let _ = std::fs::remove_dir_all(&backup);
        let _ = std::fs::rename(&paths.resources, &backup);
    }
    std::fs::create_dir_all(&paths.resources).map_err(|e| e.to_string())?;
    copy_contents(&root, &paths.resources)?;
    if !paths.resources.join("horizon-resources.json").is_file() {
        return Err("资源包安装后缺少 horizon-resources.json".into());
    }
    emitter.log("资源包已安装");
    emitter.fraction(1.0);
    Ok(())
}

fn find_root(dir: &Path) -> Option<PathBuf> {
    if dir.join("horizon-resources.json").is_file() {
        return Some(dir.to_path_buf());
    }
    let mut children: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    if children.len() == 1 {
        let child = children.pop()?;
        if child.join("horizon-resources.json").is_file() {
            return Some(child);
        }
    }
    None
}

fn copy_contents(source: &Path, destination: &Path) -> Result<(), String> {
    for entry in std::fs::read_dir(source)
        .map_err(|e| format!("{}: {e}", source.display()))?
        .flatten()
    {
        let path = entry.path();
        let target = destination.join(entry.file_name());
        if path.is_dir() {
            std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
            copy_contents(&path, &target)?;
        } else {
            std::fs::copy(&path, &target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Write a line to the launcher's own log, for crashes the player may need to send.
pub fn append_log(paths: &Paths, line: &str) {
    let _ = std::fs::create_dir_all(&paths.logs);
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(paths.logs.join("launcher.log"))
    {
        let _ = writeln!(file, "{line}");
    }
}
