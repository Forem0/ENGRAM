use std::{
    collections::HashSet,
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};

use eframe::egui::{self, Align, Color32, Layout, RichText, Sense, Stroke, Vec2};
use engram_core::{Case, CaseDiff, Category, Confidence, ModuleState, ScanOptions, SourceKind};
use image::ImageEncoder;

use crate::{
    map,
    theme::{self, *},
    Launch,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Map,
    Findings,
    Timeline,
    Connections,
    Compare,
    Modules,
}

impl View {
    const ALL: [(Self, &'static str, &'static str); 6] = [
        (Self::Map, "01", "Memory map"),
        (Self::Findings, "02", "Findings"),
        (Self::Timeline, "03", "Timeline"),
        (Self::Connections, "04", "Connections"),
        (Self::Compare, "05", "Compare cases"),
        (Self::Modules, "06", "Modules"),
    ];
    fn from_argument(value: Option<&str>) -> Self {
        match value {
            Some("findings") => Self::Findings,
            Some("timeline") => Self::Timeline,
            Some("connections") => Self::Connections,
            Some("compare") => Self::Compare,
            Some("modules") => Self::Modules,
            _ => Self::Map,
        }
    }
}

pub struct EngramApp {
    case: Case,
    view: View,
    selected: Option<String>,
    query: String,
    category: Option<Category>,
    linked_only: bool,
    worker: Option<mpsc::Receiver<Result<Case, String>>>,
    pending_source: Option<PathBuf>,
    scan_started: Option<Instant>,
    error: Option<String>,
    notice: Option<(String, Instant)>,
    baseline: Option<Case>,
    diff: Option<CaseDiff>,
    mascot: Option<egui::TextureHandle>,
    screenshot: Option<PathBuf>,
    screenshot_requested: bool,
    screenshot_frames: u32,
    exit_after_screenshot: bool,
    show_all_warnings: bool,
    relationship_count: usize,
}

impl EngramApp {
    pub fn new(cc: &eframe::CreationContext<'_>, launch: Launch) -> Self {
        theme::apply(&cc.egui_ctx);
        let mascot = image::load_from_memory(include_bytes!("../../../assets/mascot.png"))
            .ok()
            .map(|image| {
                let image = image.into_rgba8();
                cc.egui_ctx.load_texture(
                    "engram-archivist",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [image.width() as usize, image.height() as usize],
                        image.as_raw(),
                    ),
                    egui::TextureOptions::LINEAR,
                )
            });
        let mut app = Self {
            case: engram_core::demo_case(),
            view: View::from_argument(launch.view.as_deref()),
            selected: None,
            query: String::new(),
            category: None,
            linked_only: false,
            worker: None,
            pending_source: None,
            scan_started: None,
            error: None,
            notice: None,
            baseline: None,
            diff: None,
            mascot,
            screenshot: launch.screenshot,
            screenshot_requested: false,
            screenshot_frames: 0,
            exit_after_screenshot: launch.exit_after_screenshot,
            show_all_warnings: false,
            relationship_count: 0,
        };
        app.relationship_count = Self::count_relationships(&app.case);
        app.select_initial();
        if let Some(path) = launch.case {
            match engram_core::read_case(&path) {
                Ok(case) => app.replace_case(case),
                Err(error) => app.error = Some(error.to_string()),
            }
        }
        if let Some(path) = launch.source {
            app.start_scan(path, cc.egui_ctx.clone());
        }
        app
    }

    fn select_initial(&mut self) {
        self.selected = self
            .case
            .findings
            .iter()
            .find(|f| !f.related_ids.is_empty())
            .or_else(|| self.case.findings.first())
            .map(|f| f.id.clone());
    }

    fn replace_case(&mut self, case: Case) {
        self.relationship_count = Self::count_relationships(&case);
        self.case = case;
        self.category = None;
        self.query.clear();
        self.select_initial();
        self.diff = self
            .baseline
            .as_ref()
            .map(|old| engram_core::compare_cases(old, &self.case));
        self.error = None;
    }

    fn count_relationships(case: &Case) -> usize {
        let identifiers: HashSet<_> = case
            .findings
            .iter()
            .map(|finding| finding.id.as_str())
            .collect();
        let mut relationships = HashSet::new();
        for finding in &case.findings {
            for other in &finding.related_ids {
                if finding.id == *other || !identifiers.contains(other.as_str()) {
                    continue;
                }
                let pair = if finding.id < *other {
                    (finding.id.as_str(), other.as_str())
                } else {
                    (other.as_str(), finding.id.as_str())
                };
                relationships.insert(pair);
            }
        }
        relationships.len()
    }

    fn start_scan(&mut self, path: PathBuf, ctx: egui::Context) {
        if self.worker.is_some() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        self.pending_source = Some(path.clone());
        self.scan_started = Some(Instant::now());
        self.error = None;
        self.worker = Some(receiver);
        std::thread::spawn(move || {
            let result = engram_core::scan_path(&path, &ScanOptions::default())
                .map_err(|error| error.to_string());
            let _ = sender.send(result);
            ctx.request_repaint();
        });
    }

    fn pick_source(&mut self, ctx: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Choose your offline source folder or iPhone backup")
            .pick_folder()
        {
            self.start_scan(path, ctx.clone());
        }
    }

    fn open_case(&mut self, baseline: bool) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title(if baseline {
                "Choose an earlier ENGRAM case"
            } else {
                "Open an ENGRAM case"
            })
            .add_filter("ENGRAM case", &["json", "engram"])
            .pick_file()
        {
            match engram_core::read_case(&path) {
                Ok(case) if baseline => {
                    self.diff = Some(engram_core::compare_cases(&case, &self.case));
                    self.baseline = Some(case);
                    self.view = View::Compare;
                }
                Ok(case) => self.replace_case(case),
                Err(error) => self.error = Some(error.to_string()),
            }
        }
    }

    fn save_case(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Save masked ENGRAM case")
            .set_file_name("engram-case.json")
            .add_filter("JSON case", &["json"])
            .save_file()
        {
            // A case is a separate derived output. Never place it inside evidence.
            let inside_source = self.case.source_kind != SourceKind::Demo
                && std::fs::canonicalize(&self.case.source_root)
                    .ok()
                    .zip(
                        path.parent()
                            .and_then(|parent| std::fs::canonicalize(parent).ok()),
                    )
                    .is_some_and(|(source, parent)| parent.starts_with(source));
            if inside_source {
                self.error = Some(
                    "Choose an output outside the source folder. ENGRAM keeps evidence unchanged."
                        .into(),
                );
                return;
            }
            match engram_core::write_case(&self.case, &path) {
                Ok(()) => self.notice = Some(("Case saved. Candidate secret values remain masked; file paths and metadata are retained.".into(), Instant::now())),
                Err(error) => self.error = Some(error.to_string()),
            }
        }
    }

    fn poll_worker(&mut self) {
        let result = self.worker.as_ref().map(mpsc::Receiver::try_recv);
        match result {
            Some(Ok(result)) => {
                self.worker = None;
                self.pending_source = None;
                self.scan_started = None;
                match result {
                    Ok(case) => {
                        self.replace_case(case);
                        self.notice = Some(("Analysis finished. Review coverage and candidate findings before drawing conclusions.".into(), Instant::now()));
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            Some(Err(mpsc::TryRecvError::Disconnected)) => {
                self.worker = None;
                self.pending_source = None;
                self.scan_started = None;
                self.error = Some(
                    "The analysis worker stopped unexpectedly. The previous case is still open."
                        .into(),
                );
            }
            _ => (),
        }
    }

    fn header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("masthead")
            .exact_height(73.0)
            .frame(
                egui::Frame::new()
                    .fill(PAPER)
                    .inner_margin(egui::Margin::symmetric(22, 15)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        eyebrow(ui, "ENGRAM / THE MEMORY DESK");
                        ui.label(
                            RichText::new("A brain for your digital past.")
                                .size(17.0)
                                .strong(),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add_enabled(self.worker.is_none(), egui::Button::new("Save case"))
                            .clicked()
                        {
                            self.save_case();
                        }
                        if ui
                            .add_enabled(self.worker.is_none(), egui::Button::new("Open case"))
                            .clicked()
                        {
                            self.open_case(false);
                        }
                        let demo = self.case.source_kind == SourceKind::Demo;
                        pill(
                            ui,
                            if demo {
                                "SYNTHETIC DEMO"
                            } else {
                                "OFFLINE SNAPSHOT"
                            },
                            if demo {
                                OCHRE
                            } else {
                                category_color(Category::Document)
                            },
                        );
                    });
                });
            });
    }

    fn rail(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("navigation")
            .exact_width(205.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(RAIL)
                    .inner_margin(egui::Margin::symmetric(18, 22)),
            )
            .show(ctx, |ui| {
                if let Some(mascot) = &self.mascot {
                    ui.add(egui::Image::new(mascot).fit_to_exact_size(Vec2::splat(70.0)));
                }
                ui.add_space(5.0);
                ui.label(RichText::new("ENGRAM").size(27.0).strong().color(PAPER));
                ui.label(
                    RichText::new("DIGITAL RESIDUE / 0.1")
                        .monospace()
                        .size(9.0)
                        .color(RAIL_MUTED),
                );
                ui.add_space(26.0);
                for (view, number, label) in View::ALL {
                    let active = self.view == view;
                    let response = ui.add_sized(
                        [ui.available_width(), 39.0],
                        egui::Button::new(
                            RichText::new(format!("{number}    {label}"))
                                .color(if active { INK } else { PAPER })
                                .size(13.0),
                        )
                        .fill(if active {
                            Color32::from_rgb(222, 187, 110)
                        } else {
                            RAIL
                        })
                        .stroke(Stroke::NONE),
                    );
                    if response.clicked() {
                        self.view = view;
                    }
                }
                ui.add_space(26.0);
                ui.separator();
                ui.add_space(12.0);
                ui.label(
                    RichText::new("SOURCE")
                        .monospace()
                        .size(10.0)
                        .color(RAIL_MUTED),
                );
                ui.label(
                    RichText::new(self.case.source_kind.label())
                        .size(13.0)
                        .color(PAPER),
                );
                ui.add(
                    egui::Label::new(RichText::new(&self.case.title).size(11.0).color(RAIL_MUTED))
                        .truncate(),
                )
                .on_hover_text(format!(
                    "{}\nCase completed: {}",
                    self.case.source_root,
                    date(Some(self.case.completed_at))
                ));
                ui.label(
                    RichText::new(if self.case.source_kind == SourceKind::Demo {
                        "Generated test data.\nNo personal files scanned."
                    } else {
                        "Explicitly selected offline data.\nSource files stay unchanged."
                    })
                    .size(11.0)
                    .color(RAIL_MUTED),
                );
                ui.add_space(14.0);
                if ui
                    .add_enabled(
                        self.worker.is_none(),
                        egui::Button::new(RichText::new("+  Choose source").color(INK)).fill(OCHRE),
                    )
                    .clicked()
                {
                    self.pick_source(ctx);
                }
                if ui
                    .add_enabled(
                        self.worker.is_none(),
                        egui::Button::new(RichText::new("Load demonstration").color(PAPER))
                            .fill(RAIL)
                            .stroke(Stroke::new(1.0_f32, MUTED)),
                    )
                    .clicked()
                {
                    self.replace_case(engram_core::demo_case());
                    self.view = View::Map;
                }
                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    ui.label(
                        RichText::new("DEVELOPER PREVIEW")
                            .monospace()
                            .size(9.0)
                            .color(RAIL_MUTED),
                    );
                    ui.label(
                        RichText::new("Offline by design.\nEvidence before conclusions.")
                            .size(11.0)
                            .color(RAIL_MUTED),
                    );
                });
            });
    }

    fn inspector(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("inspector").default_width(280.0).width_range(240.0..=380.0)
            .frame(egui::Frame::new().fill(CARD).stroke(Stroke::new(1.0_f32, LINE)).inner_margin(18))
            .show(ctx, |ui| {
                eyebrow(ui, "EVIDENCE INSPECTOR"); ui.add_space(12.0);
                let finding = self.selected.as_ref().and_then(|id| self.case.findings.iter().find(|finding| &finding.id == id)).cloned();
                let Some(finding) = finding else { ui.heading("Inspect a trace"); ui.label("Select a point on the memory map or a finding to read its recorded evidence."); return; };
                egui::ScrollArea::vertical().id_salt("inspector_scroll").show(ui, |ui| {
                    pill(ui, finding.category.label(), category_color(finding.category));
                    ui.add_space(10.0); ui.label(RichText::new(&finding.title).size(19.0).strong());
                    ui.add_space(6.0); ui.label(&finding.summary);
                    ui.add_space(14.0); ui.separator(); ui.add_space(10.0);
                    eyebrow(ui, "OBSERVATION");
                    ui.label(RichText::new(finding.confidence.label()).strong());
                    if finding.confidence == Confidence::Candidate { ui.label(RichText::new("A candidate needs review. It does not establish deletion or compromise.").size(11.0).color(MUTED)); }
                    if finding.category == Category::Secret { pill(ui, "CANDIDATE VALUE MASKED", RED); }
                    else if finding.sensitive { pill(ui, "SENSITIVE METADATA", RED); }
                    ui.add_space(12.0); eyebrow(ui, "SOURCE PATH");
                    ui.add(egui::Label::new(RichText::new(&finding.path).monospace().size(11.0)).wrap());
                    if ui.small_button("Copy path").clicked() { ctx.copy_text(finding.path.clone()); }
                    eyebrow(ui, "LOCATOR"); ui.label(RichText::new(&finding.locator).monospace().size(11.0));
                    eyebrow(ui, "RULE"); ui.label(RichText::new(&finding.rule_id).monospace().size(11.0));
                    eyebrow(ui, "SOURCE TIMESTAMP"); ui.label(RichText::new(date(finding.observed_at)).size(12.0));
                    ui.label(RichText::new("Timestamp meaning depends on the source. This is not a deletion time.").size(10.0).color(MUTED));
                    ui.add_space(8.0); eyebrow(ui, "RECORDED FACTS");
                    for evidence in &finding.evidence {
                        ui.label(RichText::new(&evidence.label).size(10.0).color(MUTED));
                        ui.add(egui::Label::new(RichText::new(&evidence.value).size(12.0)).wrap());
                    }
                    if let Some(hash) = &finding.sha256 {
                        ui.add_space(8.0); eyebrow(ui, "FILE SHA-256");
                        ui.add(egui::Label::new(RichText::new(hash).monospace().size(10.0)).wrap());
                        if ui.small_button("Copy digest").clicked() { ctx.copy_text(hash.clone()); }
                    }
                    ui.add_space(12.0); eyebrow(ui, &format!("RELATED TRACES / {}", finding.related_ids.len()));
                    for related in &finding.related_ids {
                        if let Some(other) = self.case.findings.iter().find(|f| &f.id == related) {
                            if ui.link(&other.title).clicked() { self.selected = Some(related.clone()); }
                        }
                    }
                    if finding.related_ids.is_empty() { ui.label(RichText::new("No relationship recorded.").size(11.0).color(MUTED)); }
                });
            });
    }

    fn statistics(&self, ui: &mut egui::Ui) {
        let stats = [
            (
                "TRACES RECORDED",
                self.case.findings.len().to_string(),
                "Across available modules",
            ),
            (
                "FILES EXAMINED",
                self.case.stats.files_examined.to_string(),
                "Within configured limits",
            ),
            (
                "RELATIONSHIPS",
                self.relationship_count.to_string(),
                "Explicit links in this case",
            ),
            (
                "FILES SKIPPED",
                self.case.stats.skipped_files.to_string(),
                "See coverage and warnings",
            ),
        ];
        ui.columns(4, |columns| {
            for (column, (label, value, description)) in columns.iter_mut().zip(stats) {
                egui::Frame::new()
                    .fill(CARD)
                    .stroke(Stroke::new(1.0_f32, LINE))
                    .inner_margin(14)
                    .show(column, |ui| {
                        ui.set_min_width(ui.available_width());
                        eyebrow(ui, label);
                        ui.label(RichText::new(value).size(28.0).strong());
                        ui.label(RichText::new(description).size(10.0).color(MUTED));
                    });
            }
        });
    }

    fn page_heading(ui: &mut egui::Ui, kicker: &str, title: &str, description: &str) {
        eyebrow(ui, kicker);
        ui.label(RichText::new(title).size(30.0).strong());
        ui.label(RichText::new(description).size(12.0).color(MUTED));
        ui.add_space(16.0);
    }

    fn memory_map(&mut self, ui: &mut egui::Ui, connections: bool) {
        Self::page_heading(ui, if connections { "RELATIONSHIP DESK" } else { "YOUR DIGITAL MEMORY" },
            if connections { "Follow the connections." } else { "What remains, connected." },
            "Every point is a recorded finding. Every line comes from a relationship stored in the case.");
        self.statistics(ui);
        ui.add_space(16.0);
        egui::Frame::new()
            .fill(CARD)
            .stroke(Stroke::new(1.0_f32, LINE))
            .inner_margin(16)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    eyebrow(
                        ui,
                        if connections {
                            "CONNECTED TRACES"
                        } else {
                            "MEMORY MAP"
                        },
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.checkbox(&mut self.linked_only, "Linked only");
                    });
                });
                let action = map::draw(
                    ui,
                    &self.case,
                    self.selected.as_deref(),
                    connections || self.linked_only,
                );
                match action {
                    Some(map::Action::Select(id)) => self.selected = Some(id),
                    Some(map::Action::Category(category)) => {
                        self.category = Some(category);
                        self.view = View::Findings;
                    }
                    None => (),
                }
            });
        ui.add_space(12.0);
        if connections && self.case.findings.iter().all(|f| f.related_ids.is_empty()) {
            ui.label("No relationships were recorded for this source. Unrelated findings remain available in Findings.");
        } else {
            ui.label(RichText::new("Select a node to inspect evidence. Category totals include all findings; map displays up to 80 per category.").size(11.0).color(MUTED));
        }
    }

    fn filtered_indices(&self) -> Vec<usize> {
        let query = self.query.to_lowercase();
        self.case
            .findings
            .iter()
            .enumerate()
            .filter(|(_, finding)| {
                self.category
                    .is_none_or(|category| finding.category == category)
                    && (!self.linked_only || !finding.related_ids.is_empty())
                    && (query.is_empty()
                        || format!(
                            "{} {} {} {}",
                            finding.title, finding.path, finding.rule_id, finding.origin
                        )
                        .to_lowercase()
                        .contains(&query))
            })
            .map(|(index, _)| index)
            .collect()
    }

    fn finding_filters(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text("Search title, path or rule…")
                    .desired_width((ui.available_width() - 145.0).max(140.0)),
            );
            ui.checkbox(&mut self.linked_only, "Linked only");
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .selectable_label(self.category.is_none(), "All")
                .clicked()
            {
                self.category = None;
            }
            for category in Category::ALL {
                if ui
                    .selectable_label(self.category == Some(category), category.label())
                    .clicked()
                {
                    self.category = Some(category);
                }
            }
        });
        ui.add_space(8.0);
    }

    fn findings(&mut self, ui: &mut egui::Ui) {
        Self::page_heading(
            ui,
            "EVIDENCE INDEX",
            "Inspect every trace.",
            "Search the actual case. Secret candidates are masked before entering this view.",
        );
        self.finding_filters(ui);
        let indices = self.filtered_indices();
        eyebrow(
            ui,
            &format!(
                "{} FINDINGS / {} TOTAL",
                indices.len(),
                self.case.findings.len()
            ),
        );
        ui.add_space(8.0);
        if indices.is_empty() {
            Self::empty_state(
                ui,
                "No matching findings",
                "Change the filter or choose another supported offline source.",
            );
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt("finding_rows")
            .show_rows(ui, 72.0, indices.len(), |ui, range| {
                for &finding_index in &indices[range] {
                    let finding = self.case.findings[finding_index].clone();
                    let active = self.selected.as_deref() == Some(finding.id.as_str());
                    let response = egui::Frame::new()
                        .fill(if active {
                            Color32::from_rgb(235, 222, 193)
                        } else {
                            CARD
                        })
                        .stroke(Stroke::new(1.0_f32, if active { OCHRE } else { LINE }))
                        .inner_margin(11)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&finding.title).strong().size(13.0));
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(finding.confidence.label())
                                            .size(10.0)
                                            .color(MUTED),
                                    );
                                    ui.label(
                                        RichText::new(finding.category.label())
                                            .size(10.0)
                                            .color(category_color(finding.category)),
                                    );
                                });
                            });
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&finding.path)
                                        .monospace()
                                        .size(10.0)
                                        .color(MUTED),
                                )
                                .truncate(),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "{}  /  {} recorded relationship(s)",
                                    finding.rule_id,
                                    finding.related_ids.len()
                                ))
                                .size(10.0)
                                .color(MUTED),
                            );
                        })
                        .response;
                    if ui
                        .interact(response.rect, ui.id().with(&finding.id), Sense::click())
                        .clicked()
                    {
                        self.selected = Some(finding.id);
                    }
                }
            });
    }

    fn timeline(&mut self, ui: &mut egui::Ui) {
        Self::page_heading(ui, "SOURCE CHRONOLOGY", "Put the traces in time.", "Source timestamps, sorted newest first. They do not establish when an item was deleted.");
        self.finding_filters(ui);
        let mut indices = self.filtered_indices();
        indices.sort_by_key(|&index| std::cmp::Reverse(self.case.findings[index].observed_at));
        let undated = indices
            .iter()
            .filter(|&&index| self.case.findings[index].observed_at.is_none())
            .count();
        eyebrow(
            ui,
            &format!(
                "{} DATED / {} WITHOUT A SOURCE TIMESTAMP",
                indices.len() - undated,
                undated
            ),
        );
        egui::ScrollArea::vertical()
            .id_salt("timeline_rows")
            .show_rows(ui, 78.0, indices.len(), |ui, range| {
                for &index in &indices[range] {
                    let finding = self.case.findings[index].clone();
                    egui::Frame::new()
                        .inner_margin(egui::Margin::symmetric(0, 9))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("●").color(category_color(finding.category)),
                                );
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(date(finding.observed_at))
                                            .monospace()
                                            .size(10.0)
                                            .color(MUTED),
                                    );
                                    if ui.link(&finding.title).clicked() {
                                        self.selected = Some(finding.id.clone());
                                    }
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(&finding.path).size(11.0).color(MUTED),
                                        )
                                        .truncate(),
                                    );
                                });
                            });
                        });
                    ui.separator();
                }
            });
    }

    fn compare(&mut self, ui: &mut egui::Ui) {
        Self::page_heading(ui, "BEFORE / AFTER", "Check what changed.", "Compare recorded observations between two compatible cases. Missing from a case does not prove erased from a device.");
        if ui.button("Choose earlier case…").clicked() {
            self.open_case(true);
        }
        ui.add_space(12.0);
        let (Some(baseline), Some(diff)) = (&self.baseline, &self.diff) else {
            Self::empty_state(ui, "Bring a second point in time.", "Open an earlier ENGRAM case to compare added, removed, changed and unchanged observations.");
            return;
        };
        egui::Frame::new()
            .fill(CARD)
            .stroke(Stroke::new(1.0_f32, LINE))
            .inner_margin(14)
            .show(ui, |ui| {
                ui.label(
                    RichText::new(format!("{}  →  {}", baseline.title, self.case.title)).strong(),
                );
                ui.label(
                    RichText::new(format!(
                        "{}  →  {}",
                        date(Some(baseline.completed_at)),
                        date(Some(self.case.completed_at))
                    ))
                    .size(11.0)
                    .color(MUTED),
                );
            });
        ui.add_space(15.0);
        if baseline.source_root != self.case.source_root {
            ui.colored_label(
                RED,
                "Different source paths: differences may reflect source coverage, not deletion.",
            );
        }
        let groups = [
            ("ADDED", &diff.added, category_color(Category::Document)),
            ("REMOVED FROM CASE", &diff.removed, RED),
            ("CHANGED", &diff.changed, OCHRE),
            ("UNCHANGED", &diff.unchanged, MUTED),
        ];
        egui::ScrollArea::vertical().id_salt("diff_rows").show(ui, |ui| {
            for (title, ids, color) in groups {
                ui.add_space(12.0); ui.label(RichText::new(format!("{title} / {}", ids.len())).monospace().size(11.0).color(color));
                for id in ids.iter().take(250) {
                    if let Some(finding) = self.case.findings.iter().chain(baseline.findings.iter()).find(|f| &f.id == id) {
                        ui.label(RichText::new(format!("{} · {}", finding.title, finding.path)).size(12.0));
                    }
                }
                if ids.len() > 250 { ui.small("First 250 entries shown in this group. Full differences are available through the CLI."); }
            }
        });
    }

    fn modules(&self, ui: &mut egui::Ui) {
        Self::page_heading(
            ui,
            "CAPABILITY REGISTER",
            "Know the limits.",
            "Modules report their actual state. Planned recovery features produce no findings.",
        );
        egui::ScrollArea::vertical().id_salt("module_rows").show(ui, |ui| {
            for module in &self.case.modules {
                let color = match module.state { ModuleState::Ready => category_color(Category::Document), ModuleState::Experimental => OCHRE, ModuleState::Planned | ModuleState::Unavailable => MUTED };
                egui::Frame::new().fill(CARD).stroke(Stroke::new(1.0_f32, LINE)).inner_margin(16).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| { ui.label(RichText::new(&module.name).size(16.0).strong()); ui.with_layout(Layout::right_to_left(Align::Center), |ui| pill(ui, module.state.label(), color)); });
                    ui.label(RichText::new(&module.detail).size(12.0).color(MUTED));
                }); ui.add_space(4.0);
            }
            ui.add_space(16.0); eyebrow(ui, "ANALYSIS RECEIPT");
            ui.label(format!("Examined {} · {} · {} skipped · {} ms", self.case.stats.files_examined, bytes(self.case.stats.bytes_examined), self.case.stats.skipped_files, self.case.stats.elapsed_ms));
            ui.label(RichText::new("Candidate values are not tested against online services. Keychain extraction, arbitrary app access and recovery from erased storage are not implied by this preview.").size(12.0).color(MUTED));
        });
    }

    fn empty_state(ui: &mut egui::Ui, title: &str, description: &str) {
        ui.add_space(65.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(title).size(23.0).strong());
            ui.add_space(9.0);
            ui.label(RichText::new(description).color(MUTED));
        });
    }

    fn footer(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(PAPER)
                    .inner_margin(egui::Margin::symmetric(20, 9)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if self.case.source_kind == SourceKind::Demo {
                            "●  SYNTHETIC DATA"
                        } else {
                            "●  SOURCE READS ONLY"
                        })
                        .monospace()
                        .size(9.0)
                        .color(OCHRE),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{} findings · {}",
                            self.case.findings.len(),
                            bytes(self.case.stats.bytes_examined)
                        ))
                        .monospace()
                        .size(9.0)
                        .color(MUTED),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new("ENGRAM  /  DEVELOPER PREVIEW")
                                .monospace()
                                .size(9.0)
                                .color(MUTED),
                        );
                    });
                });
            });
    }

    fn screenshot_handling(&mut self, ctx: &egui::Context) {
        self.screenshot_frames += 1;
        if self.screenshot.is_some()
            && !self.screenshot_requested
            && self.screenshot_frames >= 5
            && self.worker.is_none()
        {
            self.screenshot_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let screenshots: Vec<_> = ctx.input(|input| {
            input
                .events
                .iter()
                .filter_map(|event| {
                    if let egui::Event::Screenshot { image, .. } = event {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
                .collect()
        });
        if let Some(image) = screenshots.first() {
            if let Some(path) = self.screenshot.take() {
                let resolved_parent = path
                    .parent()
                    .filter(|parent| !parent.as_os_str().is_empty())
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .canonicalize();
                let inside_source = self.case.source_kind != SourceKind::Demo
                    && std::fs::canonicalize(&self.case.source_root)
                        .ok()
                        .zip(resolved_parent.ok())
                        .is_some_and(|(source, parent)| parent.starts_with(source));
                if inside_source {
                    self.error = Some(
                        "Screenshot output must be outside the selected source folder.".into(),
                    );
                    if self.exit_after_screenshot {
                        eprintln!(
                            "ENGRAM: screenshot output must be outside the selected source folder."
                        );
                        std::process::exit(1);
                    }
                    return;
                }
                let pixels: Vec<u8> = image
                    .pixels
                    .iter()
                    .flat_map(|pixel| pixel.to_array())
                    .collect();
                let saved = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
                    .map_err(image::ImageError::IoError)
                    .and_then(|file| {
                        image::codecs::png::PngEncoder::new(file).write_image(
                            &pixels,
                            image.size[0] as u32,
                            image.size[1] as u32,
                            image::ExtendedColorType::Rgba8,
                        )
                    });
                match saved {
                    Ok(()) => {
                        if self.exit_after_screenshot {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                    Err(error) => {
                        self.error = Some(format!("Cannot save new screenshot: {error}"));
                        if self.exit_after_screenshot {
                            eprintln!("ENGRAM: cannot save new screenshot: {error}");
                            std::process::exit(1);
                        }
                    }
                }
            }
        }
        if self.screenshot.is_some() {
            ctx.request_repaint_after(Duration::from_millis(80));
        }
    }
}

impl eframe::App for EngramApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_worker();
        self.header(ctx);
        self.footer(ctx);
        self.rail(ctx);
        self.inspector(ctx);
        egui::CentralPanel::default().frame(egui::Frame::new().fill(PAPER).inner_margin(24)).show(ctx, |ui| {
            if let Some(error) = self.error.clone() {
                egui::Frame::new().fill(Color32::from_rgb(245, 224, 214)).inner_margin(12).show(ui, |ui| {
                    ui.horizontal(|ui| { ui.colored_label(RED, "Analysis needs attention"); if ui.small_button("Dismiss").clicked() { self.error = None; } });
                    ui.label(error);
                }); ui.add_space(10.0);
            }
            if let Some((notice, time)) = &self.notice {
                if time.elapsed() < Duration::from_secs(9) { ui.label(RichText::new(notice).size(11.0).color(category_color(Category::Document))); ui.add_space(6.0); }
            }
            if self.worker.is_some() {
                ui.horizontal(|ui| { ui.spinner(); ui.label(RichText::new("Reading selected offline source…").strong()); });
                ui.label(RichText::new(self.pending_source.as_ref().map(|p| p.display().to_string()).unwrap_or_default()).monospace().size(11.0));
                ui.label(RichText::new(format!("{} s elapsed · bounded analysis · the previous case remains visible below", self.scan_started.map(|time| time.elapsed().as_secs()).unwrap_or(0))).size(11.0).color(MUTED));
                ui.separator(); ctx.request_repaint_after(Duration::from_millis(100));
            }
            match self.view {
                View::Map => self.memory_map(ui, false), View::Findings => self.findings(ui),
                View::Timeline => self.timeline(ui), View::Connections => self.memory_map(ui, true),
                View::Compare => self.compare(ui), View::Modules => self.modules(ui),
            }
        });
        if !self.case.warnings.is_empty() {
            egui::Window::new("Coverage notes")
                .open(&mut self.show_all_warnings)
                .default_width(560.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .show(ui, |ui| {
                            for warning in &self.case.warnings {
                                ui.label(format!("• {warning}"));
                            }
                        });
                });
            egui::Area::new(egui::Id::new("coverage_badge"))
                .anchor(egui::Align2::LEFT_BOTTOM, [225.0, -42.0])
                .show(ctx, |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(format!(
                                    "{} coverage note(s)  ↗",
                                    self.case.warnings.len()
                                ))
                                .size(11.0),
                            )
                            .fill(Color32::from_rgb(236, 224, 194)),
                        )
                        .clicked()
                    {
                        self.show_all_warnings = !self.show_all_warnings;
                    }
                });
        }
        self.screenshot_handling(ctx);
    }
}
