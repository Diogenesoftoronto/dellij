mod app;
mod browser;
mod colors;
mod diff_view;
mod notifications;
mod sidebar;
mod watcher;

use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use clap::Parser;
use gpui::*;

use app::AppModel;

#[derive(Debug, Parser)]
#[command(name = "dellij-gui", about = "dellij desktop GUI")]
struct Cli {
    #[arg(long)]
    project_root: Option<Utf8PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let project_root = dellij_core::git::resolve_project_root(cli.project_root)?;

    App::new().run(move |cx: &mut AppContext| {
        let model = cx.new_model(|cx| AppModel::new(project_root.clone(), cx));

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Default::default(),
                    size: size(px(1280.), px(800.)),
                })),
                titlebar: Some(TitlebarOptions {
                    title: Some(SharedString::from("dellij")),
                    appears_transparent: false,
                    ..Default::default()
                }),
                focus: true,
                ..Default::default()
            },
            move |cx| {
                cx.new_view(|cx| app::RootView::new(model.clone(), cx))
            },
        )
        .expect("failed to open window")
        .activate(cx);
    });

    Ok(())
}
