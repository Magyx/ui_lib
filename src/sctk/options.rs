use smithay_client_toolkit::shell::{
    wlr_layer::{Anchor, KeyboardInteractivity, Layer},
    xdg::window::WindowDecorations,
};

use crate::model::Size;

#[derive(Clone, Debug)]
pub enum OutputSet {
    /// Use single-output selector
    One(OutputSelector),
    /// Use the last active output.
    Active,
    /// Mirror the surface to every compositor output
    All,
    /// Explicit list
    List(Vec<OutputSelector>),
}

#[derive(Clone, Debug)]
pub enum OutputSelector {
    /// First output in SCTK’s list (current behavior)
    First,
    /// Nth output (0-based)
    Index(usize),
    /// Choose the output whose info.name/model/make starts with this string
    NamePrefix(String),
    /// Prefer laptop panel-ish names (eDP, LVDS), fall back to First
    InternalPrefer,
    /// Pick the output with the highest reported scale factor
    HighestScale,
}

/// Options describing the layer-shell surface (instead of winit's WindowAttributes).
#[derive(Clone, Debug)]
pub struct LayerOptions {
    pub layer: Layer,
    pub size: Size<u32>,
    pub anchors: Anchor,
    /// Negative means "auto" (no reservation). Positive reserves screen space (e.g. status bar).
    pub exclusive_zone: i32,
    pub keyboard_interactivity: KeyboardInteractivity,
    /// Namespace, useful for compositor rules.
    pub namespace: Option<String>,
    pub output: Option<OutputSet>,
}

impl Default for LayerOptions {
    fn default() -> Self {
        Self {
            layer: Layer::Top,
            size: Size::new(640, 360),
            anchors: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
            exclusive_zone: -1,
            keyboard_interactivity: KeyboardInteractivity::None,
            namespace: Some("ui".to_string()),
            output: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct XdgOptions {
    pub size: Size<u32>,
    pub title: String,
    pub app_id: Option<String>,
    pub decorations: WindowDecorations,
    pub output: Option<OutputSelector>,
}

impl Default for XdgOptions {
    fn default() -> Self {
        Self {
            size: Size::new(640, 360),
            title: "my_app".to_string(),
            app_id: Some("ui".to_string()),
            decorations: WindowDecorations::RequestClient,
            output: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LockOptions {
    pub size: Size<u32>,
    pub output: Option<OutputSet>,
}

impl Default for LockOptions {
    fn default() -> Self {
        Self {
            size: Size::new(640, 360),
            output: None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Options {
    Layer(LayerOptions),
    Xdg(XdgOptions),
    Lock(LockOptions),
}
