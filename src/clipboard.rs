pub trait ClipboardProvider {
    /// Current clipboard text, or `None` if empty/unavailable.
    fn get_text(&mut self) -> Option<String>;
    /// Replace the clipboard contents.
    fn set_text(&mut self, text: String);
}

#[derive(Default)]
pub struct LocalClipboard {
    contents: Option<String>,
}

impl ClipboardProvider for LocalClipboard {
    fn get_text(&mut self) -> Option<String> {
        self.contents.clone()
    }
    fn set_text(&mut self, text: String) {
        self.contents = Some(text);
    }
}

pub struct Clipboard {
    provider: Box<dyn ClipboardProvider>,
}

impl Default for Clipboard {
    fn default() -> Self {
        Self::new(LocalClipboard::default())
    }
}

impl Clipboard {
    pub fn new(provider: impl ClipboardProvider + 'static) -> Self {
        Self {
            provider: Box::new(provider),
        }
    }

    /// Swap the backing provider (e.g. a backend installing the OS clipboard).
    pub(crate) fn set_provider(&mut self, provider: impl ClipboardProvider + 'static) {
        self.provider = Box::new(provider);
    }

    pub fn get_text(&mut self) -> Option<String> {
        self.provider.get_text()
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.provider.set_text(text.into());
    }
}

// arboard: cross-platform (Windows / macOS / Linux-X11, optional Wayland)
#[cfg(feature = "clipboard-arboard")]
mod arboard_impl {
    use super::ClipboardProvider;

    /// System clipboard backed by [`arboard`].
    ///
    /// Works on Windows, macOS, and Linux. On Linux it uses X11 by default
    /// (functional under XWayland); enable arboard's own `wayland-data-control`
    /// feature for native Wayland with automatic X11 fallback. Note the
    /// data-control protocol isn't supported by every compositor (e.g. GNOME) —
    /// prefer [`super::SmithayClipboard`] when running the SCTK backend.
    pub struct ArboardClipboard {
        inner: arboard::Clipboard,
    }

    impl ArboardClipboard {
        /// Open the system clipboard. Returns `None` if it's unavailable
        /// (e.g. headless / no display server), so callers can fall back to
        /// the in-process [`super::LocalClipboard`].
        pub fn new() -> Option<Self> {
            match arboard::Clipboard::new() {
                Ok(inner) => Some(Self { inner }),
                Err(_e) => {
                    #[cfg(feature = "tracing")]
                    tracing::warn!("arboard: could not open system clipboard: {_e}");
                    None
                }
            }
        }
    }

    impl ClipboardProvider for ArboardClipboard {
        fn get_text(&mut self) -> Option<String> {
            match self.inner.get_text() {
                Ok(s) => Some(s),
                // Empty / non-text clipboard is normal: nothing to paste.
                Err(arboard::Error::ContentNotAvailable) => None,
                Err(_e) => {
                    #[cfg(feature = "tracing")]
                    tracing::warn!("arboard: get_text failed: {_e}");
                    None
                }
            }
        }

        fn set_text(&mut self, text: String) {
            if let Err(_e) = self.inner.set_text(text) {
                #[cfg(feature = "tracing")]
                tracing::warn!("arboard: set_text failed: {_e}");
            }
        }
    }
}
#[cfg(feature = "clipboard-arboard")]
pub use arboard_impl::ArboardClipboard;

// smithay-clipboard: native Wayland for windowed apps (the SCTK backend)
#[cfg(feature = "clipboard-smithay")]
mod smithay_impl {
    use std::ffi::c_void;

    use super::ClipboardProvider;

    /// Native Wayland clipboard backed by [`smithay_clipboard`].
    ///
    /// Speaks the core `wl_data_device` protocol, so it works on every
    /// compositor (including GNOME), unlike arboard's data-control path. It
    /// runs its own worker thread, so `load`/`store` take `&self`.
    pub struct SmithayClipboard {
        inner: smithay_clipboard::Clipboard,
    }

    impl SmithayClipboard {
        /// # Safety
        /// `display_ptr` must be a valid `*mut wl_display` for the connection
        /// the app's surfaces live on, and must outlive this clipboard. Get it
        /// from the SCTK `Connection` (see the wiring note below). Must be
        /// created on the main thread.
        pub unsafe fn new(display_ptr: *mut c_void) -> Self {
            Self {
                inner: unsafe { smithay_clipboard::Clipboard::new(display_ptr) },
            }
        }
    }

    impl ClipboardProvider for SmithayClipboard {
        fn get_text(&mut self) -> Option<String> {
            // `load` errors on an empty clipboard — treat as "nothing to paste".
            self.inner.load().ok()
        }

        fn set_text(&mut self, text: String) {
            self.inner.store(text);
        }
    }
}
#[cfg(feature = "clipboard-smithay")]
pub use smithay_impl::SmithayClipboard;
