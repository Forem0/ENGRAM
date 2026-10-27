use eframe::egui::{self, Color32, FontFamily, FontId, RichText, Stroke, TextStyle};
use engram_core::Category;

pub const PAPER: Color32 = Color32::from_rgb(245, 241, 231);
pub const INK: Color32 = Color32::from_rgb(29, 32, 31);
pub const MUTED: Color32 = Color32::from_rgb(114, 114, 104);
pub const OCHRE: Color32 = Color32::from_rgb(187, 137, 48);
pub const LINE: Color32 = Color32::from_rgb(218, 213, 201);
pub const CARD: Color32 = Color32::from_rgb(252, 249, 242);
pub const RAIL: Color32 = Color32::from_rgb(28, 33, 31);
pub const RAIL_MUTED: Color32 = Color32::from_rgb(153, 160, 147);
pub const RED: Color32 = Color32::from_rgb(165, 75, 58);

pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(26.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(13.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(11.0, FontFamily::Monospace),
        ),
        (
            TextStyle::Small,
            FontId::new(11.0, FontFamily::Proportional),
        ),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(10.0, 9.0);
    style.spacing.button_padding = egui::vec2(13.0, 9.0);
    style.spacing.interact_size.y = 32.0;
    style.visuals = egui::Visuals::light();
    style.visuals.override_text_color = Some(INK);
    style.visuals.panel_fill = PAPER;
    style.visuals.window_fill = CARD;
    style.visuals.extreme_bg_color = CARD;
    style.visuals.faint_bg_color = Color32::from_rgb(237, 233, 222);
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, LINE);
    style.visuals.widgets.inactive.weak_bg_fill = CARD;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, LINE);
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(235, 225, 199);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, OCHRE);
    style.visuals.widgets.active.weak_bg_fill = Color32::from_rgb(225, 209, 169);
    style.visuals.selection.bg_fill = Color32::from_rgb(229, 211, 169);
    style.visuals.selection.stroke = Stroke::new(1.0_f32, INK);
    ctx.set_style(style);
}

pub fn category_color(category: Category) -> Color32 {
    match category {
        Category::Media => Color32::from_rgb(88, 127, 125),
        Category::Secret => Color32::from_rgb(177, 107, 65),
        Category::Document => Color32::from_rgb(130, 140, 87),
        Category::Browser => Color32::from_rgb(98, 119, 156),
        Category::Message => Color32::from_rgb(143, 113, 139),
        Category::Location => Color32::from_rgb(170, 147, 83),
        Category::Database => Color32::from_rgb(103, 109, 110),
    }
}

pub fn eyebrow(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).monospace().size(10.0).color(MUTED));
}

pub fn pill(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.10))
        .corner_radius(3)
        .inner_margin(egui::Margin::symmetric(7, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(text).monospace().size(10.0).color(color));
        });
}

pub fn bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

pub fn date(timestamp: Option<i64>) -> String {
    timestamp
        .and_then(|value| chrono::DateTime::from_timestamp(value, 0))
        .map(|value| value.format("%d %b %Y · %H:%M UTC").to_string())
        .unwrap_or_else(|| "No source timestamp".into())
}
