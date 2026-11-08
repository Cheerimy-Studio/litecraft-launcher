//! Starting the game process and streaming its output to the window.

use crate::events::{Emitter, Event};
use crate::minecraft::hide_console;
use crate::paths::Paths;
use crate::settings::Settings;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::Arc;

/// Starts `litecraft.exe` with the prepared resources and this launcher's game folder.
/// Returns the process's exit code.
pub fn launch(paths: &Paths, settings: &Settings, emit: &Arc<Emitter>) -> Result<i32, String> {
    let exe = paths.game_exe.clone().ok_or_else(|| {
        format!(
            "没有找到 litecraft{}。请把游戏和 LiteCraft Launther 放在同一目录，\
             或放到 bin/ 下。\n启动器查找过：{}",
            std::env::consts::EXE_SUFFIX,
            paths.exe_dir.display()
        )
    })?;

    // `current_dir` must exist before the process starts; a fresh install has no game folder.
    std::fs::create_dir_all(&paths.game)
        .map_err(|e| format!("无法创建游戏目录 {}: {e}", paths.game.display()))?;
    let mut command = Command::new(&exe);
    command.current_dir(&paths.game);
    command
        .arg("--resources")
        .arg(&paths.resources)
        .arg("--game-dir")
        .arg(&paths.game)
        .arg("--home")
        .arg(&paths.data)
        .arg("--name")
        .arg(if settings.player_name.trim().is_empty() {
            "Steve"
        } else {
            settings.player_name.trim()
        })
        .arg("--width")
        .arg(settings.window_width.max(320).to_string())
        .arg("--height")
        .arg(settings.window_height.max(240).to_string());
    if settings.fullscreen {
        command.arg("--fullscreen");
    }
    for arg in split_args(&settings.extra_args) {
        command.arg(arg);
    }
    command.env("LITECRAFT_HOME", &paths.data);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    hide_console(&mut command);

    emit.log(&format!("启动 {}", exe.display()));
    let mut child = command
        .spawn()
        .map_err(|e| format!("无法启动 {}: {e}", exe.display()))?;

    let mut readers = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        readers.push(reader(stdout, Arc::clone(emit)));
    }
    if let Some(stderr) = child.stderr.take() {
        readers.push(reader(stderr, Arc::clone(emit)));
    }

    let status = child.wait().map_err(|e| format!("等待游戏退出失败: {e}"))?;
    for handle in readers {
        let _ = handle.join();
    }
    let code = status.code().unwrap_or(-1);
    emit.send(Event::GameLog(format!(
        "游戏已退出{}",
        if code == 0 {
            String::new()
        } else {
            format!("（退出码 {code}）")
        }
    )));
    Ok(code)
}

fn reader<R: std::io::Read + Send + 'static>(
    stream: R,
    emit: Arc<Emitter>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        for line in BufReader::new(stream).lines().map_while(Result::ok) {
            emit.send(Event::GameLog(line));
        }
    })
}

/// Whitespace-separated extra arguments, with `"quoted values"` kept together.
pub fn split_args(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quote = false;
    for ch in text.chars() {
        match ch {
            '"' => quote = !quote,
            c if c.is_whitespace() && !quote => {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}
