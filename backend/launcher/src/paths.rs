//! Where the launcher keeps everything.
//!
//! A portable copy (`portable.txt` beside the executable) keeps its files beside the launcher;
//! otherwise they live under the user's data directory. A resource folder shipped beside the
//! executable is used as-is, so a packaged build starts without downloading anything.

use std::path::PathBuf;

/// The pinned Minecraft version, the same one the game's `research-lock.json` pins.
pub const GAME_VERSION: &str = "26.3";

#[derive(Clone, Debug)]
pub struct Paths {
    pub exe_dir: PathBuf,
    /// Launcher data: settings, logs, downloads, prepared resources.
    pub data: PathBuf,
    /// Official-launcher layout: versions/26.3, libraries/, assets/.
    pub minecraft: PathBuf,
    /// Prepared resources handed to the game with `--resources`.
    pub resources: PathBuf,
    /// `resources/` (or `resources/26.3/`) shipped beside the executable, when present.
    pub bundled_resources: Option<PathBuf>,
    /// The small `prepare/` tree shipped beside the executable (lock, Java sources, exporters).
    pub prepare_assets: PathBuf,
    /// Writable copy of the prepare tree; `.local/` scratch lives here too.
    pub prepare_root: PathBuf,
    pub game: PathBuf,
    pub logs: PathBuf,
    pub game_exe: Option<PathBuf>,
}

impl Paths {
    pub fn resolve(game_dir_override: &str) -> Paths {
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("litecraft-launther"));
        let exe_dir = exe
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        let portable = exe_dir.join("portable.txt").is_file();
        let data = if portable {
            exe_dir.join("LiteCraftData")
        } else {
            dirs::data_dir()
                .unwrap_or_else(|| exe_dir.clone())
                .join("LiteCraft")
        };

        let bundled = [
            exe_dir.join("resources").join(GAME_VERSION),
            exe_dir.join("resources"),
        ]
        .into_iter()
        .find(|p| p.join("horizon-resources.json").is_file());

        let resources = bundled
            .clone()
            .unwrap_or_else(|| data.join("resources").join(GAME_VERSION));

        let game = if game_dir_override.trim().is_empty() {
            data.join("game")
        } else {
            PathBuf::from(game_dir_override.trim())
        };

        let suffix = std::env::consts::EXE_SUFFIX;
        let game_exe = [
            exe_dir.join(format!("litecraft{suffix}")),
            exe_dir.join("bin").join(format!("litecraft{suffix}")),
            exe_dir
                .join("..")
                .join("target")
                .join("release")
                .join(format!("litecraft{suffix}")),
            data.join("bin").join(format!("litecraft{suffix}")),
        ]
        .into_iter()
        .find(|p| p.is_file());

        Paths {
            minecraft: data.join("minecraft"),
            prepare_assets: exe_dir.join("prepare"),
            prepare_root: data.join("prepare"),
            logs: data.join("logs"),
            exe_dir,
            data,
            resources,
            bundled_resources: bundled,
            game,
            game_exe,
        }
    }

    /// `%APPDATA%\LiteCraft\launcher.json` (or `LiteCraftData\launcher.json` when portable).
    pub fn settings_file(&self) -> PathBuf {
        self.data.join("launcher.json")
    }

    pub fn game_options(&self) -> PathBuf {
        self.game.join("options.txt")
    }
}
