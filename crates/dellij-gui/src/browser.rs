//! Browser pane: launches a separate OS-level window via wry.
//!
//! GPUI does not embed WebViews natively, so we spawn a dedicated wry
//! event-loop in its own thread. The window stays open until closed by the user.
use std::thread;

/// Spawn an OS WebView window navigated to `url`.
/// Returns immediately — the window runs in its own thread.
pub fn launch_browser_window(url: &str) {
    let url = url.to_string();
    thread::spawn(move || {
        if let Err(e) = run_browser_thread(&url) {
            eprintln!("dellij-gui browser error: {e}");
        }
    });
}

fn run_browser_thread(url: &str) -> anyhow::Result<()> {
    use tao::event::{Event, WindowEvent};
    use tao::event_loop::{ControlFlow, EventLoop};
    use tao::window::WindowBuilder;
    use wry::WebViewBuilder;

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(format!("dellij browser — {url}"))
        .with_inner_size(tao::dpi::LogicalSize::new(1200, 800))
        .build(&event_loop)?;

    let _webview = WebViewBuilder::new(&window)
        .with_url(url)
        .with_devtools(true)
        // Inject a helper script so agent code can call window.dellij.notify(msg)
        .with_initialization_script(DELLIJ_BROWSER_SCRIPT)
        .build()?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent { event: WindowEvent::CloseRequested, .. } = event {
            *control_flow = ControlFlow::Exit;
        }
    });
}

/// JS helper injected into every page opened in the dellij browser.
/// Exposes `window.dellij.snapshot()` and `window.dellij.click(selector)`.
const DELLIJ_BROWSER_SCRIPT: &str = r#"
window.dellij = {
  // Take a plain-text snapshot of visible text on the page.
  snapshot: function() {
    return document.body ? document.body.innerText : '';
  },

  // Click an element matching a CSS selector.
  click: function(selector) {
    const el = document.querySelector(selector);
    if (el) { el.click(); return true; }
    return false;
  },

  // Fill an input field.
  fill: function(selector, value) {
    const el = document.querySelector(selector);
    if (el) {
      el.value = value;
      el.dispatchEvent(new Event('input', { bubbles: true }));
      el.dispatchEvent(new Event('change', { bubbles: true }));
      return true;
    }
    return false;
  },

  // Evaluate arbitrary JS and return the result as a string.
  eval: function(code) {
    try { return String(eval(code)); } catch(e) { return 'error: ' + e.message; }
  },
};
"#;
