use gpui::*;
use dellij_core::{Workspace, WorkspaceStatus};
pub mod auth;
use auth::GitHubAuth;
use convex::{ConvexClient, Value};
use std::collections::BTreeMap;
use futures::StreamExt;
use std::env;

struct MobileApp {
    workspaces: Vec<Workspace>,
    convex_url: String,
    github_auth: GitHubAuth,
    access_token: Option<String>,
}

impl MobileApp {
    fn new(cx: &mut Context<Self>) -> Self {
        let convex_url = env::var("CONVEX_URL").unwrap_or_else(|_| "https://your-deployment.convex.cloud".to_string());
        let auth_token = env::var("CONVEX_AUTH_TOKEN").ok();
        
        let mut app = Self {
            workspaces: Vec::new(),
            convex_url: convex_url.clone(),
            github_auth: GitHubAuth::new(),
            access_token: None,
        };

        let view_handle = cx.handle().downgrade();
        
        cx.spawn(|mut cx| async move {
            if let Ok(mut client) = ConvexClient::new(&convex_url).await {
                if let Some(token) = auth_token {
                    client.set_auth(Some(token));
                }

                if let Ok(mut subscription) = client.subscribe("workspaces:list", BTreeMap::new()).await {
                    while let Some(result) = subscription.next().await {
                        if let Ok(Value::Array(items)) = result {
                            let mut new_workspaces = Vec::new();
                            for item in items {
                                if let Value::Object(obj) = item {
                                    if let Some(Value::String(slug)) = obj.get("slug") {
                                        new_workspaces.push(Workspace {
                                            slug: slug.clone(),
                                            prompt: String::new(),
                                            agent: String::new(),
                                            branch_name: String::new(),
                                            base_branch: String::new(),
                                            worktree_path: camino::Utf8PathBuf::new(),
                                            status: WorkspaceStatus::Working,
                                            created_at: chrono::Utc::now(),
                                            updated_at: chrono::Utc::now(),
                                            ports: vec![],
                                            urls: vec![],
                                            last_command: None,
                                            notes: vec![],
                                            pr_number: None,
                                            pr_url: None,
                                            layout: None,
                                        });
                                    }
                                }
                            }
                            
                            let _ = view_handle.update(&mut cx, |this, cx| {
                                this.workspaces = new_workspaces;
                                cx.notify();
                            });
                        }
                    }
                }
            }
        }).detach();

        app
    }

    fn login(&mut self, _cx: &mut Context<Self>) {
        let (url, _csrf) = self.github_auth.authorize_url();
        let _ = opener::open(url.as_str());
        // In a real mobile app, we'd handle the dellij://auth deep link
        // which would call self.handle_callback(code)
    }
}

struct HomeView {
    app: Model<MobileApp>,
}

impl HomeView {
    fn new(app: Model<MobileApp>, _cx: &mut ViewContext<Self>) -> Self {
        Self { app }
    }
}

impl Render for HomeView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let app = self.app.read(cx);
        
        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(rgb(0x1e1e2e))
            .child(
                div()
                    .h(px(60.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .border_b_1()
                    .border_color(rgb(0x313244))
                    .child(
                        div()
                            .text_color(rgb(0xcba6f7))
                            .font_weight(FontWeight::BOLD)
                            .text_lg()
                            .child("dellij mobile")
                    )
                    .child(
                        div()
                            .cursor_pointer()
                            .on_click(cx.listener(|view, _, cx| {
                                view.app.update(cx, |app, cx| app.login(cx));
                            }))
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(rgb(0x313244))
                                    .text_color(rgb(0xcdd6f4))
                                    .text_sm()
                                    .child(if app.access_token.is_some() { "Linked" } else { "Link GitHub" })
                            )
                    )
            )
            .child(
                div()
                    .flex_1()
                    .overflow_y_scroll()
                    .children(app.workspaces.iter().map(|ws| {
                        self.render_workspace_item(ws, cx)
                    }))
            )
            .child(
                div()
                    .h(px(70.))
                    .bg(rgb(0x181825))
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_around()
                    .child(self.render_nav_item("⌂", "Home", true))
                    .child(self.render_nav_item("⚠", "Alerts", false))
                    .child(self.render_nav_item("⚙", "Settings", false))
            )
    }
}

impl HomeView {
    fn render_workspace_item(&self, ws: &Workspace, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .items_center()
            .p_4()
            .border_b_1()
            .border_color(rgb(0x313244))
            .child(
                div()
                    .w(px(12.))
                    .h(px(12.))
                    .rounded_full()
                    .bg(self.status_color(ws.status))
            )
            .child(
                div()
                    .flex_col()
                    .ml_4()
                    .child(
                        div()
                            .text_color(rgb(0xcdd6f4))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(ws.slug.clone())
                    )
                    .child(
                        div()
                            .text_color(rgb(0xa6adc8))
                            .text_xs()
                            .child(format!("{} • {}", ws.agent, ws.branch_name))
                    )
            )
    }

    fn render_nav_item(&self, icon: &'static str, label: &'static str, active: bool) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .child(
                div()
                    .text_xl()
                    .text_color(if active { rgb(0xcba6f7) } else { rgb(0x6c7086) })
                    .child(icon)
            )
            .child(
                div()
                    .text_xs()
                    .text_color(if active { rgb(0xcba6f7) } else { rgb(0x6c7086) })
                    .child(label)
            )
    }

    fn status_color(&self, status: WorkspaceStatus) -> Rgb {
        match status {
            WorkspaceStatus::Working => rgb(0x89b4fa), // Blue
            WorkspaceStatus::Blocked => rgb(0xf9e2af), // Yellow
            WorkspaceStatus::Error   => rgb(0xf38ba8), // Red
            WorkspaceStatus::Done    => rgb(0xa6e3a1), // Green
            WorkspaceStatus::Review  => rgb(0xcba6f7), // Mauve
            WorkspaceStatus::Waiting => rgb(0x6c7086), // Overlay0
        }
    }
}

fn main() {
    env_logger::init();
    App::new().run(|cx: &mut AppContext| {
        let app_model = cx.new_model(|cx| MobileApp::new(cx));
        cx.open_window(WindowOptions::default(), |cx| {
            cx.new_view(|cx| HomeView::new(app_model, cx))
        });
    });
}
