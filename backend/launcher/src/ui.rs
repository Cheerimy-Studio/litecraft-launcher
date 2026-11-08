//! PCL2-inspired light theme: blue accent, white cards, a quiet sidebar.

use egui::{
    Align2, Color32, FontFamily, FontId, Painter, Pos2, Rect, Response, RichText, Rounding, Sense,
    Stroke, Ui, Vec2,
};

pub const BG: Color32 = Color32::from_rgb(0xF1, 0xF4, 0xF9);
pub const SIDEBAR: Color32 = Color32::from_rgb(0xE8, 0xEE, 0xF7);
pub const CARD: Color32 = Color32::WHITE;
pub const BORDER: Color32 = Color32::from_rgb(0xDA, 0xE2, 0xEE);
pub const TEXT: Color32 = Color32::from_rgb(0x2C, 0x35, 0x45);
pub const MUTED: Color32 = Color32::from_rgb(0x76, 0x84, 0x99);
pub const ACCENT: Color32 = Color32::from_rgb(0x2F, 0x7B, 0xD8);
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0x4C, 0x93, 0xE9);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(0xE2, 0xED, 0xFC);
pub const GREEN: Color32 = Color32::from_rgb(0x35, 0x93, 0x50);
pub const RED: Color32 = Color32::from_rgb(0xCE, 0x4B, 0x4B);
pub const AMBER: Color32 = Color32::from_rgb(0xB8, 0x7A, 0x1E);
pub const DARK: Color32 = Color32::from_rgb(0x20, 0x27, 0x33);
pub const DARK_TEXT: Color32 = Color32::from_rgb(0xC5, 0xCE, 0xDB);

pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    // The bundled subset is OFL; system fonts are only a fallback for glyphs it lacks.
    fonts.font_data.insert(
        "litecraft-cjk".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/NotoSansSC-Regular.otf")),
    );
    if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
        family.insert(0, "litecraft-cjk".to_owned());
    }
    if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
        family.push("litecraft-cjk".to_owned());
    }
    ctx.set_fonts(fonts);

    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::light();
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.panel_fill = BG;
    style.visuals.window_fill = CARD;
    style.visuals.extreme_bg_color = CARD;
    style.visuals.faint_bg_color = Color32::from_rgb(0xF7, 0xF9, 0xFC);
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    style.visuals.selection.bg_fill = ACCENT_SOFT;
    style.visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    // Labels wrap at their card's width instead of forcing the window wider.
    style.wrap_mode = Some(egui::TextWrapMode::Wrap);
    style.spacing.button_padding = Vec2::new(12.0, 7.0);
    style.text_styles.insert(
        egui::TextStyle::Body,
        FontId::new(14.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        FontId::new(14.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        FontId::new(12.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        FontId::new(22.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        FontId::new(12.0, FontFamily::Monospace),
    );
    ctx.set_style(style);
}

pub fn page_title(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(title).size(24.0).strong().color(TEXT));
    ui.label(RichText::new(subtitle).size(13.0).color(MUTED));
    ui.add_space(6.0);
}

pub fn section(ui: &mut Ui, title: &str) {
    ui.add_space(2.0);
    ui.label(RichText::new(title).size(15.0).strong().color(TEXT));
    ui.add_space(2.0);
}

pub fn muted(ui: &mut Ui, text: impl Into<String>) {
    ui.label(RichText::new(text).size(12.5).color(MUTED));
}

pub fn card(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    let width = ui.available_width();
    egui::Frame::none()
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .rounding(Rounding::same(8.0))
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            // Both bounds: a long path label must wrap instead of growing the card past its
            // column and pushing the next one off the window.
            let inner = (width - 32.0).max(0.0);
            ui.set_width(inner);
            ui.set_max_width(inner);
            add(ui);
        });
    ui.add_space(10.0);
}

pub fn pill(ui: &mut Ui, text: &str, fill: Color32, ink: Color32) {
    egui::Frame::none()
        .fill(fill)
        .rounding(Rounding::same(10.0))
        .inner_margin(egui::Margin::symmetric(9.0, 3.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.5).color(ink));
        });
}

/// PCL2's big launch button: a solid blue block with a play mark.
pub fn primary_button(ui: &mut Ui, label: &str, size: Vec2, enabled: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let hovered = response.hovered() && enabled;
    let fill = if !enabled {
        Color32::from_rgb(0x9F, 0xB4, 0xCF)
    } else if hovered {
        ACCENT_HOVER
    } else {
        ACCENT
    };
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(8.0), fill);
    let ink = Color32::WHITE;
    let icon_center = Pos2::new(rect.center().x - size.x * 0.5 + 30.0, rect.center().y);
    icon(painter, icon_center, 16.0, Icon::Play, ink);
    painter.text(
        Pos2::new(rect.center().x + 10.0, rect.center().y),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(17.0),
        ink,
    );
    response
}

pub fn button(ui: &mut Ui, label: &str, enabled: bool) -> Response {
    let mut button = egui::Button::new(RichText::new(label).size(13.0).color(if enabled {
        TEXT
    } else {
        MUTED
    }))
    .fill(CARD)
    .stroke(Stroke::new(1.0_f32, BORDER))
    .rounding(Rounding::same(6.0))
    .min_size(Vec2::new(0.0, 30.0));
    if !enabled {
        button = button.fill(Color32::from_rgb(0xF4, 0xF6, 0xFA));
    }
    let response = ui.add(button);
    if enabled {
        response
    } else {
        response.on_hover_text("当前不可用")
    }
}

pub fn chip_button(ui: &mut Ui, label: &str, selected: bool) -> Response {
    let (fill, ink, stroke) = if selected {
        (ACCENT_SOFT, ACCENT, ACCENT)
    } else {
        (CARD, MUTED, BORDER)
    };
    let response = ui.add(
        egui::Button::new(RichText::new(label).size(13.0).color(ink))
            .fill(fill)
            .stroke(Stroke::new(1.0_f32, stroke))
            .rounding(Rounding::same(6.0))
            .min_size(Vec2::new(0.0, 30.0)),
    );
    response
}

pub fn nav_item(ui: &mut Ui, icon_kind: Icon, label: &str, selected: bool) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::click());
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, Rounding::same(6.0), ACCENT_SOFT);
        painter.rect_filled(
            Rect::from_min_size(rect.left_top(), Vec2::new(3.0, rect.height())),
            Rounding::same(2.0),
            ACCENT,
        );
    } else if response.hovered() {
        painter.rect_filled(
            rect,
            Rounding::same(6.0),
            Color32::from_rgb(0xDE, 0xE7, 0xF4),
        );
    }
    let ink = if selected { ACCENT } else { MUTED };
    icon(
        painter,
        Pos2::new(rect.left() + 20.0, rect.center().y),
        17.0,
        icon_kind,
        ink,
    );
    painter.text(
        Pos2::new(rect.left() + 38.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.5),
        if selected { ACCENT } else { TEXT },
    );
    response
}

pub fn progress(ui: &mut Ui, fraction: f32, enabled: bool) {
    let height = 8.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        Rounding::same(4.0),
        Color32::from_rgb(0xE2, 0xE9, 0xF3),
    );
    if enabled {
        let width = rect.width() * fraction.clamp(0.0, 1.0);
        if width > 1.0 {
            painter.rect_filled(
                Rect::from_min_size(rect.left_top(), Vec2::new(width, height)),
                Rounding::same(4.0),
                ACCENT,
            );
        }
    }
}

pub fn status_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
}

pub fn log_panel(ui: &mut Ui, lines: &[String], height: f32) {
    egui::Frame::none()
        .fill(DARK)
        .rounding(Rounding::same(8.0))
        .inner_margin(egui::Margin::same(12.0))
        .show(ui, |ui| {
            ui.set_height(height);
            ui.set_width(ui.available_width());
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if lines.is_empty() {
                        ui.label(
                            RichText::new("（暂无输出）")
                                .monospace()
                                .size(12.0)
                                .color(Color32::from_rgb(0x7C, 0x88, 0x99)),
                        );
                    }
                    for line in lines {
                        let color = if line.contains("错误")
                            || line.contains("失败")
                            || line.contains("无法")
                        {
                            Color32::from_rgb(0xF0, 0x8A, 0x8A)
                        } else {
                            DARK_TEXT
                        };
                        ui.label(RichText::new(line).monospace().size(12.0).color(color));
                    }
                });
        });
}

// -------------------------------------------------------------------------------------------------
// Icons, drawn rather than bundled: crisp at every scale and no asset pipeline.

#[derive(Clone, Copy)]
#[allow(dead_code)] // the full icon set is drawn; not every page uses every glyph yet
pub enum Icon {
    Play,
    Download,
    Settings,
    Info,
    Folder,
    Refresh,
    Check,
    Log,
    Stop,
}

pub fn icon(painter: &Painter, center: Pos2, size: f32, kind: Icon, color: Color32) {
    let s = size * 0.5;
    let stroke = Stroke::new(1.6_f32, color);
    match kind {
        Icon::Play => {
            let points = vec![
                Pos2::new(center.x - s * 0.55, center.y - s * 0.75),
                Pos2::new(center.x - s * 0.55, center.y + s * 0.75),
                Pos2::new(center.x + s * 0.75, center.y),
            ];
            painter.add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
        }
        Icon::Download => {
            painter.line_segment(
                [
                    Pos2::new(center.x, center.y - s * 0.8),
                    Pos2::new(center.x, center.y + s * 0.25),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x - s * 0.45, center.y - s * 0.1),
                    Pos2::new(center.x, center.y + s * 0.4),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x + s * 0.45, center.y - s * 0.1),
                    Pos2::new(center.x, center.y + s * 0.4),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x - s * 0.7, center.y + s * 0.8),
                    Pos2::new(center.x + s * 0.7, center.y + s * 0.8),
                ],
                stroke,
            );
        }
        Icon::Settings => {
            painter.circle_stroke(center, s * 0.38, stroke);
            for i in 0..6 {
                let angle = std::f32::consts::TAU * i as f32 / 6.0;
                let inner = s * 0.55;
                let outer = s * 0.9;
                painter.line_segment(
                    [
                        Pos2::new(
                            center.x + angle.cos() * inner,
                            center.y + angle.sin() * inner,
                        ),
                        Pos2::new(
                            center.x + angle.cos() * outer,
                            center.y + angle.sin() * outer,
                        ),
                    ],
                    stroke,
                );
            }
        }
        Icon::Info => {
            painter.circle_stroke(center, s * 0.85, stroke);
            painter.circle_filled(Pos2::new(center.x, center.y - s * 0.4), 1.5, color);
            painter.line_segment(
                [
                    Pos2::new(center.x, center.y - s * 0.05),
                    Pos2::new(center.x, center.y + s * 0.5),
                ],
                stroke,
            );
        }
        Icon::Folder => {
            let body = Rect::from_center_size(
                Pos2::new(center.x, center.y + s * 0.12),
                Vec2::new(size * 0.9, size * 0.62),
            );
            painter.rect_stroke(body, Rounding::same(2.0), stroke);
            painter.line_segment(
                [
                    Pos2::new(body.left() + 1.0, body.top() - s * 0.2),
                    Pos2::new(body.left() + size * 0.32, body.top() - s * 0.2),
                ],
                stroke,
            );
        }
        Icon::Refresh => {
            let mut points = Vec::new();
            for i in 0..=24 {
                let angle =
                    -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * 0.8 * i as f32 / 24.0;
                points.push(Pos2::new(
                    center.x + angle.cos() * s * 0.7,
                    center.y + angle.sin() * s * 0.7,
                ));
            }
            painter.add(egui::Shape::line(points, stroke));
            let head = Pos2::new(center.x + s * 0.75, center.y - s * 0.2);
            painter.add(egui::Shape::convex_polygon(
                vec![
                    head,
                    Pos2::new(head.x - s * 0.42, head.y - s * 0.1),
                    Pos2::new(head.x - s * 0.12, head.y - s * 0.5),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Check => {
            painter.line_segment(
                [
                    Pos2::new(center.x - s * 0.7, center.y),
                    Pos2::new(center.x - s * 0.15, center.y + s * 0.55),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x - s * 0.15, center.y + s * 0.55),
                    Pos2::new(center.x + s * 0.8, center.y - s * 0.6),
                ],
                stroke,
            );
        }
        Icon::Log => {
            for i in 0..3 {
                let y = center.y + (i as f32 - 1.0) * s * 0.55;
                painter.line_segment(
                    [
                        Pos2::new(center.x - s * 0.8, y),
                        Pos2::new(center.x + s * 0.8, y),
                    ],
                    stroke,
                );
            }
        }
        Icon::Stop => {
            painter.rect_filled(
                Rect::from_center_size(center, Vec2::splat(size * 0.55)),
                Rounding::same(2.0),
                color,
            );
        }
    }
}
