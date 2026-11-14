//! Headless mode: the same download/prepare/launch steps without the window.
//!
//! `litecraft-launther --download`, `--prepare`, `--check`, `--launch`, `--install-pack URL`.
//! The launcher's own scripts and power users use it; the window is just another caller.

use crate::events::{Emitter, Event};
use crate::job::{self, Mode};
use crate::launch;
use crate::net::Net;
use crate::paths::Paths;
use crate::settings::{Settings, Source};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::Arc;

const HELP: &str = "\
LiteCraft Launther

用法：
  litecraft-launther                      打开启动器
  litecraft-launther --download           下载 Minecraft 26.3 的原版文件
  litecraft-launther --prepare            准备（或刷新）游戏资源
  litecraft-launther --check              只检查资源是否完整
  litecraft-launther --launch             直接启动游戏
  litecraft-launther --install-pack URL   安装准备好的资源 zip（如 GitHub Release）

选项：
  --game-dir DIR        游戏目录
  --minecraft-dir DIR   PCL 的 .minecraft（复用原版客户端文件）
  --resources-dir DIR   准备好的资源输出目录
  --version-json FILE   PCL 已安装的版本 JSON
  --client-jar FILE     PCL 已安装的客户端 JAR
  --source official|bmclapi|custom
  --custom-base URL     自定义镜像根地址
  --name NAME           玩家名
  --no-sounds           不下载音效
  -h, --help            显示此帮助
";

pub fn run() -> Option<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return None;
    }

    let mut mode: Option<Mode> = None;
    let mut game_dir = String::new();
    let mut minecraft_dir: Option<String> = None;
    let mut resources_dir: Option<String> = None;
    let mut version_json: Option<String> = None;
    let mut client_jar: Option<String> = None;
    let mut source: Option<Source> = None;
    let mut custom_base: Option<String> = None;
    let mut name: Option<String> = None;
    let mut sounds = true;

    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        let value = |i: &mut usize, what: &str| -> Result<String, String> {
            *i += 1;
            args.get(*i)
                .cloned()
                .ok_or_else(|| format!("{what} 缺少参数"))
        };
        let parsed = match arg {
            "-h" | "--help" => {
                print!("{HELP}");
                return Some(0);
            }
            "--download" => {
                mode = Some(Mode::Download);
                Ok(())
            }
            "--prepare" | "--all" => {
                mode = Some(Mode::All);
                Ok(())
            }
            "--check" => {
                mode = Some(Mode::Check);
                Ok(())
            }
            "--launch" => {
                mode = Some(Mode::Check);
                Ok(())
            }
            "--install-pack" => value(&mut i, "--install-pack").map(|url| {
                mode = Some(Mode::InstallPack(url));
            }),
            "--game-dir" => value(&mut i, "--game-dir").map(|dir| game_dir = dir),
            "--minecraft-dir" => value(&mut i, "--minecraft-dir").map(|dir| minecraft_dir = Some(dir)),
            "--resources-dir" => value(&mut i, "--resources-dir").map(|dir| resources_dir = Some(dir)),
            "--version-json" => value(&mut i, "--version-json").map(|path| version_json = Some(path)),
            "--client-jar" => value(&mut i, "--client-jar").map(|path| client_jar = Some(path)),
            "--source" => value(&mut i, "--source").map(|source_name| {
                source = Some(match source_name.as_str() {
                    "official" => Source::Official,
                    "bmclapi" => Source::Bmclapi,
                    "custom" => Source::Custom,
                    _ => Source::Bmclapi,
                });
            }),
            "--custom-base" => value(&mut i, "--custom-base").map(|base| custom_base = Some(base)),
            "--name" => value(&mut i, "--name").map(|player| name = Some(player)),
            "--no-sounds" => {
                sounds = false;
                Ok(())
            }
            other => Err(format!("未知参数 {other}\n\n{HELP}")),
        };
        if let Err(e) = parsed {
            eprintln!("{e}");
            return Some(2);
        }
        i += 1;
    }

    let Some(mode) = mode else {
        eprintln!("没有指定操作。\n\n{HELP}");
        return Some(2);
    };
    let launch_after = args.iter().any(|a| a == "--launch");

    let mut paths = Paths::resolve(&game_dir);
    if let Some(dir) = minecraft_dir {
        paths.minecraft = std::path::PathBuf::from(dir);
    }
    if let Some(dir) = resources_dir {
        paths.resources = std::path::PathBuf::from(dir);
    }
    if let Some(path) = version_json {
        paths.version_json = Some(std::path::PathBuf::from(path));
    }
    if let Some(path) = client_jar {
        paths.client_jar = Some(std::path::PathBuf::from(path));
    }
    let mut settings = Settings::load(&paths.settings_file());
    if !game_dir.is_empty() {
        settings.game_dir = game_dir;
    }
    if let Some(source) = source {
        settings.source = source;
    }
    if let Some(base) = custom_base {
        settings.custom_base = base;
    }
    if let Some(name) = name {
        settings.player_name = name;
    }
    settings.download_sounds = sounds;
    let _ = std::fs::create_dir_all(&paths.data);

    let (tx, rx) = mpsc::channel();
    let emitter = Arc::new(Emitter::new(tx));
    let printer = std::thread::spawn(move || {
        let mut shown = -1i32;
        for event in rx {
            match event {
                Event::Log(line) => println!("{line}"),
                Event::Phase(phase) => {
                    println!("== {phase}");
                    shown = -1;
                }
                Event::Fraction(fraction) => {
                    let percent = (fraction * 100.0).floor() as i32;
                    if percent != shown {
                        shown = percent;
                        print!("\r  {percent:3}%");
                        use std::io::Write;
                        let _ = std::io::stdout().flush();
                    }
                }
                Event::GameLog(line) => println!("{line}"),
                Event::Done(Ok(())) | Event::GameExited(0) => {}
                Event::Done(Err(error)) => eprintln!("错误：{error}"),
                Event::GameExited(code) => println!("游戏退出码 {code}"),
            }
        }
    });

    let mut code = 0;
    if launch_after {
        if !paths.resources.join("horizon-resources.json").is_file() {
            eprintln!("资源尚未准备：{}", paths.resources.display());
            code = 1;
        } else if let Err(e) = settings.sync_game_options(&paths.game_options()) {
            eprintln!("写入视距设置失败：{e}");
            code = 1;
        } else {
            match launch::launch(&paths, &settings, &emitter) {
                Ok(exit) => code = exit,
                Err(e) => {
                    eprintln!("启动失败：{e}");
                    code = 1;
                }
            }
        }
    } else {
        let net = Net::new();
        let cancel = AtomicBool::new(false);
        match job::run(mode, &paths, &settings, &net, &cancel, &emitter) {
            Ok(()) => println!("\n完成"),
            Err(e) => {
                eprintln!("\n错误：{e}");
                code = 1;
            }
        }
    }

    drop(emitter);
    let _ = printer.join();
    Some(code)
}
