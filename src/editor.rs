use crate::Parameters;
use baseview::*;
use egui::*;
use egui_baseview::*;
use std::sync::Arc;
use vst::{editor::Editor, plugin::PluginParameters};

// ------------------ //
// 1. Setting UI size //
// ------------------ //
const WINDOW_WIDTH: usize = 256;
const WINDOW_HEIGHT: usize = 256;

// --------------------------------- //
// 2. Creating `PluginEditor` struct //
// --------------------------------- //
pub struct PluginEditor {
    pub params: Arc<Parameters>,
    pub window_handle: Option<WindowParent>,
    pub is_open: bool,
}

// ------------------------ //
// 3. Implementing `Editor` //
// ------------------------ //
impl Editor for PluginEditor {
    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn size(&self) -> (i32, i32) {
        (WINDOW_WIDTH as i32, WINDOW_HEIGHT as i32)
    }

    fn is_open(&mut self) -> bool {
        self.is_open
    }

    fn close(&mut self) {
        self.is_open = false;
        if let Some(mut window_handle) = self.window_handle.take() {
            (window_handle.0).close();
        }
    }

    fn open(&mut self, parent: *mut ::std::ffi::c_void) -> bool {
        log::info!("Editor open");
        match self.is_open {
            true => false,
            false => {
                // ---------------------------- //
                // 4. Setting up `egui` for use //
                // ---------------------------- //
                self.is_open = true;
                let settings = Settings {
                    window: WindowOpenOptions {
                        title: String::from("synthy"),
                        size: Size::new(WINDOW_WIDTH as f64, WINDOW_HEIGHT as f64),
                        scale: WindowScalePolicy::SystemScaleFactor,
                    },
                    render_settings: RenderSettings::default(),
                };

                let window_handle = EguiWindow::open_parented(
                    &VstParent(parent),
                    settings,
                    self.params.clone(),
                    |_egui_ctx, _queue, _state| {},
                    |egui_ctx: &CtxRef, _, state: &mut Arc<Parameters>| {
                        draw_ui(egui_ctx, state);
                    },
                );

                self.window_handle = Some(WindowParent(window_handle));
                true
            }
        }
    }    

}

// ---------------------------- //
// 4. Wrapper types boilerplate //
// ---------------------------- //
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

struct VstParent(*mut ::std::ffi::c_void);
unsafe impl Send for VstParent {}

pub struct WindowParent(pub WindowHandle);
unsafe impl Send for WindowParent {}

#[cfg(target_os = "macos")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::macos::MacOSHandle;

        RawWindowHandle::MacOS(MacOSHandle {
            ns_view: self.0 as *mut ::std::ffi::c_void,
            ..MacOSHandle::empty()
        })
    }
}

#[cfg(target_os = "windows")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::windows::WindowsHandle;

        RawWindowHandle::Windows(WindowsHandle {
            hwnd: self.0,
            ..WindowsHandle::empty()
        })
    }
}

#[cfg(target_os = "linux")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::unix::XcbHandle;

        RawWindowHandle::Xcb(XcbHandle {
            window: self.0 as u32,
            ..XcbHandle::empty()
        })
    }
}

#[inline(always)]
fn draw_ui(ctx: &CtxRef, params: &mut Arc<Parameters>) -> egui::Response {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical(|ui| {
            ui.label("hello rust");
            ui.label(format!(
                    "Modulation: {}",
                    params.get_parameter(crate::Parameter::Modulation as i32)
            ));
        })
    })
    .response
}
