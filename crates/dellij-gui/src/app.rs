//! Root application model and top-level view.
use std::fs;

use camino::Utf8PathBuf;
use gpui::*;

use dellij_core::{Config, Workspace};

use crate::colors;
use crate::diff_view::DiffView;
use crate::notifications::NotificationView;
use crate::sidebar::SidebarView;
use crate::watcher::StatusWatcher;

// ── Panel ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Panel { Diff, Notifications, Browser }

// ── AppModel ──────────────────────────────────────────────────────────────────

pub struct AppModel {
    pub project_root: Utf8PathBuf,
    pub config: Option<Config>,
    pub selected_slug: Option<String>,
    pub active_panel: Panel,
    pub diff_cache: std::collections::HashMap<String, String>,
    _watcher: Option<StatusWatcher>,
}

impl AppModel {
    pub fn new(project_root: Utf8PathBuf, cx: &mut ModelContext<Self>) -> Self {
        let config = Self::load_config(&project_root);
        let status_dir = project_root.join(".dellij/status");

        let handle = cx.handle();
        let watcher = StatusWatcher::spawn(&status_dir, move || {
            drop(handle.clone());
        }).ok();

        let selected = config
            .as_ref()
            .and_then(|c| c.workspaces.first())
            .map(|w| w.slug.clone());

        Self { project_root, config, selected_slug: selected,
               active_panel: Panel::Diff, diff_cache: Default::default(),
               _watcher: watcher }
    }

    pub fn reload(&mut self, cx: &mut ModelContext<Self>) {
        self.config = Self::load_config(&self.project_root);
        self.diff_cache.clear();
        cx.notify();
    }

    pub fn select(&mut self, slug: String, cx: &mut ModelContext<Self>) {
        self.selected_slug = Some(slug.clone());
        if !self.diff_cache.contains_key(&slug) {
            if let Some(ws) = self.selected_workspace() {
                let diff = dellij_core::git::workspace_diff(
                    &self.project_root, &ws.base_branch, &ws.branch_name,
                ).unwrap_or_default();
                self.diff_cache.insert(slug, diff);
            }
        }
        cx.notify();
    }

    pub fn set_panel(&mut self, panel: Panel, cx: &mut ModelContext<Self>) {
        self.active_panel = panel;
        cx.notify();
    }

    pub fn selected_workspace(&self) -> Option<&Workspace> {
        let slug = self.selected_slug.as_ref()?;
        self.config.as_ref()?.workspaces.iter().find(|w| &w.slug == slug)
    }

    pub fn workspaces(&self) -> &[Workspace] {
        self.config.as_ref().map(|c| c.workspaces.as_slice()).unwrap_or(&[])
    }

    pub fn attention_count(&self) -> usize {
        self.workspaces().iter().filter(|w| w.status.needs_attention()).count()
    }

    pub fn open_in_editor(&self, slug: &str, editor: &str) -> anyhow::Result<()> {
        if let Some(ws) = self.config.as_ref()
            .and_then(|c| c.workspaces.iter().find(|w| w.slug == slug))
        {
            dellij_core::git::open_in_editor(editor, &ws.worktree_path)?;
        }
        Ok(())
    }

    fn load_config(root: &Utf8PathBuf) -> Option<Config> {
        let raw = fs::read_to_string(root.join(".dellij/dellij.json")).ok()?;
        serde_json::from_str(&raw).ok()
    }
}

// ── RootView ──────────────────────────────────────────────────────────────────

pub struct RootView {
    model: Model<AppModel>,
    sidebar: View<SidebarView>,
    diff: View<DiffView>,
    notifications: View<NotificationView>,
}

impl RootView {
    pub fn new(model: Model<AppModel>, cx: &mut ViewContext<Self>) -> Self {
        let sidebar      = cx.new_view(|cx| SidebarView::new(model.clone(), cx));
        let diff         = cx.new_view(|cx| DiffView::new(model.clone(), cx));
        let notifications = cx.new_view(|cx| NotificationView::new(model.clone(), cx));
        cx.subscribe(&model, |_, _, _, cx| cx.notify()).detach();
        Self { model, sidebar, diff, notifications }
    }

    fn render_titlebar(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let app      = self.model.read(cx);
        let project  = app.project_root
            .file_name()
            .unwrap_or(app.project_root.as_str())
            .to_string();
        let ws_count = app.workspaces().len();
        let attn     = app.attention_count();

        // Mini status pills for quick overview
        let status_pills = app.workspaces().iter().map(|ws| {
            div()
                .w(px(8.))
                .h(px(8.))
                .rounded_full()
                .bg(colors::status_color(ws.status))
                .flex_shrink_0()
        });

        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .h(px(44.))
            .px_4()
            .bg(colors::CRUST)
            .border_b_1()
            .border_color(colors::SURFACE0)
            .shadow(smallvec![BoxShadow {
                color: rgba(0x00000030),
                offset: point(px(0.), px(1.)),
                blur_radius: px(4.),
                spread_radius: px(0.),
            }])
            // Left: branding + project
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .text_color(colors::MAUVE)
                            .font_weight(FontWeight::BOLD)
                            .text_base()
                            .child("◆ dellij"),
                    )
                    .child(
                        div()
                            .w_px()
                            .h(px(16.))
                            .bg(colors::SURFACE1),
                    )
                    .child(
                        div()
                            .text_color(colors::SUBTEXT0)
                            .text_sm()
                            .child(project),
                    ),
            )
            // Center: workspace dot indicators
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(5.))
                    .children(status_pills),
            )
            // Right: stats
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .text_color(colors::OVERLAY0)
                            .text_xs()
                            .child(format!("{ws_count} workspaces")),
                    )
                    .when(attn > 0, |d| {
                        d.child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_1()
                                .px(px(7.))
                                .py(px(2.))
                                .rounded_full()
                                .bg(rgba(0xf38ba820))
                                .border_1()
                                .border_color(rgba(0xf38ba840))
                                .child(
                                    div()
                                        .text_color(colors::RED)
                                        .text_xs()
                                        .child(format!("⚠ {attn}")),
                                ),
                        )
                    }),
            )
    }

    fn render_tab_pills(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let panel = self.model.read(cx).active_panel;
        let attn  = self.model.read(cx).attention_count();

        let pill = |icon: &'static str, label: &'static str, count: usize,
                     p: Panel, cx: &mut ViewContext<Self>| {
            let model  = self.model.clone();
            let active = panel == p;
            let badge  = if count > 0 {
                Some(div()
                    .px(px(5.))
                    .py_px()
                    .rounded_full()
                    .bg(if active { rgba(0xf38ba840) } else { rgba(0xf38ba825) })
                    .text_color(if active { colors::RED } else { colors::OVERLAY0 })
                    .text_xs()
                    .child(count.to_string()))
            } else {
                None
            };

            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_1()
                .px_3()
                .py(px(5.))
                .rounded_full()
                .cursor_pointer()
                .bg(if active { colors::SURFACE1 } else { Rgba::default() })
                .text_color(if active { colors::TEXT } else { colors::OVERLAY0 })
                .shadow(if active {
                    smallvec![BoxShadow {
                        color: rgba(0x00000030),
                        offset: point(px(0.), px(1.)),
                        blur_radius: px(3.),
                        spread_radius: px(0.),
                    }]
                } else {
                    smallvec![]
                })
                .hover(|s| {
                    s.bg(colors::SURFACE0)
                     .text_color(colors::SUBTEXT1)
                })
                .on_click(cx.listener(move |_, _, cx| {
                    model.update(cx, |m, cx| m.set_panel(p, cx));
                }))
                .child(div().text_xs().child(icon))
                .child(div().text_sm().child(label))
                .when_some(badge, |d, b| d.child(b))
        };

        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .px_4()
            .py_2()
            .bg(colors::MANTLE)
            .border_b_1()
            .border_color(colors::SURFACE0)
            .child(pill("~", "Diff",          0,    Panel::Diff,          cx))
            .child(pill("⚠", "Alerts",       attn, Panel::Notifications, cx))
            .child(pill("⬡", "Browser",      0,    Panel::Browser,       cx))
    }

    fn render_content(&self, cx: &mut ViewContext<Self>) -> AnyElement {
        match self.model.read(cx).active_panel {
            Panel::Diff          => self.diff.clone().into_any_element(),
            Panel::Notifications => self.notifications.clone().into_any_element(),
            Panel::Browser       => cx.new_view(|_| BrowserPrompt).into_any_element(),
        }
    }
}

impl Render for RootView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(colors::BASE)
            .text_color(colors::TEXT)
            .font_family("system-ui")
            .child(self.render_titlebar(cx))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .overflow_hidden()
                    // Sidebar
                    .child(
                        div()
                            .w(px(280.))
                            .h_full()
                            .flex_shrink_0()
                            .bg(colors::MANTLE)
                            .border_r_1()
                            .border_color(colors::SURFACE0)
                            .child(self.sidebar.clone()),
                    )
                    // Main panel
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(self.render_tab_pills(cx))
                            .child(
                                div()
                                    .flex_1()
                                    .overflow_hidden()
                                    .child(self.render_content(cx)),
                            ),
                    ),
            )
    }
}

// ── BrowserPrompt ─────────────────────────────────────────────────────────────

struct BrowserPrompt;

impl Render for BrowserPrompt {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .w_full()
            .h_full()
            .gap_4()
            .bg(colors::BASE)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_3()
                    .p_8()
                    .rounded_2xl()
                    .bg(colors::MANTLE)
                    .border_1()
                    .border_color(colors::SURFACE1)
                    .shadow(smallvec![BoxShadow {
                        color: rgba(0x00000040),
                        offset: point(px(0.), px(4.)),
                        blur_radius: px(24.),
                        spread_radius: px(0.),
                    }])
                    .child(
                        div()
                            .text_color(colors::OVERLAY0)
                            .text_4xl()
                            .child("⬡"),
                    )
                    .child(
                        div()
                            .text_color(colors::TEXT)
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Browser"),
                    )
                    .child(
                        div()
                            .text_color(colors::OVERLAY0)
                            .text_sm()
                            .text_center()
                            .max_w(px(280.))
                            .child("Click a port or URL in the sidebar to launch the browser window."),
                    ),
            )
    }
}
