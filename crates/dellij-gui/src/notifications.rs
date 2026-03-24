//! Notification panel — attention cards with status accents.
use gpui::*;

use dellij_core::Workspace;

use crate::app::AppModel;
use crate::colors;

pub struct NotificationView {
    model: Model<AppModel>,
}

impl NotificationView {
    pub fn new(model: Model<AppModel>, cx: &mut ViewContext<Self>) -> Self {
        cx.subscribe(&model, |_, _, _, cx| cx.notify()).detach();
        Self { model }
    }

    fn render_card(ws: &Workspace) -> impl IntoElement {
        let color       = colors::status_color(ws.status);
        let status_bg   = colors::status_bg(ws.status);
        let icon        = colors::status_icon(ws.status);
        let agent_color = colors::agent_color(&ws.agent);
        let agent_label = colors::agent_short(&ws.agent);
        let latest_note = ws.notes.last().cloned().unwrap_or_default();
        let branch      = ws.branch_name.trim_start_matches("dellij/").to_string();

        div()
            .flex()
            .flex_col()
            .rounded_xl()
            .overflow_hidden()
            .bg(colors::MANTLE)
            .border_1()
            .border_color(rgba_alpha(color, 0.25))
            .shadow(smallvec![
                BoxShadow {
                    color: rgba(0x00000020),
                    offset: point(px(0.), px(2.)),
                    blur_radius: px(8.),
                    spread_radius: px(0.),
                },
                BoxShadow {
                    color: rgba_alpha(color, 0.06),
                    offset: point(px(0.), px(0.)),
                    blur_radius: px(16.),
                    spread_radius: px(2.),
                },
            ])
            // Tinted top bar
            .child(
                div()
                    .h(px(3.))
                    .w_full()
                    .bg(color),
            )
            // Card body
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_4()
                    // ── title row ─────────────────────────────────────────────
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap_2()
                                    // Status icon
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .w(px(24.))
                                            .h(px(24.))
                                            .rounded_full()
                                            .bg(rgba_alpha(color, 0.15))
                                            .text_color(color)
                                            .text_sm()
                                            .child(icon),
                                    )
                                    // Slug
                                    .child(
                                        div()
                                            .text_color(colors::TEXT)
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(ws.slug.clone()),
                                    ),
                            )
                            // Status badge
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .px(px(8.))
                                            .py(px(3.))
                                            .rounded_full()
                                            .bg(rgba_alpha(color, 0.15))
                                            .border_1()
                                            .border_color(rgba_alpha(color, 0.3))
                                            .text_color(color)
                                            .text_xs()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(ws.status.to_string().to_uppercase()),
                                    ),
                            ),
                    )
                    // ── branch + agent row ────────────────────────────────────
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .px(px(6.))
                                    .py_px()
                                    .rounded(px(4.))
                                    .bg(rgba_alpha(agent_color, 0.15))
                                    .text_color(agent_color)
                                    .text_xs()
                                    .font_family("monospace")
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(agent_label),
                            )
                            .child(
                                div()
                                    .text_color(colors::SUBTEXT0)
                                    .text_xs()
                                    .font_family("monospace")
                                    .child(branch),
                            )
                            .when_some(ws.pr_number, |d, n| {
                                d.child(
                                    div()
                                        .px(px(6.))
                                        .py_px()
                                        .rounded(px(4.))
                                        .bg(rgba(0xfab38718))
                                        .text_color(colors::PEACH)
                                        .text_xs()
                                        .child(format!("PR #{n}")),
                                )
                            }),
                    )
                    // ── latest note ───────────────────────────────────────────
                    .when(!latest_note.is_empty(), |d| {
                        d.child(
                            div()
                                .flex()
                                .flex_row()
                                .items_start()
                                .gap_2()
                                .p_3()
                                .rounded_lg()
                                .bg(rgba(0x11111b80))
                                .border_1()
                                .border_color(colors::SURFACE0)
                                .child(
                                    div()
                                        .text_color(colors::OVERLAY0)
                                        .text_xs()
                                        .flex_shrink_0()
                                        .child("»"),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .text_color(colors::SUBTEXT0)
                                        .text_xs()
                                        .italic()
                                        .child(latest_note),
                                ),
                        )
                    }),
            )
    }
}

fn rgba_alpha(c: Rgba, a: f32) -> Rgba {
    Rgba { r: c.r, g: c.g, b: c.b, a }
}

impl Render for NotificationView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let workspaces: Vec<Workspace> = self
            .model.read(cx).workspaces().iter()
            .filter(|w| w.status.needs_attention())
            .cloned()
            .collect();
        let count = workspaces.len();

        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(colors::BASE)
            // ── header ────────────────────────────────────────────────────────
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
                            .gap_2()
                            .child(
                                div()
                                    .text_color(colors::TEXT)
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Alerts"),
                            )
                            .when(count > 0, |d| {
                                d.child(
                                    div()
                                        .px(px(7.))
                                        .py(px(2.))
                                        .rounded_full()
                                        .bg(rgba(0xf38ba820))
                                        .border_1()
                                        .border_color(rgba(0xf38ba840))
                                        .text_color(colors::RED)
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(count.to_string()),
                                )
                            }),
                    )
                    .child(
                        div()
                            .text_color(colors::OVERLAY0)
                            .text_xs()
                            .child(if count == 0 { "all clear" } else { "needs action" }),
                    ),
            )
            // ── cards ─────────────────────────────────────────────────────────
            .child(
                div()
                    .flex_1()
                    .overflow_y_scroll()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .when(workspaces.is_empty(), |d| {
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
                                            .text_color(colors::GREEN)
                                            .text_4xl()
                                            .child("✓"),
                                    )
                                    .child(
                                        div()
                                            .text_color(colors::SUBTEXT0)
                                            .text_sm()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child("All clear"),
                                    )
                                    .child(
                                        div()
                                            .text_color(colors::OVERLAY0)
                                            .text_xs()
                                            .child("No workspaces need attention."),
                                    ),
                            )
                    })
                    .children(workspaces.iter().map(|ws| Self::render_card(ws))),
            )
    }
}
