//! The window: a PCL2-shaped sidebar and four pages (home, resources, settings, about).

use crate::events::{Emitter, Event};
use crate::job::{self, Mode};
use crate::launch;
use crate::minecraft::{self, Presence};
use crate::net::Net;
use crate::paths::Paths;
use crate::prepare;
use crate::settings::{Settings, Source};
use crate::ui::{self, Icon};
use eframe::egui;
use egui::{Align, Color32, Layout, RichText, Vec2};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::Duration;

/// The in-game cap this build allows; kept in step with `options_file.rs`.
pub const MAX_RENDER_DISTANCE: i32 = 512;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Home,
    Resources,
    Settings,
    About,
}

impl Page {
    fn label(self) -> &'static str {
        match self {
            Page::Home => "主页",
            Page::Resources => "资源与下载",
            Page::Settings => "设置",
            Page::About => "关于",
        }
    }

    fn icon(self) -> Icon {
        match self {
            Page::Home => Icon::Play,
            Page::Resources => Icon::Download,
            Page::Settings => Icon::Settings,
            Page::About => Icon::Info,
        }
    }
}

enum Action {
    Play,
    Start(Mode),
    Cancel,
    Save,
    Open(PathBuf),
}

pub struct App {
    paths: Paths,
    settings: Settings,
    page: Page,
    emitter: Arc<Emitter>,
    events: Receiver<Event>,
    cancel: Arc<AtomicBool>,
    running: bool,
    game_running: bool,
    phase: String,
    fraction: f32,
    logs: Vec<String>,
    game_log: Vec<String>,
    error: Option<String>,
    status: (Presence, Presence, Presence, Presence),
    ready: bool,
    pack_url: String,
    notice: Option<(String, std::time::Instant)>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> App {
        ui::install(&cc.egui_ctx);

        let provisional = Paths::resolve("");
        let mut settings = Settings::load(&provisional.settings_file());
        let paths = Paths::resolve(&settings.game_dir);
        settings.adopt_game_options(&paths.game_options());

        // Debug/automation: open a page directly (`LITECRAFT_LAUNCHER_PAGE=settings`).
        let page = match std::env::var("LITECRAFT_LAUNCHER_PAGE").as_deref() {
            Ok("resources") => Page::Resources,
            Ok("settings") => Page::Settings,
            Ok("about") => Page::About,
            _ => Page::Home,
        };

        let (tx, events) = mpsc::channel();
        let mut app = App {
            paths,
            pack_url: settings.resource_pack_url.clone(),
            settings,
            page,
            emitter: Arc::new(Emitter::new(tx)),
            events,
            cancel: Arc::new(AtomicBool::new(false)),
            running: false,
            game_running: false,
            phase: "等待操作".into(),
            fraction: 0.0,
            logs: Vec::new(),
            game_log: Vec::new(),
            error: None,
            status: (
                Presence::Missing,
                Presence::Missing,
                Presence::Missing,
                Presence::Missing,
            ),
            ready: false,
            notice: None,
        };
        app.refresh();
        app.log(format!(
            "LiteCraft Launther {} · 数据目录 {}",
            env!("CARGO_PKG_VERSION"),
            app.paths.data.display()
        ));
        if app.paths.bundled_resources.is_some() {
            app.log("已检测到随启动器附带的本体资源，可直接启动");
        }
        app
    }

    fn refresh(&mut self) {
        self.status = minecraft::status(&self.paths);
        self.ready = prepare::is_ready(&self.paths, self.settings.download_sounds);
    }

    fn log(&mut self, line: impl Into<String>) {
        self.logs.push(line.into());
        if self.logs.len() > 500 {
            self.logs.drain(..100);
        }
        prepare::append_log(
            &self.paths,
            self.logs.last().map(String::as_str).unwrap_or(""),
        );
    }

    fn start(&mut self, mode: Mode) {
        if self.running || self.game_running {
            return;
        }
        self.running = true;
        self.error = None;
        self.fraction = 0.0;
        self.phase = "开始…".into();
        self.logs.clear();
        self.cancel = Arc::new(AtomicBool::new(false));

        let paths = self.paths.clone();
        let settings = self.settings.clone();
        let emitter = Arc::clone(&self.emitter);
        let cancel = Arc::clone(&self.cancel);
        std::thread::spawn(move || {
            let net = Net::new();
            let result = job::run(mode, &paths, &settings, &net, &cancel, &emitter);
            emitter.send(Event::Done(result));
        });
    }

    fn play(&mut self) {
        if self.running || self.game_running {
            return;
        }
        if !self.ready {
            self.log("资源还不完整，先下载并准备");
            self.start(Mode::All);
            return;
        }
        if let Err(e) = self.settings.sync_game_options(&self.paths.game_options()) {
            self.error = Some(format!("写入视距设置失败：{e}"));
            return;
        }
        let _ = self.settings.save(&self.paths.settings_file());

        self.game_running = true;
        self.game_log.clear();
        self.error = None;
        let paths = self.paths.clone();
        let settings = self.settings.clone();
        let emitter = Arc::clone(&self.emitter);
        std::thread::spawn(move || match launch::launch(&paths, &settings, &emitter) {
            Ok(code) => emitter.send(Event::GameExited(code)),
            Err(e) => {
                emitter.log(format!("启动失败：{e}"));
                emitter.send(Event::GameExited(-1));
            }
        });
    }

    fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.events.try_recv() {
            match event {
                Event::Log(line) => self.log(line),
                Event::Phase(phase) => {
                    self.phase = phase;
                    self.fraction = 0.0;
                }
                Event::Fraction(value) => self.fraction = value,
                Event::Done(Ok(())) => {
                    self.running = false;
                    self.fraction = 1.0;
                    self.phase = "完成".into();
                    self.notice = Some(("操作完成".into(), std::time::Instant::now()));
                    self.refresh();
                }
                Event::Done(Err(error)) => {
                    self.running = false;
                    self.error = Some(error.clone());
                    self.log(format!("错误：{error}"));
                }
                Event::GameLog(line) => {
                    self.game_log.push(line);
                    if self.game_log.len() > 800 {
                        self.game_log.drain(..200);
                    }
                }
                Event::GameExited(code) => {
                    self.game_running = false;
                    self.log(if code == 0 {
                        "游戏正常退出"
                    } else {
                        "游戏异常退出，详情见日志"
                    });
                }
            }
        }
        if self
            .notice
            .as_ref()
            .is_some_and(|(_, when)| when.elapsed() > Duration::from_secs(4))
        {
            self.notice = None;
        }
        if self.running || self.game_running {
            ctx.request_repaint_after(Duration::from_millis(120));
        }
    }

    fn apply(&mut self, action: Action) {
        match action {
            Action::Play => self.play(),
            Action::Start(mode) => self.start(mode),
            Action::Cancel => {
                self.cancel.store(true, Ordering::Relaxed);
                self.log("正在取消…");
            }
            Action::Save => {
                self.settings.resource_pack_url = self.pack_url.clone();
                self.paths = Paths::resolve(&self.settings.game_dir);
                if let Err(e) = self.settings.save(&self.paths.settings_file()) {
                    self.error = Some(e);
                }
                self.refresh();
            }
            Action::Open(path) => open_path(&path),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll(ctx);
        egui::TopBottomPanel::top("topbar")
            .exact_height(56.0)
            .frame(
                egui::Frame::none()
                    .fill(ui::CARD)
                    .stroke(egui::Stroke::new(1.0_f32, ui::BORDER)),
            )
            .show(ctx, |ui| self.topbar(ui));
        egui::SidePanel::left("nav")
            .exact_width(200.0)
            .resizable(false)
            .frame(egui::Frame::none().fill(ui::SIDEBAR))
            .show(ctx, |ui| self.sidebar(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(ui::BG))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let full = ui.available_width();
                        let width = full.min(940.0);
                        let pad = ((full - width) * 0.5).max(0.0);
                        ui.horizontal(|ui| {
                            ui.add_space(pad);
                            ui.vertical(|ui| {
                                ui.set_width(width);
                                ui.add_space(14.0);
                                match self.page {
                                    Page::Home => self.home(ui),
                                    Page::Resources => self.resources(ui),
                                    Page::Settings => self.settings_page(ui),
                                    Page::About => self.about(ui),
                                }
                                ui.add_space(24.0);
                            });
                        });
                    });
            });
    }
}

impl App {
    fn topbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(18.0);
            ui.vertical(|ui| {
                ui.add_space(9.0);
                ui.label(RichText::new("LiteCraft Launther").size(15.0).strong());
                ui.label(
                    RichText::new(format!(
                        "v{} · Minecraft {}",
                        env!("CARGO_PKG_VERSION"),
                        crate::paths::GAME_VERSION
                    ))
                    .size(11.0)
                    .color(ui::MUTED),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(18.0);
                if ui::chip_button(ui, "打开数据目录", false).clicked() {
                    let path = self.paths.data.clone();
                    self.apply(Action::Open(path));
                }
                if let Some((text, _)) = &self.notice {
                    ui::pill(ui, text, Color32::from_rgb(0xE1, 0xF2, 0xE6), ui::GREEN);
                }
                let (text, fill, ink) = if self.game_running {
                    ("游戏运行中", Color32::from_rgb(0xE1, 0xF2, 0xE6), ui::GREEN)
                } else if self.running {
                    ("处理中", Color32::from_rgb(0xFF, 0xF3, 0xDC), ui::AMBER)
                } else if self.ready {
                    ("资源就绪", Color32::from_rgb(0xE1, 0xF2, 0xE6), ui::GREEN)
                } else {
                    ("资源未准备", Color32::from_rgb(0xFF, 0xF3, 0xDC), ui::AMBER)
                };
                ui::pill(ui, text, fill, ink);
            });
        });
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(18.0);
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(40.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(rect, egui::Rounding::same(9.0), ui::ACCENT);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "LC",
                egui::FontId::proportional(17.0),
                Color32::WHITE,
            );
            ui.add_space(2.0);
            ui.vertical(|ui| {
                ui.add_space(4.0);
                ui.label(RichText::new("LiteCraft").size(15.0).strong());
                ui.label(RichText::new("Launther").size(11.0).color(ui::MUTED));
            });
        });
        ui.add_space(22.0);

        let mut go = None;
        for page in [Page::Home, Page::Resources, Page::Settings, Page::About] {
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                if ui::nav_item(ui, page.icon(), page.label(), self.page == page).clicked() {
                    go = Some(page);
                }
            });
        }
        if let Some(page) = go {
            self.page = page;
        }

        let height = ui.available_height();
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), height),
            Layout::bottom_up(Align::LEFT),
            |ui| {
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(
                        RichText::new("MIT · 非官方项目")
                            .size(10.5)
                            .color(ui::MUTED),
                    );
                });
            },
        );
    }

    fn home(&mut self, ui: &mut egui::Ui) {
        ui::page_title(
            ui,
            "主页",
            "Minecraft Java Edition 26.3 · Rust 原生客户端 · 开箱即玩",
        );
        let mut action: Option<Action> = None;

        ui::card(ui, |ui| {
            let compact = ui.available_width() < 640.0;
            let label = if self.game_running {
                "游戏运行中"
            } else if self.running {
                "处理中…"
            } else if self.ready {
                "启动游戏"
            } else {
                "下载并准备"
            };
            let enabled = !self.running && !self.game_running;
            let title = |ui: &mut egui::Ui| {
                ui.add_space(4.0);
                ui.label(RichText::new("LiteCraft").size(28.0).strong());
                ui.label(
                    RichText::new("Minecraft Java Edition 26.3 · Rust 原生客户端")
                        .size(13.0)
                        .color(ui::MUTED),
                );
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui::pill(
                        ui,
                        &format!("版本 {}", env!("CARGO_PKG_VERSION")),
                        ui::ACCENT_SOFT,
                        ui::ACCENT,
                    );
                    ui::pill(
                        ui,
                        &format!("视距 {}", self.settings.render_distance),
                        Color32::from_rgb(0xEC, 0xF1, 0xF8),
                        ui::MUTED,
                    );
                });
            };
            let logo = |ui: &mut egui::Ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(76.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, egui::Rounding::same(14.0), ui::ACCENT);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "LC",
                    egui::FontId::proportional(30.0),
                    Color32::WHITE,
                );
                ui.add_space(14.0);
            };
            if compact {
                ui.horizontal(|ui| {
                    logo(ui);
                    ui.vertical(|ui| {
                        ui.set_max_width(ui.available_width());
                        title(ui);
                    });
                });
                ui.add_space(6.0);
                if ui::primary_button(ui, label, Vec2::new(ui.available_width(), 52.0), enabled)
                    .clicked()
                {
                    action = Some(if self.ready {
                        Action::Play
                    } else {
                        Action::Start(Mode::All)
                    });
                }
            } else {
                ui.horizontal(|ui| {
                    // Reserve the launch button's column before the title grows: a wide title
                    // otherwise leaves the button no room and pushes it off the card.
                    let title_width = (ui.available_width() - 90.0 - 220.0).max(180.0);
                    logo(ui);
                    ui.vertical(|ui| {
                        ui.set_max_width(title_width);
                        title(ui);
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui::primary_button(ui, label, Vec2::new(204.0, 58.0), enabled).clicked()
                        {
                            action = Some(if self.ready {
                                Action::Play
                            } else {
                                Action::Start(Mode::All)
                            });
                        }
                    });
                });
            }

            if self.running {
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&self.phase).size(12.5).color(ui::TEXT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{:.0}%", self.fraction * 100.0))
                                .size(12.0)
                                .color(ui::MUTED),
                        );
                    });
                });
                ui::progress(ui, self.fraction, true);
            } else if self.ready {
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui::status_dot(ui, ui::GREEN);
                    ui.label(
                        RichText::new("资源已就绪，点击右上角启动游戏")
                            .size(12.5)
                            .color(ui::MUTED),
                    );
                });
            } else {
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui::status_dot(ui, ui::AMBER);
                    ui.label(
                        RichText::new(
                            "首次运行会自动从 Mojang / BMCLAPI 下载约 700 MB 原版文件并本地准备",
                        )
                        .size(12.5)
                        .color(ui::MUTED),
                    );
                });
            }
            if let Some(error) = &self.error {
                ui.add_space(8.0);
                ui.label(RichText::new(error).size(12.0).color(ui::RED));
            }
        });

        if ui.available_width() >= 720.0 {
            ui.horizontal_top(|ui| {
                let width = (ui.available_width() - 10.0) * 0.5;
                ui.allocate_ui_with_layout(
                    Vec2::new(width, 0.0),
                    Layout::top_down(Align::LEFT),
                    |ui| self.profile_card(ui),
                );
                ui.add_space(10.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(width, 0.0),
                    Layout::top_down(Align::LEFT),
                    |ui| {
                        if let Some(chosen) = self.quick_card(ui) {
                            action = Some(chosen);
                        }
                    },
                );
            });
        } else {
            self.profile_card(ui);
            if let Some(chosen) = self.quick_card(ui) {
                action = Some(chosen);
            }
        }

        ui.add_space(2.0);
        ui::section(ui, "运行日志");
        ui::log_panel(ui, &self.logs, 120.0);

        if let Some(action) = action {
            self.apply(action);
        }
    }

    fn resources(&mut self, ui: &mut egui::Ui) {
        ui::page_title(
            ui,
            "资源与下载",
            "从公开源获取 Minecraft 26.3，并在本地准备游戏资源",
        );
        let mut action: Option<Action> = None;

        ui::card(ui, |ui| {
            ui::section(ui, "下载源");
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                for (source, label) in [
                    (Source::Official, "官方源 Mojang"),
                    (Source::Bmclapi, "BMCLAPI 镜像（推荐国内）"),
                    (Source::Custom, "自定义镜像"),
                ] {
                    if ui::chip_button(ui, label, self.settings.source == source).clicked() {
                        self.settings.source = source.clone();
                        action = Some(Action::Save);
                    }
                }
            });
            if self.settings.source == Source::Custom {
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.settings.custom_base)
                        .hint_text("https://你的镜像根地址")
                        .desired_width(f32::INFINITY),
                );
                ui::muted(
                    ui,
                    "自定义镜像按 Mojang 的路径结构拼接；失败时会自动回退官方源。",
                );
            }
            ui.add_space(2.0);
            ui::muted(ui, "下载的文件都会校验 SHA-1；已存在的文件不会重复下载。");
        });

        let (client, assets, server, prepared) = self.status;
        ui::card(ui, |ui| {
            ui::section(ui, "组件状态");
            ui.add_space(6.0);
            component_row(
                ui,
                "Minecraft 26.3 客户端",
                "版本信息 + 客户端 JAR + 依赖库",
                client,
            );
            component_row(
                ui,
                "原版资源",
                "贴图、字体、语言、音效（约 485 MB）",
                assets,
            );
            component_row(ui, "服务端 JAR", "导出原版数据所需（约 62 MB）", server);
            component_row(ui, "已准备资源", "游戏启动时使用的资源目录", prepared);
            ui.add_space(4.0);
            ui::muted(ui, &format!("资源目录：{}", self.paths.resources.display()));
            if self.paths.bundled_resources.is_some() {
                ui::muted(ui, "当前使用启动器附带的内置资源，无需下载。");
            }
        });

        ui::card(ui, |ui| {
            ui::section(ui, "操作");
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                let enabled = !self.running && !self.game_running;
                if ui::button(ui, "一键下载并准备", enabled).clicked() {
                    action = Some(Action::Start(Mode::All));
                }
                if ui::button(ui, "仅下载原版文件", enabled).clicked() {
                    action = Some(Action::Start(Mode::Download));
                }
                if ui::button(ui, "重新准备资源", enabled).clicked() {
                    action = Some(Action::Start(Mode::Prepare));
                }
                if ui::button(ui, "校验资源", enabled).clicked() {
                    action = Some(Action::Start(Mode::Check));
                }
                if self.running && ui::button(ui, "取消", true).clicked() {
                    action = Some(Action::Cancel);
                }
            });
            if self.settings.download_sounds {
                ui::muted(ui, "包含音效（约 370 MB）；可在设置中关闭以加快首次启动。");
            } else {
                ui::muted(ui, "已关闭音效下载，游戏将静音启动。");
            }
        });

        ui::card(ui, |ui| {
            ui::section(ui, "GitHub 资源包");
            ui.add_space(4.0);
            ui::muted(
                ui,
                "填入一个包含 horizon-resources.json 的 zip 地址（例如 GitHub Release 附件），直接安装准备好的资源。",
            );
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.pack_url)
                        .hint_text("https://github.com/.../resources-26.3.zip")
                        .desired_width(f32::INFINITY),
                );
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui::button(
                    ui,
                    "安装资源包",
                    !self.running && !self.pack_url.trim().is_empty(),
                )
                .clicked()
                {
                    action = Some(Action::Start(Mode::InstallPack(self.pack_url.clone())));
                }
                if ui::button(ui, "保存地址", true).clicked() {
                    self.settings.resource_pack_url = self.pack_url.clone();
                    action = Some(Action::Save);
                }
            });
        });

        if self.running {
            ui::card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&self.phase).size(13.0).strong());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{:.0}%", self.fraction * 100.0))
                                .size(12.0)
                                .color(ui::MUTED),
                        );
                    });
                });
                ui.add_space(2.0);
                ui::progress(ui, self.fraction, true);
            });
        }
        if let Some(error) = &self.error {
            ui::card(ui, |ui| {
                ui.label(
                    RichText::new("上次操作失败")
                        .size(13.0)
                        .strong()
                        .color(ui::RED),
                );
                ui.label(RichText::new(error).size(12.0).color(ui::RED));
            });
        }

        ui::section(ui, "输出");
        ui::log_panel(ui, &self.logs, 180.0);

        if let Some(action) = action {
            self.apply(action);
        }
    }

    fn settings_page(&mut self, ui: &mut egui::Ui) {
        ui::page_title(ui, "设置", "游戏、下载与启动器选项");
        let mut action: Option<Action> = None;
        let mut changed = false;

        ui::card(ui, |ui| {
            ui::section(ui, "游戏");
            ui.add_space(4.0);
            ui.spacing_mut().slider_width = 190.0;
            egui::Grid::new("game-settings")
                .num_columns(2)
                .spacing(egui::vec2(18.0, 10.0))
                .show(ui, |ui| {
                    ui.label(RichText::new("玩家名").size(13.0).color(ui::MUTED));
                    changed |= ui
                        .add(
                            egui::TextEdit::singleline(&mut self.settings.player_name)
                                .desired_width(380.0),
                        )
                        .changed();
                    ui.end_row();

                    ui.label(RichText::new("游戏目录").size(13.0).color(ui::MUTED));
                    let game_hint = self.paths.data.join("game").display().to_string();
                    ui.horizontal(|ui| {
                        changed |= ui
                            .add(
                                egui::TextEdit::singleline(&mut self.settings.game_dir)
                                    .hint_text(game_hint)
                                    .desired_width(380.0),
                            )
                            .changed();
                        if ui::button(ui, "打开", true).clicked() {
                            action = Some(Action::Open(self.paths.game.clone()));
                        }
                    });
                    ui.end_row();

                    ui.label(RichText::new("视距（区块）").size(13.0).color(ui::MUTED));
                    ui.horizontal(|ui| {
                        let before = self.settings.render_distance;
                        let response = ui.add(
                            egui::Slider::new(
                                &mut self.settings.render_distance,
                                2..=MAX_RENDER_DISTANCE,
                            )
                            .show_value(false),
                        );
                        if response.changed() {
                            self.settings.render_distance_dirty = true;
                            changed = true;
                        }
                        let typed = ui.add(
                            egui::DragValue::new(&mut self.settings.render_distance)
                                .range(2..=MAX_RENDER_DISTANCE)
                                .speed(1.0),
                        );
                        if typed.changed() && self.settings.render_distance != before {
                            self.settings.render_distance_dirty = true;
                            changed = true;
                        }
                        ui::muted(ui, "区块");
                    });
                    ui.end_row();

                    ui.label(RichText::new("窗口尺寸").size(13.0).color(ui::MUTED));
                    ui.horizontal(|ui| {
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut self.settings.window_width)
                                    .range(320..=7680),
                            )
                            .changed();
                        ui.label("×");
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut self.settings.window_height)
                                    .range(240..=4320),
                            )
                            .changed();
                        changed |= ui.checkbox(&mut self.settings.fullscreen, "全屏").changed();
                    });
                    ui.end_row();

                    ui.label(RichText::new("额外参数").size(13.0).color(ui::MUTED));
                    changed |= ui
                        .add(
                            egui::TextEdit::singleline(&mut self.settings.extra_args)
                                .hint_text("传给 litecraft.exe 的参数")
                                .desired_width(380.0),
                        )
                        .changed();
                    ui.end_row();
                });
            ui.add_space(4.0);
            ui::muted(
                ui,
                format!(
                    "视距上限 {MAX_RENDER_DISTANCE} 区块，改动会在下次启动时写入游戏 options.txt。\
                     更远的远景可在游戏内用 LOD Distance（远景地形）扩展。"
                ),
            );
        });

        ui::card(ui, |ui| {
            ui::section(ui, "下载");
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                for (source, label) in [
                    (Source::Official, "官方源"),
                    (Source::Bmclapi, "BMCLAPI"),
                    (Source::Custom, "自定义"),
                ] {
                    if ui::chip_button(ui, label, self.settings.source == source).clicked() {
                        self.settings.source = source.clone();
                        changed = true;
                    }
                }
            });
            if self.settings.source == Source::Custom {
                changed |= ui
                    .add(
                        egui::TextEdit::singleline(&mut self.settings.custom_base)
                            .hint_text("https://你的镜像根地址")
                            .desired_width(f32::INFINITY),
                    )
                    .changed();
            }
            ui.add_space(4.0);
            changed |= ui
                .checkbox(&mut self.settings.download_sounds, "下载音效（约 370 MB）")
                .changed();
            ui::muted(ui, "关闭后游戏仍可运行，但没有声音。");
        });

        ui::card(ui, |ui| {
            ui::section(ui, "启动器");
            ui.add_space(4.0);
            kv(ui, "数据目录", &self.paths.data.display().to_string());
            kv(
                ui,
                "便携模式",
                if self.paths.data.join("..").join("portable.txt").is_file() {
                    "已启用"
                } else {
                    "在启动器旁放 portable.txt 可启用"
                },
            );
            ui.horizontal_wrapped(|ui| {
                if ui::button(ui, "打开数据目录", true).clicked() {
                    action = Some(Action::Open(self.paths.data.clone()));
                }
                if ui::button(ui, "打开日志", true).clicked() {
                    action = Some(Action::Open(self.paths.logs.clone()));
                }
            });
        });

        if changed {
            action = Some(Action::Save);
        }
        if let Some(action) = action {
            self.apply(action);
        }
    }

    fn profile_card(&self, ui: &mut egui::Ui) {
        ui::card(ui, |ui| {
            ui::section(ui, "当前档案");
            ui.add_space(4.0);
            kv(ui, "玩家名", &self.settings.player_name);
            kv(ui, "游戏目录", &self.paths.game.display().to_string());
            kv(
                ui,
                "资源来源",
                if self.paths.bundled_resources.is_some() {
                    "启动器内置 resources/"
                } else {
                    "本地准备（%APPDATA%/LiteCraft）"
                },
            );
            kv(
                ui,
                "视距",
                &format!("{} 区块", self.settings.render_distance),
            );
        });
    }

    fn quick_card(&self, ui: &mut egui::Ui) -> Option<Action> {
        let mut action = None;
        ui::card(ui, |ui| {
            ui::section(ui, "快速操作");
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if ui::button(ui, "打开游戏目录", true).clicked() {
                    action = Some(Action::Open(self.paths.game.clone()));
                }
                if ui::button(ui, "打开日志", true).clicked() {
                    action = Some(Action::Open(self.paths.logs.clone()));
                }
            });
            ui.horizontal_wrapped(|ui| {
                if ui::button(ui, "检查资源", !self.running).clicked() {
                    action = Some(Action::Start(Mode::Check));
                }
                if ui::button(ui, "重新准备", !self.running).clicked() {
                    action = Some(Action::Start(Mode::Prepare));
                }
            });
            if self.running && ui::button(ui, "取消当前操作", true).clicked() {
                action = Some(Action::Cancel);
            }
        });
        action
    }

    fn about(&mut self, ui: &mut egui::Ui) {
        ui::page_title(ui, "关于", "LiteCraft Launther");
        ui::card(ui, |ui| {
            ui::section(ui, "LiteCraft Launther 0.1.0");
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "一个面向 LiteCraft 的轻量启动器：从公开源获取 Minecraft 26.3，\
                     在本地准备资源，然后启动 Rust 原生客户端。",
                )
                .size(13.0),
            );
            ui.add_space(6.0);
            kv(ui, "游戏内核", "Horizon 核心（保留）· Rust · MIT");
            kv(ui, "目标版本", "Minecraft Java Edition 26.3");
            kv(ui, "启动器界面", "PCL2 风格（本仓库重新实现）");
        });
        ui::card(ui, |ui| {
            ui::section(ui, "素材与许可");
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "启动器与源码不包含任何 Minecraft 素材。原版贴图、音效与数据在首次运行时\
                     从 Mojang 的公开地址（或用户选择的镜像）下载，并在你的电脑上准备。\
                     Minecraft 的代码、素材与商标属于 Mojang 与 Microsoft。",
                )
                .size(12.5)
                .color(ui::MUTED),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    "LiteCraft 是非官方项目，与 Mojang、Microsoft 及 Horizon 团队无隶属关系。\
                     上游 Horizon 代码以 MIT 许可提供。",
                )
                .size(12.5)
                .color(ui::MUTED),
            );
        });
        ui::card(ui, |ui| {
            ui::section(ui, "链接");
            ui.add_space(4.0);
            ui.hyperlink_to(
                "Horizon 上游仓库",
                "https://github.com/horizon-rust/horizon",
            );
            ui.hyperlink_to(
                "Minecraft 版本清单",
                "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json",
            );
            ui.hyperlink_to("BMCLAPI 镜像", "https://bmclapi2.bangbang93.com");
        });
        ui::card(ui, |ui| {
            ui::section(ui, "游戏日志");
            ui.add_space(4.0);
            ui::log_panel(ui, &self.game_log, 160.0);
        });
    }
}

fn kv(ui: &mut egui::Ui, key: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{key}："))
                .size(12.5)
                .color(ui::MUTED),
        );
        // Paths are long: one line, ellipsised, rather than a card wider than its column.
        ui.add(
            egui::Label::new(RichText::new(value).size(12.5).color(ui::TEXT))
                .truncate()
                .sense(egui::Sense::hover()),
        )
        .on_hover_text(value);
    });
}

fn component_row(ui: &mut egui::Ui, name: &str, detail: &str, state: Presence) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(26.0), egui::Sense::hover());
        let (fill, ink, text) = match state {
            Presence::Ready => (Color32::from_rgb(0xE1, 0xF2, 0xE6), ui::GREEN, "已就绪"),
            Presence::Partial => (Color32::from_rgb(0xFF, 0xF3, 0xDC), ui::AMBER, "不完整"),
            Presence::Missing => (Color32::from_rgb(0xFA, 0xE7, 0xE7), ui::RED, "未下载"),
        };
        ui.painter()
            .rect_filled(rect, egui::Rounding::same(6.0), fill);
        let icon_kind = if state == Presence::Ready {
            Icon::Check
        } else {
            Icon::Download
        };
        ui::icon(ui.painter(), rect.center(), 14.0, icon_kind, ink);
        ui.add_space(8.0);
        ui.vertical(|ui| {
            ui.label(RichText::new(name).size(13.0).strong());
            ui.label(RichText::new(detail).size(11.5).color(ui::MUTED));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui::pill(ui, text, fill, ink);
        });
    });
    ui.add_space(4.0);
}

fn open_path(path: &std::path::Path) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer").arg(path).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
}
