//! Launcher settings, saved as JSON beside the launcher's data.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Where the pinned game files come from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// Mojang's own hosts.
    Official,
    /// BMCLAPI, the mirror PCL2 and other Chinese launchers use.
    Bmclapi,
    /// A mirror that keeps Mojang's paths under `custom_base`.
    Custom,
}

impl Default for Source {
    fn default() -> Self {
        // PCL2's default: Mojang's own hosts, with a mirror one click away.
        Source::Official
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub player_name: String,
    /// Empty means `<data>/game`.
    pub game_dir: String,
    pub render_distance: i32,
    /// Set when the launcher changed `render_distance`; written to options.txt at the next launch.
    pub render_distance_dirty: bool,
    pub window_width: u32,
    pub window_height: u32,
    pub fullscreen: bool,
    pub source: Source,
    pub custom_base: String,
    pub download_sounds: bool,
    pub extra_args: String,
    /// Optional zip of prepared resources, e.g. a GitHub release asset.
    pub resource_pack_url: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            player_name: "Steve".into(),
            game_dir: String::new(),
            render_distance: 16,
            render_distance_dirty: false,
            window_width: 1280,
            window_height: 720,
            fullscreen: false,
            source: Source::default(),
            custom_base: String::new(),
            download_sounds: true,
            extra_args: String::new(),
            resource_pack_url: String::new(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Settings {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let tmp = path.with_extension("json.part");
        std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(())
    }

    /// Reads the render distance the game itself saved, so the launcher shows the value the
    /// player set in-game instead of overwriting it.
    pub fn adopt_game_options(&mut self, path: &Path) {
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        for line in text.lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            match key.trim() {
                "renderDistance" => {
                    if let Ok(v) = value.trim().parse::<i32>() {
                        if !self.render_distance_dirty {
                            self.render_distance = v;
                        }
                    }
                }
                "fullscreen" => {
                    self.fullscreen = value.trim() == "true";
                }
                _ => {}
            }
        }
    }

    /// Writes the launcher's render distance into the game's `options.txt`, keeping every other
    /// line. The game reads this file at startup; the CLI flag would override in-game changes.
    pub fn sync_game_options(&mut self, path: &Path) -> Result<(), String> {
        if !self.render_distance_dirty {
            return Ok(());
        }
        let mut lines: Vec<String> = std::fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect();
        let line = format!("renderDistance:{}", self.render_distance);
        let mut replaced = false;
        for existing in lines.iter_mut() {
            if existing.starts_with("renderDistance:") {
                *existing = line.clone();
                replaced = true;
                break;
            }
        }
        if !replaced {
            lines.push(line);
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        let text = format!("{}\n", lines.join("\n"));
        let tmp = path.with_extension("txt.part");
        std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))?;
        self.render_distance_dirty = false;
        Ok(())
    }
}
