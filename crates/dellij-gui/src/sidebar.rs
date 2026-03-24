//! Left sidebar — workspace list, status-accented cards, IDE deep-links.
use gpui::*;

use dellij_core::{ahead_behind, Workspace};

use crate::app::AppModel;
use crate::browser::launch_browser_window;
use crate::colors;

pub struct SidebarView {
    model: Model<AppModel>,
}

impl SidebarView {
    pub fn new(model: Model<AppModel>, cx: &mut ViewContext<Self>) -> Self {
        cx.subscribe(&model, |_, _, _, cx| cx.notify()).detach();
        Self { model }
    }

    fn render_section_header(label: &'static str, count: usize) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .px_4()
            .pt_4()
            .pb_1()
            .child(
                div()
                    .text_color(colors::OVERLAY0)
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .tracking_widest()
                    .child(label.to_uppercase()),
            )
            .child(
                div()
                    .px(px(6.))
                    .py_px()
                    .rounded_full()
                    .bg(colors::SURFACE0)
                    .text_color(colors::OVERLAY1)
                    .text_xs()
                    .child(count.to_string()),
            )
    }

    fn render_workspace(
        &self,
        ws: &Workspace,
        selected: bool,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let slug        = ws.slug.clone();
        let model       = self.model.clone();
        let model_edit  = self.model.clone();
        let project_root = self.model.read(cx).project_root.clone();

        let status_color = colors::status_color(ws.status);
        let status_icon  = colors::status_icon(ws.status);
        let agent_color  = colors::agent_color(&ws.agent);
        let agent_label  = colors::agent_short(&ws.agent);

        let ab = ahead_behind(&project_root, &ws.branch_name, &ws.base_branch)
            .map(|a| a.to_string())
            .unwrap_or_default();
        let branch_display = ws.branch_name.trim_start_matches("dellij/").to_string();

        let ports: Vec<u16>   = ws.ports.clone();
        let urls: Vec<String> = ws.urls.clone();
        let pr_num            = ws.pr_number;

        let launch_url = urls.first().cloned()
            .or_else(|| ports.first().map(|p| format!("http://localhost:{p}")));

        // ── container ────────────────────────────────────────────────────────
        div()
            .flex()
            .flex_row()
            .mx_2()
            .my(px(1.))
            .rounded_lg()
            .overflow_hidden()
            .cursor_pointer()
            .bg(if selected { colors::SELECT_BG } else { Rgba::default() })
            .hover(|s| s.bg(rgba(0x31324480)))
            .shadow(if selected {
                smallvec![BoxShadow {
                    color: rgba(0x00000028),
                    offset: point(px(0.), px(2.)),
                    blur_radius: px(8.),
                    spread_radius: px(0.),
                }]
            } else {
                smallvec![]
            })
            .on_click(cx.listener(move |_, _, cx| {
                model.update(cx, |m, cx| m.select(slug.clone(), cx));
            }))
            // Left accent bar (status colour, full height)
            .child(
                div()
                    .w(px(3.))
                    .flex_shrink_0()
                    .bg(if selected { status_color } else { rgba(0x00000000) })
                    .transition_color(),
            )
            // ── card content ─────────────────────────────────────────────────
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .px_3()
                    .py(px(10.))
                    .gap_1()
                    // ── row 1: slug + agent pill + status icon ────────────────
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap(px(6.))
                                    // Agent pill
                                    .child(
                                        div()
                                            .px(px(5.))
                                            .py_px()
                                            .rounded(px(4.))
                                            .bg(rgba_from(agent_color, 0.18))
                                            .border_1()
                                            .border_color(rgba_from(agent_color, 0.35))
                                            .text_color(agent_color)
                                            .text_xs()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .font_family("monospace")
                                            .child(agent_label),
                                    )
                                    // Slug
                                    .child(
                                        div()
                                            .text_color(if selected { colors::TEXT } else { colors::SUBTEXT1 })
                                            .text_sm()
                                            .font_weight(if selected { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .child(ws.slug.clone()),
                                    ),
                            )
                            // Status dot (right)
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .text_color(status_color)
                                    .text_xs()
                                    .child(status_icon),
                            ),
                    )
                    // ── row 2: branch + ahead/behind ──────────────────────────
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_color(colors::OVERLAY0)
                                    .text_xs()
                                    .font_family("monospace")
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .child(branch_display),
                            )
                            .when(!ab.is_empty(), |d| {
                                d.child(
                                    div()
                                        .flex_shrink_0()
                                        .px(px(5.))
                                        .py_px()
                                        .rounded_full()
                                        .bg(rgba(0x89b4fa18))
                                        .text_color(colors::BLUE)
                                        .text_xs()
                                        .font_family("monospace")
                                        .child(ab),
                                )
                            }),
                    )
                    // ── row 3: ports + PR (only when present) ─────────────────
                    .when(!ports.is_empty() || pr_num.is_some(), |d| {
                        d.child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(6.))
                                .children(ports.iter().map(|p| {
                                    div()
                                        .px(px(5.))
                                        .py_px()
                                        .rounded(px(4.))
                                        .bg(rgba(0x89dceb18))
                                        .text_color(colors::CYAN)
                                        .text_xs()
                                        .font_family("monospace")
                                        .child(format!(":{p}"))
                                }))
                                .when_some(pr_num, |d, n| {
                                    d.child(
                                        div()
                                            .px(px(5.))
                                            .py_px()
                                            .rounded(px(4.))
                                            .bg(rgba(0xfab38718))
                                            .text_color(colors::PEACH)
                                            .text_xs()
                                            .child(format!("PR #{n}")),
                                    )
                                })
                        )
                    })
                    // ── row 4: action buttons (selected only) ─────────────────
                    .when(selected, |d| {
                        d.child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(4.))
                                .pt(px(4.))
                                .child(editor_btn("VS Code", {
                                    let slug = ws.slug.clone();
                                    let m = model_edit.clone();
                                    move |cx| { let _ = m.read(cx).open_in_editor(&slug, "code"); }
                                }, cx))
                                .child(editor_btn("Cursor", {
                                    let slug = ws.slug.clone();
                                    let m = model_edit.clone();
                                    move |cx| { let _ = m.read(cx).open_in_editor(&slug, "cursor"); }
                                }, cx))
                                .child(editor_btn("Zed", {
                                    let slug = ws.slug.clone();
                                    let m = model_edit.clone();
                                    move |cx| { let _ = m.read(cx).open_in_editor(&slug, "zed"); }
                                }, cx))
                                .when_some(launch_url, |d, url| {
                                    d.child(editor_btn("↗ Web", move |_cx| {
                                        launch_browser_window(&url);
                                    }, cx))
                                }),
                        )
                    }),
            )
    }

    fn render_footer(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let attn = self.model.read(cx).attention_count();
        if attn == 0 {
            return div()
                .h(px(32.))
                .flex()
                .items_center()
                .justify_center()
                .border_t_1()
                .border_color(colors::SURFACE0)
                .child(
                    div()
                        .text_xs()
                        .text_color(colors::OVERLAY0)
                        .child("● all clear"),
                );
        }
        div()
            .h(px(36.))
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .gap_2()
            .border_t_1()
            .border_color(colors::SURFACE0)
            .bg(rgba(0xf38ba808))
            .child(
                div()
                    .text_xs()
                    .text_color(colors::RED)
                    .child(format!("⚠ {attn} need{} attention", if attn == 1 { "s" } else { "" })),
            )
    }
}

// ── tiny helper: editor/action button ────────────────────────────────────────

fn editor_btn(
    label: &'static str,
    handler: impl Fn(&mut WindowContext) + 'static,
    _cx: &mut ViewContext<SidebarView>,
) -> impl IntoElement {
    div()
        .px(px(7.))
        .py(px(3.))
        .rounded(px(5.))
        .bg(colors::SURFACE1)
        .text_color(colors::SUBTEXT0)
        .text_xs()
        .cursor_pointer()
        .hover(|s| s.bg(colors::SURFACE2).text_color(colors::TEXT))
        .active(|s| s.bg(colors::OVERLAY0))
        .on_click(move |_, cx| handler(cx))
        .child(label)
}

/// Create a color with a custom alpha from an existing Rgba.
fn rgba_from(c: Rgba, alpha: f32) -> Rgba {
    Rgba { r: c.r, g: c.g, b: c.b, a: alpha }
}

impl Render for SidebarView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let selected   = self.model.read(cx).selected_slug.clone();
        let workspaces: Vec<Workspace> = self.model.read(cx).workspaces().to_vec();
        let count      = workspaces.len();

        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            // Section header
            .child(Self::render_section_header("Workspaces", count))
            // List
            .child(
                div()
                    .flex_1()
                    .overflow_y_scroll()
                    .py_1()
                    .flex()
                    .flex_col()
                    .children(workspaces.iter().map(|ws| {
                        let is_sel = selected.as_deref() == Some(&ws.slug);
                        self.render_workspace(ws, is_sel, cx)
                    }))
                    .when(workspaces.is_empty(), |d| {
                        d.flex()
                            .flex_1()
                            .items_center()
                            .justify_center()
                            .p_6()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_color(colors::SURFACE2)
                                            .text_2xl()
                                            .child("◇"),
                                    )
                                    .child(
                                        div()
                                            .text_color(colors::OVERLAY0)
                                            .text_xs()
                                            .text_center()
                                            .child("No workspaces yet.\nRun `dellij new` to create one."),
                                    ),
                            )
                    }),
            )
            // Footer
            .child(self.render_footer(cx))
    }
}
