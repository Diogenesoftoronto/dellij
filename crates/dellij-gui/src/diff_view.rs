//! Diff viewer — two-column line numbers, colored gutters, stats banner.
use gpui::*;

use crate::app::AppModel;
use crate::colors;

pub struct DiffView {
    model: Model<AppModel>,
}

impl DiffView {
    pub fn new(model: Model<AppModel>, cx: &mut ViewContext<Self>) -> Self {
        cx.subscribe(&model, |_, _, _, cx| cx.notify()).detach();
        Self { model }
    }
}

// ── diff parsing ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
enum DiffLine {
    FileHeader(String),
    HunkHeader(String),
    Added   { old: Option<u32>, new: u32, content: String },
    Removed { old: u32, new: Option<u32>, content: String },
    Context { old: u32, new: u32, content: String },
    NoNewline,
}

fn parse_diff(raw: &str) -> Vec<DiffLine> {
    let mut old_line: u32 = 0;
    let mut new_line: u32 = 0;
    let mut lines = Vec::new();

    for line in raw.lines() {
        if line.starts_with("diff --git")
            || line.starts_with("index ")
            || line.starts_with("new file")
            || line.starts_with("deleted file")
            || line.starts_with("Binary files")
        {
            lines.push(DiffLine::FileHeader(line.to_string()));
        } else if line.starts_with("--- ") || line.starts_with("+++ ") {
            lines.push(DiffLine::FileHeader(line.to_string()));
        } else if line.starts_with("@@") {
            // Parse @@ -old_start[,count] +new_start[,count] @@
            if let Some(nums) = parse_hunk_header(line) {
                old_line = nums.0;
                new_line = nums.1;
            }
            lines.push(DiffLine::HunkHeader(line.to_string()));
        } else if line.starts_with('+') {
            lines.push(DiffLine::Added {
                old: None,
                new: { let n = new_line; new_line += 1; n },
                content: line[1..].to_string(),
            });
        } else if line.starts_with('-') {
            lines.push(DiffLine::Removed {
                old: { let n = old_line; old_line += 1; n },
                new: None,
                content: line[1..].to_string(),
            });
        } else if line == "\\ No newline at end of file" {
            lines.push(DiffLine::NoNewline);
        } else {
            let content = if line.starts_with(' ') { &line[1..] } else { line };
            lines.push(DiffLine::Context {
                old: { let n = old_line; old_line += 1; n },
                new: { let n = new_line; new_line += 1; n },
                content: content.to_string(),
            });
        }
    }
    lines
}

fn parse_hunk_header(line: &str) -> Option<(u32, u32)> {
    // @@ -a[,b] +c[,d] @@
    let inner = line.trim_start_matches('@').trim_start_matches(' ');
    let parts: Vec<&str> = inner.split_whitespace().collect();
    if parts.len() < 2 { return None; }
    let old_start = parts[0].trim_start_matches('-').split(',').next()?.parse::<u32>().ok()?;
    let new_start = parts[1].trim_start_matches('+').split(',').next()?.parse::<u32>().ok()?;
    Some((old_start, new_start))
}

struct DiffStats { files: usize, added: usize, removed: usize }

fn compute_stats(lines: &[DiffLine]) -> DiffStats {
    let mut s = DiffStats { files: 0, added: 0, removed: 0 };
    for l in lines {
        match l {
            DiffLine::FileHeader(h) if h.starts_with("diff --git") => s.files += 1,
            DiffLine::Added { .. }   => s.added += 1,
            DiffLine::Removed { .. } => s.removed += 1,
            _ => {}
        }
    }
    s
}

// ── rendering ─────────────────────────────────────────────────────────────────

fn line_num_cell(n: Option<u32>) -> impl IntoElement {
    div()
        .w(px(36.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_end()
        .px(px(6.))
        .text_color(colors::OVERLAY0)
        .text_xs()
        .font_family("monospace")
        .opacity(0.6)
        .child(n.map(|v| v.to_string()).unwrap_or_default())
}

fn render_line(line: &DiffLine) -> impl IntoElement {
    match line {
        DiffLine::FileHeader(s) => div()
            .flex()
            .flex_row()
            .w_full()
            .py(px(5.))
            .px_3()
            .bg(colors::SURFACE1)
            .border_y_1()
            .border_color(rgba(0x45475a60))
            .child(
                div()
                    .text_color(colors::BLUE)
                    .text_xs()
                    .font_family("monospace")
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(s.clone()),
            ),

        DiffLine::HunkHeader(s) => div()
            .flex()
            .flex_row()
            .w_full()
            .py(px(3.))
            .bg(rgba(0x89b4fa0c))
            .border_y_1()
            .border_color(rgba(0x89b4fa20))
            .child(
                div()
                    .w(px(72.))
                    .flex_shrink_0(),
            )
            .child(
                div()
                    .flex_1()
                    .text_color(colors::SAPPHIRE)
                    .text_xs()
                    .font_family("monospace")
                    .opacity(0.8)
                    .child(s.clone()),
            ),

        DiffLine::Added { old, new, content } => div()
            .flex()
            .flex_row()
            .w_full()
            .bg(colors::ADD_BG)
            .border_l_2()
            .border_color(rgba(0xa6e3a160))
            .child(line_num_cell(*old))
            .child(line_num_cell(Some(*new)))
            .child(
                div()
                    .flex_shrink_0()
                    .w(px(16.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(colors::GREEN)
                    .text_xs()
                    .font_family("monospace")
                    .child("+"),
            )
            .child(
                div()
                    .flex_1()
                    .py_px()
                    .text_color(colors::TEXT)
                    .text_xs()
                    .font_family("monospace")
                    .child(content.clone()),
            ),

        DiffLine::Removed { old, new, content } => div()
            .flex()
            .flex_row()
            .w_full()
            .bg(colors::DEL_BG)
            .border_l_2()
            .border_color(rgba(0xf38ba860))
            .child(line_num_cell(Some(*old)))
            .child(line_num_cell(*new))
            .child(
                div()
                    .flex_shrink_0()
                    .w(px(16.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(colors::RED)
                    .text_xs()
                    .font_family("monospace")
                    .child("−"),
            )
            .child(
                div()
                    .flex_1()
                    .py_px()
                    .text_color(rgba(0xcdd6f4a0))
                    .text_xs()
                    .font_family("monospace")
                    .child(content.clone()),
            ),

        DiffLine::Context { old, new, content } => div()
            .flex()
            .flex_row()
            .w_full()
            .border_l_2()
            .border_color(Rgba::default())
            .child(line_num_cell(Some(*old)))
            .child(line_num_cell(Some(*new)))
            .child(div().flex_shrink_0().w(px(16.)))
            .child(
                div()
                    .flex_1()
                    .py_px()
                    .text_color(colors::OVERLAY1)
                    .text_xs()
                    .font_family("monospace")
                    .child(content.clone()),
            ),

        DiffLine::NoNewline => div()
            .flex()
            .flex_row()
            .w_full()
            .py_px()
            .child(div().w(px(72.)).flex_shrink_0())
            .child(
                div()
                    .flex_1()
                    .text_color(colors::OVERLAY0)
                    .text_xs()
                    .font_family("monospace")
                    .italic()
                    .child("\\ No newline at end of file"),
            ),
    }
}

fn stat_pill(count: usize, color: Rgba, prefix: &'static str) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_1()
        .px(px(8.))
        .py(px(3.))
        .rounded_full()
        .bg(rgba_alpha(color, 0.12))
        .child(
            div()
                .text_color(color)
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .child(format!("{prefix}{count}")),
        )
}

fn rgba_alpha(c: Rgba, a: f32) -> Rgba {
    Rgba { r: c.r, g: c.g, b: c.b, a }
}

impl Render for DiffView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let app      = self.model.read(cx);
        let selected = app.selected_slug.clone();

        let (title, diff_lines): (String, Vec<DiffLine>) = match &selected {
            None => ("No workspace selected".into(), vec![]),
            Some(slug) => {
                let ws = app.workspaces().iter().find(|w| &w.slug == slug);
                let title = ws.map(|w| format!(
                    "{} · {} → {}",
                    w.slug,
                    w.base_branch,
                    w.branch_name.trim_start_matches("dellij/")
                )).unwrap_or_default();
                let raw   = app.diff_cache.get(slug).map(String::as_str).unwrap_or("");
                let lines = if raw.is_empty() { vec![] } else { parse_diff(raw) };
                (title, lines)
            }
        };

        let stats    = compute_stats(&diff_lines);
        let is_empty = diff_lines.is_empty();

        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(colors::BASE)
            // ── header ───────────────────────────────────────────────────────
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_3()
                    .bg(colors::MANTLE)
                    .border_b_1()
                    .border_color(colors::SURFACE0)
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .text_color(colors::TEXT)
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .font_family("monospace")
                                    .child(title),
                            ),
                    )
                    .when(!is_empty, |d| {
                        d.child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .text_color(colors::OVERLAY0)
                                        .text_xs()
                                        .child(format!(
                                            "{} file{}",
                                            stats.files,
                                            if stats.files == 1 { "" } else { "s" }
                                        )),
                                )
                                .child(stat_pill(stats.added,   colors::GREEN, "+"))
                                .child(stat_pill(stats.removed, colors::RED,   "−"))
                        )
                    }),
            )
            // ── content ───────────────────────────────────────────────────────
            .child(
                div()
                    .flex_1()
                    .overflow_y_scroll()
                    .overflow_x_hidden()
                    .when(is_empty, |d| {
                        d.flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap_3()
                                    .p_8()
                                    .child(
                                        div()
                                            .text_color(colors::SURFACE2)
                                            .text_4xl()
                                            .child("✓"),
                                    )
                                    .child(
                                        div()
                                            .text_color(colors::OVERLAY0)
                                            .text_sm()
                                            .text_center()
                                            .child(if selected.is_none() {
                                                "Select a workspace in the sidebar."
                                            } else {
                                                "Clean — no diff against base branch."
                                            }),
                                    ),
                            )
                    })
                    .when(!is_empty, |d| {
                        d.children(diff_lines.iter().map(render_line))
                    }),
            )
    }
}
