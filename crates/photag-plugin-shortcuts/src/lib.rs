#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
//! Lifecycle-aware application and system shortcut dispatch.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cordis::{Context, Plugin, Routing, Service};
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{Code, HotKey, Modifiers},
};
use photag_ui_bridge::{ShortcutScope, UiBridgeService, UiCatalog, UiCommandInvoked};

thread_local! {
    // Windows' native event-loop pointer is thread-bound. Registration and
    // removal therefore stay on the GPUI thread.
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry::default());
}

#[derive(Default)]
struct Registry {
    manager: Option<GlobalHotKeyManager>,
    signature: Vec<(String, String)>,
    entries: Vec<(HotKey, String)>,
}

/// Applies lifecycle-owned bindings and forwards presses through Cordis.
pub struct ShortcutService {
    catalog: Arc<UiCatalog>,
    command_sender: tokio::sync::mpsc::UnboundedSender<Option<String>>,
    stop_requested: AtomicBool,
}

impl ShortcutService {
    /// Must run on GPUI's event-loop thread.
    pub fn start(&self, overrides: &BTreeMap<String, String>) -> Result<(), String> {
        self.sync_system_bindings(overrides, false)
    }

    /// Apply registrations on the UI thread and dispatch pressed command IDs.
    pub fn poll(
        &self,
        _hwnd: isize,
        overrides: &BTreeMap<String, String>,
        capture_active: bool,
    ) -> bool {
        if let Err(error) = self.sync_system_bindings(overrides, capture_active) {
            tracing::warn!(%error, "could not refresh global shortcuts");
        }
        if self.stop_requested.swap(false, Ordering::AcqRel) {
            REGISTRY.with(|registry| {
                let mut registry = registry.borrow_mut();
                unregister_all(&mut registry);
                registry.signature.clear();
            });
            return false;
        }
        let mut invoked = Vec::new();
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state() != HotKeyState::Pressed {
                continue;
            }
            REGISTRY.with(|registry| {
                if let Some((_, command_id)) = registry
                    .borrow()
                    .entries
                    .iter()
                    .find(|(hotkey, _)| hotkey.id() == event.id())
                {
                    invoked.push(command_id.clone());
                }
            });
        }
        for command_id in &invoked {
            if self.command_sender.send(Some(command_id.clone())).is_err() {
                tracing::error!(command = %command_id, "global shortcut command dispatcher is closed");
            }
        }
        !invoked.is_empty()
    }

    fn sync_system_bindings(
        &self,
        overrides: &BTreeMap<String, String>,
        capture_active: bool,
    ) -> Result<(), String> {
        let commands = self.catalog.commands().map_err(|error| error.to_string())?;
        let mut desired = Vec::new();
        for command in commands {
            for binding in command
                .keybindings
                .into_iter()
                .filter(|binding| binding.scope == ShortcutScope::System)
            {
                if capture_active {
                    continue;
                }
                let chord = overrides.get(&binding.id).unwrap_or(&binding.key);
                if chord.is_empty() {
                    continue;
                }
                let hotkey = parse_hotkey(chord).ok_or_else(|| {
                    format!("unsupported system shortcut `{chord}` ({})", binding.id)
                })?;
                desired.push((binding.id, command.id.clone(), chord.clone(), hotkey));
            }
        }
        desired.sort_by(|a, b| a.0.cmp(&b.0));
        let signature = desired
            .iter()
            .map(|(id, _, chord, _)| (id.clone(), chord.clone()))
            .collect::<Vec<_>>();
        REGISTRY.with(|registry| -> Result<(), String> {
            let mut registry = registry.borrow_mut();
            if registry.signature == signature { return Ok(()); }
            unregister_all(&mut registry);
            if registry.manager.is_none() {
                registry.manager = Some(GlobalHotKeyManager::new().map_err(|error| error.to_string())?);
            }
            let manager = registry.manager.as_ref().ok_or_else(|| "global shortcut manager unavailable".to_owned())?;
            let mut entries = Vec::new();
            for (id, command_id, chord, hotkey) in desired {
                match manager.register(hotkey) {
                    Ok(()) => entries.push((hotkey, command_id)),
                    Err(error) => tracing::warn!(%error, binding = %id, %chord, "system shortcut registration failed"),
                }
            }
            registry.entries = entries;
            registry.signature = signature;
            Ok(())
        })
    }

    fn request_stop(&self) {
        self.stop_requested.store(true, Ordering::Release);
        let _ = self.command_sender.send(None);
    }
}

fn unregister_all(registry: &mut Registry) {
    if let Some(manager) = registry.manager.as_ref() {
        for (hotkey, _) in registry.entries.drain(..) {
            if let Err(error) = manager.unregister(hotkey) {
                tracing::warn!(%error, "could not unregister global shortcut cleanly");
            }
        }
    } else {
        registry.entries.clear();
    }
}

/// Parse the settings-page chord notation into the `global-hotkey` key type.
fn parse_hotkey(chord: &str) -> Option<HotKey> {
    let mut modifiers = Modifiers::empty();
    let mut code = None;
    for part in chord
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "alt" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            "win" | "super" | "meta" => modifiers |= Modifiers::META,
            key => code = key_code(key),
        }
    }
    Some(HotKey::new(Some(modifiers), code?))
}

fn key_code(key: &str) -> Option<Code> {
    let code = match key {
        "a" => Code::KeyA,
        "b" => Code::KeyB,
        "c" => Code::KeyC,
        "d" => Code::KeyD,
        "e" => Code::KeyE,
        "f" => Code::KeyF,
        "g" => Code::KeyG,
        "h" => Code::KeyH,
        "i" => Code::KeyI,
        "j" => Code::KeyJ,
        "k" => Code::KeyK,
        "l" => Code::KeyL,
        "m" => Code::KeyM,
        "n" => Code::KeyN,
        "o" => Code::KeyO,
        "p" => Code::KeyP,
        "q" => Code::KeyQ,
        "r" => Code::KeyR,
        "s" => Code::KeyS,
        "t" => Code::KeyT,
        "u" => Code::KeyU,
        "v" => Code::KeyV,
        "w" => Code::KeyW,
        "x" => Code::KeyX,
        "y" => Code::KeyY,
        "z" => Code::KeyZ,
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "f1" => Code::F1,
        "f2" => Code::F2,
        "f3" => Code::F3,
        "f4" => Code::F4,
        "f5" => Code::F5,
        "f6" => Code::F6,
        "f7" => Code::F7,
        "f8" => Code::F8,
        "f9" => Code::F9,
        "f10" => Code::F10,
        "f11" => Code::F11,
        "f12" => Code::F12,
        "space" => Code::Space,
        "enter" => Code::Enter,
        "tab" => Code::Tab,
        "escape" | "esc" => Code::Escape,
        "backspace" => Code::Backspace,
        "delete" => Code::Delete,
        "home" => Code::Home,
        "end" => Code::End,
        "pageup" => Code::PageUp,
        "pagedown" => Code::PageDown,
        "arrowleft" | "left" => Code::ArrowLeft,
        "arrowright" | "right" => Code::ArrowRight,
        "arrowup" | "up" => Code::ArrowUp,
        "arrowdown" | "down" => Code::ArrowDown,
        _ => return None,
    };
    Some(code)
}

impl Service for ShortcutService {
    const NAME: &'static str = "photag/shortcuts";
}

#[derive(Debug, Clone, Copy)]
pub struct ShortcutsPlugin;

impl Plugin for ShortcutsPlugin {
    type Config = ();
    type Input = ();
    type PrepareError = std::convert::Infallible;
    type ApplyError = ShortcutPluginError;
    fn prepare(&self, _: ()) -> Result<Self::Input, Self::PrepareError> {
        Ok(())
    }
    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.shortcuts".into()
    }
    async fn apply(&self, context: Context, _: &Self::Input) -> Result<(), Self::ApplyError> {
        let bridge = context
            .try_service::<UiBridgeService>()
            .map_err(|error| ShortcutPluginError::Dependency(error.to_string()))?;
        let (command_sender, mut command_receiver) =
            tokio::sync::mpsc::unbounded_channel::<Option<String>>();
        let event_context = context.clone();
        tokio::spawn(async move {
            while let Some(Some(command_id)) = command_receiver.recv().await {
                if let Err(error) = event_context
                    .emit::<UiCommandInvoked>(Routing::Unscoped, command_id)
                    .await
                {
                    tracing::error!(%error, "global shortcut command dispatch failed");
                }
            }
        });
        let service = Arc::new(ShortcutService {
            catalog: Arc::clone(&bridge.catalog),
            command_sender,
            stop_requested: AtomicBool::new(false),
        });
        let _publication = context
            .provide(Arc::clone(&service))
            .map_err(|error| ShortcutPluginError::Publish(error.to_string()))?;
        let cleanup = Arc::clone(&service);
        context
            .effect_sync(move || cleanup.request_stop())
            .map_err(|error| ShortcutPluginError::Cleanup(error.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ShortcutPluginError {
    #[error("required shortcut dependency unavailable: {0}")]
    Dependency(String),
    #[error("could not publish shortcut service: {0}")]
    Publish(String),
    #[error("could not register shortcut cleanup: {0}")]
    Cleanup(String),
}
