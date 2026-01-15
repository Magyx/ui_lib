#[allow(unused)]
use std::sync::OnceLock;

#[cfg(feature = "tracy")]
static TRACY_CLIENT: OnceLock<tracy_client::Client> = OnceLock::new();

#[inline]
pub fn init() {
    #[cfg(feature = "tracy")]
    {
        TRACY_CLIENT.get_or_init(tracy_client::Client::start);
    }
}

#[inline]
pub fn frame_mark() {
    #[cfg(feature = "tracy")]
    {
        init();
        tracy_client::frame_mark();
    }
}

#[inline]
#[allow(unused)]
pub fn set_thread_name(name: &str) {
    #[cfg(feature = "tracy")]
    {
        init();
        if let Some(c) = tracy_client::Client::running() {
            c.set_thread_name(name);
        }
    }
}

#[macro_export]
macro_rules! plot {
    ($name:literal, $value:expr) => {
        #[cfg(feature = "tracy")]
        {
            $crate::profile::init();
            tracy_client::plot!($name, $value);
        }
    };
}

/// Zero-overhead scope at callsite
#[macro_export]
macro_rules! scope {
    () => {
        #[cfg(feature = "tracy")]
        let _ui_scope = {
            $crate::profile::init();
            tracy_client::span!()
        };
    };
    ($name:expr) => {
        #[cfg(feature = "tracy")]
        let _ui_scope = {
            $crate::profile::init();
            tracy_client::span!($name)
        };
    };
    ($name:expr, $callstack_depth:expr) => {
        #[cfg(feature = "tracy")]
        let _ui_scope = {
            $crate::profile::init();
            tracy_client::span!($name, $callstack_depth)
        };
    };
}

/// Dynamic scope name (allocates span metadata), use sparingly.
/// This exists because Tracy’s fast path is built around static locations.
#[macro_export]
macro_rules! scope_dyn {
    ($name:expr) => {
        #[cfg(feature = "tracy")]
        let _ui_scope = {
            $crate::profile::init();
            tracy_client::Client::running().unwrap().span_alloc(
                Some($name),
                module_path!(),
                file!(),
                line!(),
                0,
            )
        };
    };
    ($name:expr, $callstack_depth:expr) => {
        #[cfg(feature = "tracy")]
        let _ui_scope = {
            $crate::profile::init();
            tracy_client::Client::running().unwrap().span_alloc(
                Some($name),
                module_path!(),
                file!(),
                line!(),
                $callstack_depth,
            )
        };
    };
}

#[macro_export]
macro_rules! ui_tracy_global_allocator {
    ($callstack_depth:expr) => {
        #[cfg(feature = "tracy")]
        #[global_allocator]
        static GLOBAL: tracy_client::ProfiledAllocator<std::alloc::System> =
            tracy_client::ProfiledAllocator::new(std::alloc::System, $callstack_depth);

        #[cfg(not(feature = "tracy"))]
        #[global_allocator]
        static GLOBAL: std::alloc::System = std::alloc::System;
    };
}

#[cfg(feature = "tracy")]
pub use tracy_client::ProfiledAllocator;
