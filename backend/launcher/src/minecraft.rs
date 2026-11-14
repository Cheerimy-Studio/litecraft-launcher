//! Mojang's public metadata and files, the way PCL2 and the official launcher fetch them.
//!
//! Nothing here ships with LiteCraft: the client jar, libraries, assets and server jar are all
//! downloaded from the pinned version's public URLs into an official-launcher-shaped folder.

use crate::net::{FileJob, Net};
use crate::paths::{Paths, GAME_VERSION};
use crate::settings::{Settings, Source};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

pub const JAVA_RUNTIME_INDEX: &str =
    "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";
const BMCLAPI: &str = "https://bmclapi2.bangbang93.com";

/// The lock pins every accepted 26.3 metadata hash; this launcher embeds the same file the
/// game builds against, so a modified repository cannot drift from it unnoticed.
const LOCK: &str = include_str!("../../research-lock.json");

#[derive(Debug, Clone)]
pub struct Pins {
    pub version: String,
    pub manifest_sha1: String,
    pub accepted_manifest: Vec<String>,
    pub client_sha1: String,
    pub server_sha1: String,
    pub accepted_asset_index: Vec<String>,
    pub java_major: u32,
}

pub fn pins() -> Pins {
    let value: Value = serde_json::from_str(LOCK).expect("research-lock.json");
    let mc = &value["minecraft"];
    let strings = |key: &str| -> Vec<String> {
        mc[key]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    Pins {
        version: mc["version"].as_str().unwrap_or(GAME_VERSION).to_owned(),
        manifest_sha1: mc["manifest_sha1"].as_str().unwrap_or("").to_owned(),
        accepted_manifest: strings("accepted_manifest_sha1"),
        client_sha1: mc["client_sha1"].as_str().unwrap_or("").to_owned(),
        server_sha1: mc["server_sha1"].as_str().unwrap_or("").to_owned(),
        accepted_asset_index: strings("accepted_asset_index_sha1"),
        java_major: mc["java_major"].as_u64().unwrap_or(25) as u32,
    }
}

/// A callback bundle: every long step reports through these.
pub struct Job<'a> {
    pub net: &'a Net,
    pub paths: &'a Paths,
    pub settings: &'a Settings,
    pub cancel: &'a AtomicBool,
    pub log: &'a (dyn Fn(&str) + Sync),
    pub phase: &'a (dyn Fn(&str) + Sync),
    pub fraction: &'a (dyn Fn(f32) + Sync),
}

// -------------------------------------------------------------------------------------------------
// Mirror mapping

/// The mirror URL for an official one, when the selected source has one.
pub fn mirror_url(source: &Source, custom_base: &str, url: &str) -> Option<String> {
    match source {
        Source::Official => None,
        Source::Bmclapi => {
            if let Some(rest) = url.strip_prefix("https://piston-meta.mojang.com") {
                return Some(format!("{BMCLAPI}{rest}"));
            }
            if let Some(rest) = url.strip_prefix("https://launchermeta.mojang.com") {
                return Some(format!("{BMCLAPI}{rest}"));
            }
            if let Some(rest) = url.strip_prefix("https://libraries.minecraft.net") {
                return Some(format!("{BMCLAPI}/maven{rest}"));
            }
            if let Some(rest) = url.strip_prefix("https://resources.download.minecraft.net") {
                return Some(format!("{BMCLAPI}/assets{rest}"));
            }
            if let Some(rest) = url.strip_prefix("https://piston-data.mojang.com/v1/objects/") {
                let (hash, file) = rest.split_once('/').unwrap_or((rest, ""));
                return Some(match file {
                    "client.jar" => format!("{BMCLAPI}/version/{GAME_VERSION}/client"),
                    "server.jar" => format!("{BMCLAPI}/version/{GAME_VERSION}/server"),
                    _ => format!("{BMCLAPI}/v1/objects/{hash}/{file}"),
                });
            }
            None
        }
        Source::Custom => {
            let base = custom_base.trim().trim_end_matches('/');
            if base.is_empty() {
                return None;
            }
            let scheme = url.split("://").nth(1)?;
            let path = scheme.split_once('/').map(|(_, p)| p).unwrap_or("");
            Some(format!("{base}/{path}"))
        }
    }
}

/// The URLs to try, best first. The official host is always the last resort.
pub fn urls(settings: &Settings, url: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(mirror) = mirror_url(&settings.source, &settings.custom_base, url) {
        out.push(mirror);
    }
    out.push(url.to_owned());
    out
}

// -------------------------------------------------------------------------------------------------
// Version metadata

#[derive(Debug, Clone, Deserialize)]
pub struct Artifact {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LibDownloads {
    pub artifact: Option<Artifact>,
    #[serde(default)]
    pub classifiers: BTreeMap<String, Artifact>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub downloads: Option<LibDownloads>,
    #[serde(default)]
    pub rules: Option<Vec<Rule>>,
    #[serde(default)]
    pub natives: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub action: String,
    #[serde(default)]
    pub os: Option<OsRule>,
    #[serde(default)]
    pub features: Option<BTreeMap<String, bool>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OsRule {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetIndexRef {
    pub id: String,
    pub sha1: String,
    #[serde(rename = "totalSize")]
    pub total_size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionMeta {
    pub id: String,
    #[serde(rename = "assetIndex")]
    pub asset_index: AssetIndexRef,
    pub downloads: BTreeMap<String, Artifact>,
    pub libraries: Vec<Library>,
    #[serde(rename = "javaVersion", default)]
    pub java_version: Option<Value>,
}

fn os_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

fn os_arch() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else {
        std::env::consts::ARCH
    }
}

fn rule_matches(rule: &Rule) -> bool {
    // No demo account, no custom resolution, no quick play: a feature rule never applies here.
    if rule
        .features
        .as_ref()
        .is_some_and(|features| features.values().any(|on| *on))
    {
        return false;
    }
    if let Some(os) = &rule.os {
        if let Some(name) = &os.name {
            if name != os_name() {
                return false;
            }
        }
        if let Some(arch) = &os.arch {
            if arch != os_arch() {
                return false;
            }
        }
    }
    true
}

fn allowed(library: &Library) -> bool {
    let Some(rules) = &library.rules else {
        return true;
    };
    let mut allow = false;
    for rule in rules {
        if rule_matches(rule) {
            allow = rule.action == "allow";
        }
    }
    allow
}

/// `group:artifact:version[:classifier][@ext]` in Maven's layout.
pub fn maven_path(name: &str, classifier: Option<&str>) -> String {
    let (coords, ext) = name.split_once('@').unwrap_or((name, "jar"));
    let parts: Vec<&str> = coords.split(':').collect();
    if parts.len() < 3 {
        return format!("{coords}.{ext}");
    }
    let (group, artifact, version) = (parts[0], parts[1], parts[2]);
    let file = match classifier.or_else(|| parts.get(3).copied()) {
        Some(classifier) => format!("{artifact}-{version}-{classifier}.{ext}"),
        None => format!("{artifact}-{version}.{ext}"),
    };
    format!("{}/{artifact}/{version}/{file}", group.replace('.', "/"))
}

fn native_classifier(library: &Library) -> Option<String> {
    let natives = library.natives.as_ref()?;
    let key = natives.get(os_name())?;
    let arch = match std::env::consts::ARCH {
        "x86_64" => "64",
        "aarch64" => "arm64",
        _ => "32",
    };
    let key = key.replace("${arch}", arch);
    Some(key)
}

/// Fetch the pinned 26.3 JSON, checking it against every hash the lock accepts.
pub fn ensure_version_json(job: &Job) -> Result<(VersionMeta, PathBuf), String> {
    let pins = pins();
    let dir = job.paths.minecraft.join("versions").join(&pins.version);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!("{}.json", pins.version));

    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Ok(meta) = serde_json::from_str::<VersionMeta>(&text) {
            if meta.id == pins.version {
                return Ok((meta, path));
            }
        }
    }

    let pinned = format!(
        "https://piston-meta.mojang.com/v1/packages/{}/{}.json",
        pins.manifest_sha1, pins.version
    );
    let mut last = String::new();
    for url in urls(job.settings, &pinned) {
        match job.net.text(&url) {
            Ok(text) => {
                let sha1 = crate::net::sha1_bytes(text.as_bytes());
                if !accepted(&pins.accepted_manifest, &sha1) && sha1 != pins.manifest_sha1 {
                    last = format!("{url}: 版本信息 SHA-1 不在研究锁中（{sha1}）");
                    continue;
                }
                let meta: VersionMeta = serde_json::from_str(&text)
                    .map_err(|e| format!("{url}: 版本信息无法解析: {e}"))?;
                std::fs::write(&path, &text).map_err(|e| format!("{}: {e}", path.display()))?;
                (job.log)(&format!("已获取 Minecraft {} 版本信息", meta.id));
                return Ok((meta, path));
            }
            Err(e) => last = e,
        }
    }
    Err(format!(
        "无法获取 Minecraft {0} 版本信息。请检查网络或切换下载源。\n{1}",
        pins.version, last
    ))
}

/// Read the version JSON another launcher (PCL) installed, or fall back to the pinned one.
pub fn load_version_json(job: &Job) -> Result<(VersionMeta, PathBuf), String> {
    let Some(path) = &job.paths.version_json else {
        return ensure_version_json(job);
    };
    let pins = pins();
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let meta: VersionMeta = serde_json::from_str(&text)
        .map_err(|e| format!("{}: 版本信息无法解析: {e}", path.display()))?;
    if meta.id != pins.version {
        return Err(format!(
            "{} 的版本 id 是 {}，不是 {}",
            path.display(),
            meta.id,
            pins.version
        ));
    }
    Ok((meta, path.clone()))
}

fn accepted(list: &[String], sha1: &str) -> bool {
    list.iter().any(|s| s == sha1)
}

// -------------------------------------------------------------------------------------------------
// Files

/// The client jar, every allowed library and the server jar.
pub fn ensure_game_files(job: &Job, meta: &VersionMeta) -> Result<PathBuf, String> {
    let pins = pins();
    let mut jobs = Vec::new();
    let mut bytes = 0u64;

    let client = meta
        .downloads
        .get("client")
        .ok_or("26.3 的版本信息没有客户端 JAR")?;
    if !pins.client_sha1.is_empty() && client.sha1 != pins.client_sha1 {
        return Err(format!(
            "客户端 JAR 的 SHA-1 与研究锁不一致（{} ≠ {}）：版本信息可能被替换",
            client.sha1, pins.client_sha1
        ));
    }
    if let Some(path) = &job.paths.client_jar {
        // PCL already downloaded the client JAR into its own version folder; verify it instead
        // of writing a second copy into .minecraft.
        let actual = crate::net::sha1_file(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if actual != client.sha1 {
            return Err(format!(
                "PCL 安装的客户端 JAR 校验失败（{} ≠ {}）：{}",
                actual,
                client.sha1,
                path.display()
            ));
        }
    } else {
        let client_path = job
            .paths
            .minecraft
            .join("versions")
            .join(&pins.version)
            .join(format!("{}.jar", pins.version));
        push(&mut jobs, job, client, client_path, &mut bytes);
    }

    let libraries = job.paths.minecraft.join("libraries");
    for library in &meta.libraries {
        if !allowed(library) {
            continue;
        }
        let Some(downloads) = &library.downloads else {
            continue;
        };
        if let Some(artifact) = &downloads.artifact {
            let path = libraries.join(maven_path(&library.name, None));
            push(&mut jobs, job, artifact, path, &mut bytes);
        }
        if let Some(classifier) = native_classifier(library) {
            if let Some(artifact) = downloads.classifiers.get(&classifier) {
                let path = libraries.join(maven_path(&library.name, Some(&classifier)));
                push(&mut jobs, job, artifact, path, &mut bytes);
            }
        }
    }

    let server = meta
        .downloads
        .get("server")
        .ok_or("26.3 的版本信息没有服务端 JAR")?;
    if !pins.server_sha1.is_empty() && server.sha1 != pins.server_sha1 {
        return Err(format!(
            "服务端 JAR 的 SHA-1 与研究锁不一致（{} ≠ {}）",
            server.sha1, pins.server_sha1
        ));
    }
    let server_path = job
        .paths
        .minecraft
        .join("server-jars")
        .join(format!("{}.jar", pins.version));
    push(&mut jobs, job, server, server_path.clone(), &mut bytes);

    (job.phase)(&format!(
        "下载客户端、依赖库与服务端（{} 个文件）",
        jobs.len()
    ));
    (job.fraction)(0.0);
    let total = jobs.len();
    let fraction = job.fraction;
    let bytes_total = bytes.max(1);
    job.net.many(jobs, 8, job.cancel, &|done, got| {
        let by_files = done as f32 / total.max(1) as f32;
        fraction(by_files * 0.9 + (got.min(bytes_total) as f32 / bytes_total as f32) * 0.1);
    })?;
    (job.fraction)(1.0);
    Ok(server_path)
}

fn push(jobs: &mut Vec<FileJob>, job: &Job, artifact: &Artifact, dest: PathBuf, bytes: &mut u64) {
    *bytes += artifact.size;
    jobs.push(FileJob {
        urls: urls(job.settings, &artifact.url),
        dest,
        sha1: Some(artifact.sha1.clone()),
        size: Some(artifact.size),
    });
}

// -------------------------------------------------------------------------------------------------
// Assets

#[derive(Debug, Deserialize)]
struct AssetIndex {
    objects: BTreeMap<String, AssetObject>,
}

#[derive(Debug, Deserialize)]
struct AssetObject {
    hash: String,
    size: u64,
}

pub fn ensure_assets(job: &Job, meta: &VersionMeta) -> Result<PathBuf, String> {
    let index_dir = job.paths.minecraft.join("assets").join("indexes");
    std::fs::create_dir_all(&index_dir).map_err(|e| format!("{}: {e}", index_dir.display()))?;
    let index_path = index_dir.join(format!("{}.json", meta.asset_index.id));

    let mut text = None;
    if let Ok(existing) = std::fs::read_to_string(&index_path) {
        if crate::net::sha1_bytes(existing.as_bytes()) == meta.asset_index.sha1 {
            text = Some(existing);
        }
    }
    if text.is_none() {
        (job.phase)("下载资源索引");
        let mut last = String::new();
        for url in urls(job.settings, &meta.asset_index.url) {
            match job.net.text(&url) {
                Ok(fetched) => {
                    let sha1 = crate::net::sha1_bytes(fetched.as_bytes());
                    let pins = pins();
                    if sha1 != meta.asset_index.sha1 && !accepted(&pins.accepted_asset_index, &sha1)
                    {
                        last = format!("{url}: 资源索引 SHA-1 不符（{sha1}）");
                        continue;
                    }
                    std::fs::write(&index_path, &fetched)
                        .map_err(|e| format!("{}: {e}", index_path.display()))?;
                    text = Some(fetched);
                    break;
                }
                Err(e) => last = e,
            }
        }
        if text.is_none() {
            return Err(format!(
                "无法获取资源索引。请检查网络或切换下载源。\n{last}"
            ));
        }
    }

    let index: AssetIndex =
        serde_json::from_str(text.as_deref().unwrap_or("")).map_err(|e| e.to_string())?;
    let objects = job.paths.minecraft.join("assets").join("objects");
    let mut jobs = Vec::with_capacity(index.objects.len());
    for object in index.objects.values() {
        if object.hash.len() < 2 {
            continue;
        }
        let dest = objects.join(&object.hash[..2]).join(&object.hash);
        let url = format!(
            "https://resources.download.minecraft.net/{}/{}",
            &object.hash[..2],
            object.hash
        );
        jobs.push(FileJob {
            urls: urls(job.settings, &url),
            dest,
            sha1: Some(object.hash.clone()),
            size: Some(object.size),
        });
    }

    (job.phase)(&format!(
        "下载原版资源（{} 个文件，约 {} MB）",
        jobs.len(),
        meta.asset_index.total_size / 1_000_000
    ));
    (job.fraction)(0.0);
    let total = jobs.len().max(1);
    let fraction = job.fraction;
    job.net.many(jobs, 12, job.cancel, &|done, _| {
        fraction(done as f32 / total as f32);
    })?;
    (job.fraction)(1.0);
    Ok(index_path)
}

// -------------------------------------------------------------------------------------------------
// Java

/// A Java 25+ home: the folder that holds `bin/java[.exe]`.
pub fn find_java(major: u32) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(home) = std::env::var("JAVA_HOME") {
        if !home.trim().is_empty() {
            candidates.push(PathBuf::from(home));
        }
    }
    for root in java_roots() {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            candidates.push(path.clone());
            candidates.push(path.join("Contents").join("Home"));
        }
    }
    if let Some(java) = which_java() {
        if let Some(home) = java.parent().and_then(Path::parent) {
            candidates.push(home.to_path_buf());
        }
    }

    candidates
        .into_iter()
        .find(|home| java_major(home).is_some_and(|found| found >= major))
}

fn java_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if cfg!(windows) {
        for base in [
            std::env::var_os("ProgramFiles"),
            std::env::var_os("ProgramFiles(x86)"),
            std::env::var_os("LOCALAPPDATA").map(|p| {
                let mut path = PathBuf::from(p);
                path.push("Programs");
                path.into_os_string()
            }),
        ]
        .into_iter()
        .flatten()
        {
            let base = PathBuf::from(base);
            for vendor in [
                "Java",
                "Eclipse Adoptium",
                "Zulu",
                "Microsoft",
                "BellSoft",
                "Amazon Corretto",
            ] {
                roots.push(base.join(vendor));
            }
        }
    } else if cfg!(target_os = "macos") {
        roots.push(PathBuf::from("/Library/Java/JavaVirtualMachines"));
        if let Some(home) = dirs::home_dir() {
            roots.push(home.join("Library/Java/JavaVirtualMachines"));
        }
    } else {
        roots.push(PathBuf::from("/usr/lib/jvm"));
    }
    roots
}

fn which_java() -> Option<PathBuf> {
    let exe = if cfg!(windows) { "java.exe" } else { "java" };
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(exe))
        .find(|p| p.is_file())
}

fn java_major(home: &Path) -> Option<u32> {
    let exe = home
        .join("bin")
        .join(if cfg!(windows) { "java.exe" } else { "java" });
    if !exe.is_file() {
        return None;
    }
    let mut command = std::process::Command::new(&exe);
    command.arg("-version");
    hide_console(&mut command);
    let output = command.output().ok()?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let quote = text.split('"').nth(1)?;
    let major = quote.split(['.', '-', '+']).next()?.parse::<u32>().ok()?;
    // "1.8.0" style versions keep the real major in the second component.
    if major == 1 {
        quote.split('.').nth(1)?.parse().ok()
    } else {
        Some(major)
    }
}

pub fn hide_console(command: &mut std::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
}

/// Find Java 25, or fetch Mojang's own runtime for this platform.
pub fn ensure_java(job: &Job, major: u32, component_hint: Option<&str>) -> Result<PathBuf, String> {
    if let Some(home) = find_java(major) {
        (job.log)(&format!("使用 Java {}", home.display()));
        return Ok(home);
    }
    if let Some(home) = find_mojang_runtime(job.paths, major) {
        return Ok(home);
    }

    (job.phase)(&format!("下载 Java {major} 运行时"));
    let platform = platform_key().ok_or("此平台没有 Mojang 的 Java 运行时")?;
    let index: Value = serde_json::from_str(
        &job.net
            .text(JAVA_RUNTIME_INDEX)
            .map_err(|e| format!("无法获取 Java 运行时列表: {e}"))?,
    )
    .map_err(|e| e.to_string())?;
    let component = component_hint
        .filter(|name| index[platform].get(*name).is_some())
        .map(str::to_owned)
        .or_else(|| {
            index[platform].as_object().and_then(|entries| {
                entries
                    .iter()
                    .find(|(name, _)| name.starts_with("java-runtime"))
                    .map(|(name, _)| name.clone())
            })
        })
        .ok_or_else(|| format!("Mojang 的运行时列表里没有 {platform} 的 Java {major}"))?;
    let entries = index[platform][&component]
        .as_array()
        .ok_or("Java 运行时列表格式错误")?;
    let entry = entries
        .iter()
        .find(|e| {
            e["version"]["name"]
                .as_str()
                .is_some_and(|name| name.starts_with(&format!("{major}.")))
        })
        .or_else(|| entries.first())
        .ok_or("Java 运行时列表为空")?;
    let manifest_url = entry["manifest"]["url"]
        .as_str()
        .ok_or("Java 运行时没有清单")?;
    let manifest: Value = serde_json::from_str(
        &job.net
            .text(manifest_url)
            .map_err(|e| format!("无法获取 Java 运行时清单: {e}"))?,
    )
    .map_err(|e| e.to_string())?;

    let home = job
        .paths
        .minecraft
        .join("runtime")
        .join(&component)
        .join(platform)
        .join(&component);
    let mut jobs = Vec::new();
    if let Some(files) = manifest["files"].as_object() {
        for (name, file) in files {
            if file["type"].as_str() != Some("file") {
                continue;
            }
            let Some(raw) = file["downloads"]["raw"].as_object() else {
                continue;
            };
            let (Some(url), Some(sha1), Some(size)) = (
                raw.get("url").and_then(Value::as_str),
                raw.get("sha1").and_then(Value::as_str),
                raw.get("size").and_then(Value::as_u64),
            ) else {
                continue;
            };
            jobs.push(FileJob {
                urls: urls(job.settings, url),
                dest: home.join(name),
                sha1: Some(sha1.to_owned()),
                size: Some(size),
            });
        }
    }
    let total = jobs.len().max(1);
    let fraction = job.fraction;
    job.net.many(jobs, 8, job.cancel, &|done, _| {
        fraction(done as f32 / total as f32);
    })?;
    (job.log)(&format!("Java 已下载到 {}", home.display()));
    Ok(home)
}

fn platform_key() -> Option<&'static str> {
    Some(match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "windows-x64",
        ("windows", "x86") => "windows-x86",
        ("windows", "aarch64") => "windows-arm64",
        ("linux", "x86_64") => "linux",
        ("linux", "x86") => "linux-i386",
        ("macos", "x86_64") => "mac-os",
        ("macos", "aarch64") => "mac-os-arm64",
        _ => return None,
    })
}

fn find_mojang_runtime(paths: &Paths, major: u32) -> Option<PathBuf> {
    let root = paths.minecraft.join("runtime");
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let component = entry.path();
        let Ok(platforms) = std::fs::read_dir(&component) else {
            continue;
        };
        for platform in platforms.flatten() {
            let home = platform.path().join(component.file_name()?);
            if java_major(&home).is_some_and(|found| found >= major) {
                return Some(home);
            }
        }
    }
    None
}

// -------------------------------------------------------------------------------------------------
// Status for the UI

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Presence {
    Ready,
    Missing,
    Partial,
}

/// Cheap checks only: the full check (hashing every file) runs on demand.
pub fn status(paths: &Paths) -> (Presence, Presence, Presence, Presence) {
    let version = paths
        .minecraft
        .join("versions")
        .join(GAME_VERSION)
        .join(format!("{GAME_VERSION}.json"));
    let client = paths
        .minecraft
        .join("versions")
        .join(GAME_VERSION)
        .join(format!("{GAME_VERSION}.jar"));
    let server = paths
        .minecraft
        .join("server-jars")
        .join(format!("{GAME_VERSION}.jar"));
    let index = paths
        .minecraft
        .join("assets")
        .join("indexes")
        .join("34.json");
    let presence = |paths: &[&Path]| {
        let existing = paths.iter().filter(|p| p.is_file()).count();
        match existing {
            0 => Presence::Missing,
            n if n == paths.len() => Presence::Ready,
            _ => Presence::Partial,
        }
    };
    (
        presence(&[&version, &client]),
        presence(&[&index]),
        presence(&[&server]),
        if paths.resources.join("horizon-resources.json").is_file() {
            Presence::Ready
        } else {
            Presence::Missing
        },
    )
}
