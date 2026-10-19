//! Native window composition for UI Bridge panel instances.
use super::*;
use photag_ui_bridge::panel_catalog::PanelCatalog;
use std::collections::BTreeMap;

#[derive(Default)]
pub(crate) struct NativePanels {
    windows: BTreeMap<String, (AnyWindowHandle, u64)>,
    workspace_activation: Option<String>,
}

impl PhoTagShell {
    pub(crate) fn sync_native_panels(&mut self, cx: &mut Context<Self>) {
        let Ok(bridge) = self
            .cordis
            .try_service::<photag_ui_bridge::UiBridgeService>()
        else {
            return;
        };
        let Ok(panels) = bridge.panels.native_panels() else {
            return;
        };
        let stale = self
            .native_panels
            .windows
            .keys()
            .filter(|id| !panels.iter().any(|panel| &panel.id == *id))
            .cloned()
            .collect::<Vec<_>>();
        for id in stale {
            if let Some((handle, _)) = self.native_panels.windows.remove(&id) {
                let _ = handle.update(cx, |_, window, _| window.remove_window());
            }
            if self.surfaces.value("mcp.window.selected").as_deref() == Some(&id) {
                self.capture.set(self.native_window_handle);
                let _ = self.surfaces.set_value("mcp.window.selected", "main");
            }
        }
        for panel in panels {
            if let Some((handle, activation)) = self.native_panels.windows.get_mut(&panel.id) {
                if *activation != panel.activation {
                    *activation = panel.activation;
                    activate_panel(*handle, cx);
                }
                continue;
            }
            let id = panel.id.clone();
            let catalog = bridge.panels.clone();
            let surfaces = self.surfaces.clone();
            let capture = self.capture.clone();
            let options = &panel.definition.options;
            let window_options = WindowOptions {
                window_bounds: Some(WindowBounds::centered(
                    size(px(options.default_size.0), px(options.default_size.1)),
                    cx,
                )),
                window_min_size: Some(size(px(options.min_size.0), px(options.min_size.1))),
                ..WindowOptions::default()
            };
            let result = gpui_kit::open_window(window_options, cx, move |window, cx| {
                window
                    .set_window_title(&photag_ui_bridge::i18n::tr(&panel.definition.options.title));
                let hwnd = native_handle(window);
                photag_ui_bridge::winfx::apply_backdrop(hwnd);
                let _ = surfaces.set_value(&format!("panel.{}.hwnd", panel.id), &hwnd.to_string());
                let close_id = panel.id.clone();
                let close_catalog = catalog.clone();
                window.on_window_should_close(cx, move |_, _| {
                    let _ = close_catalog.close(&close_id);
                    true
                });
                let content = (panel.definition.factory)(window, cx);
                cx.new(|cx| {
                    photag_ui_bridge::refresh::poll(
                        cx,
                        std::time::Duration::from_millis(100),
                        |view: &mut NativePanelView, _| {
                            (view.capture.get() == view.hwnd
                                && (view.capture.pointer_pending() || view.capture.key_pending()))
                            .into()
                        },
                    );
                    let focus = cx.focus_handle();
                    window.focus(&focus, cx);
                    NativePanelView {
                        renders: 0,
                        content,
                        id: panel.id,
                        surfaces,
                        catalog,
                        capture,
                        hwnd,
                        focus,
                    }
                })
            });
            match result {
                Ok((handle, _)) => {
                    // Activate only after the native window and its Root exist.
                    activate_panel(handle, cx);
                    self.native_panels
                        .windows
                        .insert(id, (handle, panel.activation));
                }
                Err(error) => {
                    tracing::error!(%error, panel = %id, "could not open native plugin window");
                    let _ = bridge.panels.close(&id);
                }
            }
        }
        let activation = self.surfaces.value("workspace.activate");
        if activation != self.native_panels.workspace_activation {
            self.native_panels.workspace_activation = activation;
            activate_main(self.native_window_handle);
        }
        let selected = self
            .surfaces
            .value("mcp.window.selected")
            .unwrap_or_else(|| "main".into());
        let hwnd = if selected == "main" {
            self.native_window_handle
        } else {
            self.surfaces
                .value(&format!("panel.{selected}.hwnd"))
                .and_then(|v| v.parse().ok())
                .unwrap_or(self.native_window_handle)
        };
        self.capture.set(hwnd);
        let _ = self
            .surfaces
            .set_value("mcp.window.main", &self.native_window_handle.to_string());
    }
}

struct NativePanelView {
    renders: u64,
    content: AnyView,
    id: String,
    surfaces: Arc<photag_ui_bridge::UiSurfaceState>,
    catalog: Arc<PanelCatalog>,
    capture: Arc<photag_plugin_mcp::CaptureWindowHandle>,
    hwnd: isize,
    focus: FocusHandle,
}

impl Render for NativePanelView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.capture.get() == self.hwnd {
            super::panel_resize::dispatch_mcp_input(&self.capture, window, cx);
        }
        self.renders += 1;
        let _ = self.surfaces.set_value(
            &format!("panel.{}.render_count", self.id),
            &self.renders.to_string(),
        );
        let viewport = window.bounds().size;
        let bounds = format!(
            "0,0,{},{}",
            f32::from(viewport.width),
            f32::from(viewport.height)
        );
        let _ = self
            .surfaces
            .set_value(&format!("panel.{}.content_bounds", self.id), &bounds);
        div()
            .size_full()
            .min_w_0()
            .min_h_0()
            .bg(rgb(theme::surface_panel()))
            .text_color(rgb(theme::text_primary()))
            .font_family(theme::UI_FONT_FAMILY)
            .text_size(px(theme::FONT_SIZE_BODY))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, _| {
                if event.keystroke.key.eq_ignore_ascii_case("escape") {
                    let _ = this.catalog.close(&this.id);
                }
            }))
            .child(self.content.clone())
    }
}

fn native_handle(window: &Window) -> isize {
    #[cfg(target_os = "windows")]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        HasWindowHandle::window_handle(window)
            .ok()
            .and_then(|h| match h.as_raw() {
                RawWindowHandle::Win32(h) => Some(h.hwnd.get()),
                _ => None,
            })
            .unwrap_or_default()
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = window;
        0
    }
}

fn activate_main(hwnd: isize) {
    photag_ui_bridge::winfx::restore_and_activate(hwnd);
}

/// Both first open and single-instance reopen use the same target-only activation.
fn activate_panel(handle: AnyWindowHandle, cx: &mut App) {
    let _ = handle.update(cx, |_, window, cx| {
        window.defer(cx, |window, _| {
            #[cfg(target_os = "windows")]
            photag_ui_bridge::winfx::restore_and_activate(native_handle(window));
            // GPUI owns the platform activation path (including Windows focus
            // restrictions); showing a HWND alone is not sufficient.
            window.activate_window();
        });
    });
}
