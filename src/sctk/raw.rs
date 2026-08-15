use std::ptr::NonNull;

use smithay_client_toolkit::reexports::client::{
    Connection, Proxy, protocol::wl_surface::WlSurface,
};

#[derive(Clone, Debug)]
pub struct WaylandHandles {
    display: NonNull<std::ffi::c_void>,
    surface: NonNull<std::ffi::c_void>,
}
impl wgpu::rwh::HasWindowHandle for WaylandHandles {
    fn window_handle(&self) -> Result<wgpu::rwh::WindowHandle<'_>, wgpu::rwh::HandleError> {
        let wl = wgpu::rwh::WaylandWindowHandle::new(self.surface);
        Ok(unsafe { wgpu::rwh::WindowHandle::borrow_raw(wgpu::rwh::RawWindowHandle::from(wl)) })
    }
}
impl wgpu::rwh::HasDisplayHandle for WaylandHandles {
    fn display_handle(&self) -> Result<wgpu::rwh::DisplayHandle<'_>, wgpu::rwh::HandleError> {
        let wl = wgpu::rwh::WaylandDisplayHandle::new(self.display);
        Ok(unsafe { wgpu::rwh::DisplayHandle::borrow_raw(wgpu::rwh::RawDisplayHandle::from(wl)) })
    }
}
unsafe impl Send for WaylandHandles {}
unsafe impl Sync for WaylandHandles {}
impl WaylandHandles {
    pub fn new(conn: &Connection, wl_surface: &WlSurface) -> Self {
        let display = NonNull::new(conn.display().id().as_ptr().cast()).expect("null wl_display");
        let surface = NonNull::new(wl_surface.id().as_ptr().cast()).expect("null wl_surface");
        Self { display, surface }
    }
}
