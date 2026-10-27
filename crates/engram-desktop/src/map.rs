use std::collections::{HashMap, HashSet};

use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Sense, Shape, Stroke};
use engram_core::{Case, Category};

use crate::theme::{category_color, INK, LINE, MUTED, OCHRE};

pub enum Action {
    Select(String),
    Category(Category),
}

pub fn draw(
    ui: &mut egui::Ui,
    case: &Case,
    selected: Option<&str>,
    links_only: bool,
) -> Option<Action> {
    let height = (ui.available_height() - 12.0).clamp(300.0, 520.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let painter = ui.painter_at(rect);
    let center = rect.center() + vec2(0.0, -9.0);
    let scale = (rect.width() * 0.43).min(rect.height() * 0.44);
    let point = |x: f32, y: f32| center + vec2(x, y) * scale;

    // The two illustrated hemispheres are a category layout, not inferred anatomy.
    for side in [-1.0_f32, 1.0] {
        let points: Vec<Pos2> = (0..=96)
            .map(|i| {
                let t = i as f32 / 96.0 * std::f32::consts::TAU;
                let lobe = 1.0 + 0.045 * (t * 8.0).cos();
                point(side * 0.40 + t.cos() * 0.48 * lobe, t.sin() * 0.91 * lobe)
            })
            .collect();
        painter.add(Shape::convex_polygon(
            points,
            Color32::from_rgb(235, 228, 210),
            Stroke::new(1.3_f32, Color32::from_rgb(195, 185, 161)),
        ));
        for row in 0..4 {
            let y = -0.58 + row as f32 * 0.39;
            let curve: Vec<Pos2> = (0..35)
                .map(|i| {
                    let t = i as f32 / 34.0;
                    point(
                        side * (0.12 + t * 0.56),
                        y + (t * std::f32::consts::PI * 1.6 + row as f32).sin() * 0.11,
                    )
                })
                .collect();
            painter.add(Shape::line(
                curve,
                Stroke::new(1.0_f32, Color32::from_rgb(212, 201, 177)),
            ));
        }
    }

    let category_centers = [
        (Category::Media, (-0.48, -0.46)),
        (Category::Document, (-0.46, 0.12)),
        (Category::Browser, (-0.34, 0.62)),
        (Category::Secret, (0.47, -0.46)),
        (Category::Message, (0.46, 0.12)),
        (Category::Database, (0.34, 0.62)),
        (Category::Location, (0.0, -0.92)),
    ];
    let mut nodes = Vec::new();
    for (category, (x, y)) in category_centers {
        let matching: Vec<_> = case
            .findings
            .iter()
            .filter(|f| f.category == category)
            .filter(|f| !links_only || !f.related_ids.is_empty())
            .take(80)
            .collect();
        for (index, &finding) in matching.iter().enumerate() {
            let ring = (index as f32).sqrt();
            let theta = index as f32 * 2.399_963;
            let radius = (ring * 0.046).min(0.25);
            nodes.push((
                finding,
                point(x + theta.cos() * radius, y + theta.sin() * radius * 0.80),
            ));
        }
    }
    let positions: HashMap<_, _> = nodes
        .iter()
        .map(|(finding, position)| (finding.id.as_str(), *position))
        .collect();
    let mut edges = HashSet::new();
    for (finding, from) in &nodes {
        for related in &finding.related_ids {
            if let Some(to) = positions.get(related.as_str()) {
                let mut pair = [finding.id.as_str(), related.as_str()];
                pair.sort_unstable();
                if edges.insert((pair[0], pair[1])) {
                    let chosen =
                        selected == Some(finding.id.as_str()) || selected == Some(related.as_str());
                    painter.line_segment(
                        [*from, *to],
                        Stroke::new(
                            if chosen { 2.0_f32 } else { 0.8_f32 },
                            if chosen {
                                OCHRE
                            } else {
                                Color32::from_rgb(170, 164, 145)
                            },
                        ),
                    );
                }
            }
        }
    }
    let mut action = None;
    for (finding, position) in &nodes {
        let response = ui.interact(
            Rect::from_center_size(*position, vec2(18.0, 18.0)),
            ui.id().with(&finding.id),
            Sense::click(),
        );
        let chosen = selected == Some(finding.id.as_str());
        let radius = if chosen || response.hovered() {
            7.5
        } else {
            4.5
        };
        painter.circle_filled(*position, radius, category_color(finding.category));
        painter.circle_stroke(
            *position,
            radius,
            Stroke::new(
                if chosen { 2.0_f32 } else { 1.0_f32 },
                if chosen {
                    INK
                } else {
                    Color32::from_rgb(249, 246, 235)
                },
            ),
        );
        if response.clicked() {
            action = Some(Action::Select(finding.id.clone()));
        }
        response.on_hover_ui(|ui| {
            ui.strong(&finding.title);
            ui.small(&finding.path);
            ui.label(format!(
                "{} · {}",
                finding.category.label(),
                finding.confidence.label()
            ));
        });
    }

    for (category, (x, y)) in category_centers {
        let count = case
            .findings
            .iter()
            .filter(|f| f.category == category)
            .count();
        let label_position = point(
            x + if x < -0.1 {
                -0.39
            } else if x > 0.1 {
                0.39
            } else {
                0.0
            },
            y,
        );
        let align = if x < -0.1 {
            Align2::RIGHT_CENTER
        } else if x > 0.1 {
            Align2::LEFT_CENTER
        } else {
            Align2::CENTER_BOTTOM
        };
        let text = format!("{}  {:02}", category.label(), count);
        let label_rect = painter.text(
            label_position,
            align,
            text,
            FontId::monospace(11.0),
            if count == 0 { MUTED } else { INK },
        );
        if ui
            .interact(
                label_rect.expand(6.0),
                ui.id().with(category.label()),
                Sense::click(),
            )
            .clicked()
        {
            action = Some(Action::Category(category));
        }
        if count > 0 {
            let anchor = point(x, y);
            let line_end = pos2(
                label_position.x + if x < 0.0 { 9.0 } else { -9.0 },
                label_position.y,
            );
            if x.abs() > 0.1 {
                painter.line_segment([anchor, line_end], Stroke::new(0.6_f32, LINE));
            }
        }
    }
    painter.text(
        pos2(rect.center().x, rect.bottom() - 4.0),
        Align2::CENTER_BOTTOM,
        "CATEGORY LAYOUT  /  LINES = RECORDED RELATIONSHIPS",
        FontId::monospace(9.0),
        MUTED,
    );
    action
}
