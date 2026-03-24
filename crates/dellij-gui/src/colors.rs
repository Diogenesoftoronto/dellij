//! Catppuccin Mocha + semantic helpers.
use gpui::{hsla, Hsla, Rgba};

// ── Catppuccin Mocha palette ──────────────────────────────────────────────────

pub const BASE: Rgba       = rgba(0x1e1e2eff);
pub const MANTLE: Rgba     = rgba(0x181825ff);
pub const CRUST: Rgba      = rgba(0x11111bff);
pub const SURFACE0: Rgba   = rgba(0x313244ff);
pub const SURFACE1: Rgba   = rgba(0x45475aff);
pub const SURFACE2: Rgba   = rgba(0x585b70ff);
pub const OVERLAY0: Rgba   = rgba(0x6c7086ff);
pub const OVERLAY1: Rgba   = rgba(0x7f849cff);
pub const TEXT: Rgba       = rgba(0xcdd6f4ff);
pub const SUBTEXT0: Rgba   = rgba(0xa6adc8ff);
pub const SUBTEXT1: Rgba   = rgba(0xbac2deff);
pub const GREEN: Rgba      = rgba(0xa6e3a1ff);
pub const YELLOW: Rgba     = rgba(0xf9e2afff);
pub const RED: Rgba        = rgba(0xf38ba8ff);
pub const BLUE: Rgba       = rgba(0x89b4faff);
pub const CYAN: Rgba       = rgba(0x89dcebff);
pub const MAUVE: Rgba      = rgba(0xcba6f7ff);
pub const PEACH: Rgba      = rgba(0xfab387ff);
pub const PINK: Rgba       = rgba(0xf5c2e7ff);
pub const TEAL: Rgba       = rgba(0x94e2d5ff);
pub const SKY: Rgba        = rgba(0x89dcebff);
pub const SAPPHIRE: Rgba   = rgba(0x74c7ecff);
pub const LAVENDER: Rgba   = rgba(0xb4befeff);

// ── Semantic / alpha variants ─────────────────────────────────────────────────

/// Transparent green wash for diff +lines
pub const ADD_BG: Rgba    = rgba(0xa6e3a112);
/// Transparent red wash for diff -lines
pub const DEL_BG: Rgba    = rgba(0xf38ba812);
/// Transparent yellow wash for working status bg
pub const WORK_BG: Rgba   = rgba(0xf9e2af10);
/// Active selection highlight
pub const SELECT_BG: Rgba = rgba(0x313244cc);
/// Subtle border for cards
pub const CARD_BORDER: Rgba = rgba(0x45475a80);

// ── Status → color ────────────────────────────────────────────────────────────

use dellij_core::WorkspaceStatus;

pub fn status_color(s: WorkspaceStatus) -> Rgba {
    match s {
        WorkspaceStatus::Working => YELLOW,
        WorkspaceStatus::Waiting => CYAN,
        WorkspaceStatus::Blocked => RED,
        WorkspaceStatus::Review  => BLUE,
        WorkspaceStatus::Done    => GREEN,
        WorkspaceStatus::Error   => RED,
    }
}

pub fn status_bg(s: WorkspaceStatus) -> Rgba {
    match s {
        WorkspaceStatus::Working => rgba(0xf9e2af14),
        WorkspaceStatus::Waiting => rgba(0x89dceb14),
        WorkspaceStatus::Blocked => rgba(0xf38ba814),
        WorkspaceStatus::Review  => rgba(0x89b4fa14),
        WorkspaceStatus::Done    => rgba(0xa6e3a114),
        WorkspaceStatus::Error   => rgba(0xf38ba814),
    }
}

pub fn status_icon(s: WorkspaceStatus) -> &'static str {
    match s {
        WorkspaceStatus::Working => "⬤",
        WorkspaceStatus::Waiting => "◯",
        WorkspaceStatus::Blocked => "⊘",
        WorkspaceStatus::Review  => "◈",
        WorkspaceStatus::Done    => "✓",
        WorkspaceStatus::Error   => "✗",
    }
}

// ── Agent → color + label ─────────────────────────────────────────────────────

pub fn agent_color(agent: &str) -> Rgba {
    if agent.contains("claude") { return MAUVE; }
    if agent.contains("codex")  { return BLUE; }
    if agent.contains("opencode") { return TEAL; }
    if agent.contains("aider")  { return YELLOW; }
    if agent.contains("gemini") { return SAPPHIRE; }
    if agent.contains("cline")  { return PEACH; }
    if agent.contains("cursor") { return PINK; }
    OVERLAY1
}

pub fn agent_short(agent: &str) -> &'static str {
    if agent.contains("claude")   { return "cc"; }
    if agent.contains("codex")    { return "cx"; }
    if agent.contains("opencode") { return "oc"; }
    if agent.contains("aider")    { return "ai"; }
    if agent.contains("gemini")   { return "gm"; }
    if agent.contains("cline")    { return "cl"; }
    if agent.contains("cursor")   { return "cr"; }
    "??"
}
