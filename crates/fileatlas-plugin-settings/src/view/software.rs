use super::*;
use fileatlas_ui_bridge::components::WorkbenchStyle as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use gpui_kit::component::button::Button;
use std::path::PathBuf;

#[derive(Clone, serde::Serialize)]
pub(super) struct SoftwareStatus {
    pub name: &'static str,
    pub purpose: &'static str,
    pub state: &'static str,
    pub version: Option<String>,
    pub path: Option<String>,
    pub project_url: &'static str,
    pub download_url: &'static str,
    pub github_url: Option<&'static str>,
    pub github_note: Option<&'static str>,
}

impl SettingsView {
    pub(super) fn poll_software_check(&mut self) -> bool {
        let tab = self.surfaces.value("settings.tab");
        let active = self.surfaces.is_open("settings")
            && (tab.as_deref() == Some("software")
                || (tab.as_deref() == Some("plugin")
                    && self.surfaces.value("settings.plugin_id").as_deref()
                        == Some("fileatlas.screenshot")));
        if !active {
            self.software_tab_active = false;
            return false;
        }
        if !self.software_tab_active {
            self.software_tab_active = true;
            self.software_revision = self.software_revision.wrapping_add(1);
        }
        let revision = self.software_revision;
        let shotoshot_path = self
            .service
            .snapshot()
            .ok()
            .and_then(|settings| settings.screenshot_tool.executable);
        let changed = self
            .software_check
            .update(revision, || Ok(check_software(shotoshot_path)));
        if changed {
            let value = match self.software_check.value() {
                Some(Ok(software)) => {
                    serde_json::json!({"state":"ready", "revision":revision, "software":software})
                }
                Some(Err(error)) => serde_json::json!({"state":"failed", "error":error}),
                None => serde_json::json!({"state":"loading", "revision":revision}),
            };
            let _ = self
                .surfaces
                .set_value("settings.software-check", &value.to_string());
        }
        changed
    }

    pub(super) fn render_software_page(
        &self,
        cx: &mut Context<Self>,
        screenshot_plugin: bool,
    ) -> impl IntoElement {
        let mut content = div()
            .v_flex()
            .gap(px(fileatlas_ui_bridge::theme::space_expanded()));
        if !screenshot_plugin {
            content = content.child(
                div()
                    .h_flex()
                    .justify_end()
                    .child(self.software_refresh_button(cx)),
            );
        }

        match self.software_check.value() {
            Some(Ok(items)) => {
                for item in items.iter().filter(|item| {
                    if screenshot_plugin {
                        item.name == "Shotoshot"
                    } else {
                        matches!(item.name, "QuickLook" | "Everything")
                    }
                }) {
                    let state_label = match item.state {
                        "installed" => "Installed",
                        "unsupported" => "Detection unavailable",
                        _ => "Not detected",
                    };
                    let mut details = div()
                        .v_flex()
                        .gap(px(fileatlas_ui_bridge::theme::space_standard()));
                    details = details.child(
                        div()
                            .h_flex()
                            .items_center()
                            .justify_between()
                            .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
                            .child(
                                div()
                                    .workbench_title()
                                    .child(fileatlas_ui_bridge::i18n::tr(item.name)),
                            )
                            .child(
                                div()
                                    .workbench_assistant()
                                    .child(fileatlas_ui_bridge::i18n::tr(state_label)),
                            ),
                    );
                    details = details.child(
                        div()
                            .workbench_assistant()
                            .child(fileatlas_ui_bridge::i18n::tr(item.purpose)),
                    );
                    if let Some(version) = &item.version {
                        details = details.child(div().workbench_assistant().child(format!(
                            "{}: {version}",
                            fileatlas_ui_bridge::i18n::tr("Version")
                        )));
                    }
                    if let Some(path) = &item.path {
                        details = details.child(div().workbench_assistant().child(format!(
                            "{}: {path}",
                            fileatlas_ui_bridge::i18n::tr("Location")
                        )));
                    }
                    let project_url = item.project_url;
                    let download_url = item.download_url;
                    let github_url = item.github_url;
                    let mut links = div()
                        .h_flex()
                        .items_center()
                        .gap(px(fileatlas_ui_bridge::theme::space_standard()));
                    links = links.child(link_button(
                        format!("settings-software-{}-project", item.name),
                        "Project page",
                        project_url,
                        cx,
                    ));
                    links = links.child(link_button(
                        format!("settings-software-{}-download", item.name),
                        "Official download",
                        download_url,
                        cx,
                    ));
                    if let Some(url) = github_url {
                        links = links.child(link_button(
                            format!("settings-software-{}-github", item.name),
                            "GitHub",
                            url,
                            cx,
                        ));
                    }
                    details = details.child(links);
                    if let Some(note) = item.github_note
                        && (item.name != "Shotoshot" || item.state != "installed")
                    {
                        details = details.child(
                            div()
                                .workbench_assistant()
                                .child(fileatlas_ui_bridge::i18n::tr(note)),
                        );
                    }
                    content = content.child(
                        div()
                            .w_full()
                            .py(px(theme::space_expanded()))
                            .workbench_separator(
                                fileatlas_ui_bridge::components::DividerEdge::Bottom,
                            )
                            .child(details),
                    );
                }
            }
            Some(Err(error)) => {
                content = content.child(div().workbench_assistant().child(error.clone()));
            }
            None => {
                content = content.child(div().workbench_assistant().child(
                    fileatlas_ui_bridge::i18n::tr("Checking installed software…"),
                ));
            }
        }
        if !screenshot_plugin {
            content=content.child(div().workbench_assistant()
                .child(fileatlas_ui_bridge::i18n::tr("This page only reads installation records and checks common paths. It does not install or start listed software.")));
        }
        content
    }
    pub(super) fn software_refresh_button(&self, cx: &mut Context<Self>) -> Button {
        fileatlas_ui_bridge::components::button("settings-software-refresh")
            .outline()
            .icon(theme::small_icon(gpui_kit::assets::IconName::RefreshCw))
            .label(fileatlas_ui_bridge::i18n::tr("Refresh"))
            .on_click(cx.listener(|this, _, _, cx| {
                this.software_revision = this.software_revision.wrapping_add(1);
                cx.notify();
            }))
    }
}

fn link_button(
    id: String,
    label: &'static str,
    url: &'static str,
    cx: &mut Context<SettingsView>,
) -> Button {
    fileatlas_ui_bridge::components::button(id)
        .outline()
        .label(fileatlas_ui_bridge::i18n::tr(label))
        .on_click(cx.listener(move |_, _, _, _| {
            let _ = open::that(url);
        }))
}

fn check_software(shotoshot_path: Option<PathBuf>) -> Vec<SoftwareStatus> {
    let mut software = vec![
        software_status(
            "QuickLook",
            "Used by FileAtlas to view original files.",
            "QuickLook.exe",
            &["QuickLook", "QuickLook_is1", "QL-Win_QuickLook_is1"],
            "https://github.com/QL-Win/QuickLook",
            "https://github.com/QL-Win/QuickLook/releases",
            Some("https://github.com/QL-Win/QuickLook"),
            None,
        ),
        software_status(
            "Everything",
            "Requires Everything 1.5. FileAtlas prepares its background client when searching filenames and paths.",
            "Everything.exe",
            &["Everything", "Everything_is1"],
            "https://www.voidtools.com/",
            "https://www.voidtools.com/downloads/",
            Some("https://github.com/voidtools/ES"),
            Some(
                "GitHub links to the official ES command-line tool, not the Everything application.",
            ),
        ),
    ];
    let shotoshot_installed = shotoshot_path.as_ref().is_some_and(|path| path.is_file());
    software.push(SoftwareStatus {
        name: "Shotoshot",
        purpose: "FileAtlas uses Shotoshot for screenshot capture and annotation.",
        state: if shotoshot_installed {
            "installed"
        } else {
            "missing"
        },
        version: None,
        path: shotoshot_path.map(|path| path.display().to_string()),
        project_url: "https://github.com/Chmyke/shotoshot",
        download_url: "https://shotoshot.com/",
        github_url: Some("https://github.com/Chmyke/shotoshot"),
        github_note: Some(
            "Install Shotoshot, then select shotoshot.exe in FileAtlas's Screenshot Tool settings.",
        ),
    });
    software
}

fn software_status(
    name: &'static str,
    purpose: &'static str,
    executable: &'static str,
    uninstall_keys: &[&str],
    project_url: &'static str,
    download_url: &'static str,
    github_url: Option<&'static str>,
    github_note: Option<&'static str>,
) -> SoftwareStatus {
    #[cfg(windows)]
    let detected = detect_windows(executable, uninstall_keys);
    #[cfg(not(windows))]
    let detected: Option<(Option<String>, Option<PathBuf>)> = None;
    #[cfg(not(windows))]
    let state = "unsupported";
    #[cfg(windows)]
    let state = if detected.is_some() {
        "installed"
    } else {
        "missing"
    };
    let (version, path) = detected
        .map(|(version, path)| (version, path.map(|value| value.display().to_string())))
        .unwrap_or((None, None));
    SoftwareStatus {
        name,
        purpose,
        state,
        version,
        path,
        project_url,
        download_url,
        github_url,
        github_note,
    }
}

#[cfg(windows)]
fn detect_windows(
    executable: &str,
    uninstall_keys: &[&str],
) -> Option<(Option<String>, Option<PathBuf>)> {
    let roots = [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE];
    let views = [0, RRF_SUBKEY_WOW6464KEY, RRF_SUBKEY_WOW6432KEY];
    for root in roots {
        for view in views {
            for subkey in uninstall_keys {
                let key =
                    format!("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{subkey}");
                let version = registry_string(root, &key, Some("DisplayVersion"), view);
                let location = registry_string(root, &key, Some("InstallLocation"), view)
                    .map(|value| PathBuf::from(expand_environment(&value)));
                let icon = registry_string(root, &key, Some("DisplayIcon"), view)
                    .and_then(|value| icon_path(&value));
                let candidate = icon.filter(|path| path.is_file()).or_else(|| {
                    location
                        .map(|path| path.join(executable))
                        .filter(|path| path.is_file())
                });
                if let Some(path) = candidate {
                    return Some((version, Some(path)));
                }
            }
            let app_path_key =
                format!("Software\\Microsoft\\Windows\\CurrentVersion\\App Paths\\{executable}");
            if let Some(value) = registry_string(root, &app_path_key, None, view) {
                let path = PathBuf::from(expand_environment(value.trim().trim_matches('"')));
                if path.is_file() {
                    return Some((None, Some(path)));
                }
            }
        }
    }
    for variable in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
        if let Ok(root) = std::env::var(variable) {
            let folders: &[&str] = if executable.eq_ignore_ascii_case("QuickLook.exe") {
                &["Programs\\QuickLook", "QuickLook"]
            } else {
                &["Everything"]
            };
            for folder in folders {
                let path = PathBuf::from(&root).join(folder).join(executable);
                if path.is_file() {
                    return Some((None, Some(path)));
                }
            }
        }
    }
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|directory| directory.join(executable))
        .find(|path| path.is_file())
        .map(|path| (None, Some(path)))
}

#[cfg(windows)]
type Hkey = isize;
#[cfg(windows)]
const HKEY_CURRENT_USER: Hkey = 0x8000_0001u32 as i32 as isize;
#[cfg(windows)]
const HKEY_LOCAL_MACHINE: Hkey = 0x8000_0002u32 as i32 as isize;
#[cfg(windows)]
const RRF_RT_ANY: u32 = 0x0000_ffff;
#[cfg(windows)]
const RRF_SUBKEY_WOW6464KEY: u32 = 0x0001_0000;
#[cfg(windows)]
const RRF_SUBKEY_WOW6432KEY: u32 = 0x0002_0000;

#[cfg(windows)]
#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegGetValueW(
        key: Hkey,
        subkey: *const u16,
        value: *const u16,
        flags: u32,
        value_type: *mut u32,
        data: *mut std::ffi::c_void,
        size: *mut u32,
    ) -> i32;
}

#[cfg(windows)]
fn registry_string(root: Hkey, key: &str, value: Option<&str>, view: u32) -> Option<String> {
    let key = key.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let value = value.map(|name| name.encode_utf16().chain(Some(0)).collect::<Vec<_>>());
    let value_ptr = value
        .as_ref()
        .map_or(std::ptr::null(), |name| name.as_ptr());
    let flags = RRF_RT_ANY | view;
    let mut value_type = 0;
    let mut size = 0;
    let first = unsafe {
        RegGetValueW(
            root,
            key.as_ptr(),
            value_ptr,
            flags,
            &mut value_type,
            std::ptr::null_mut(),
            &mut size,
        )
    };
    if first != 0 || size < 2 || size > 64 * 1024 {
        return None;
    }
    let mut buffer = vec![0u16; size.div_ceil(2) as usize];
    let mut capacity = (buffer.len() * 2) as u32;
    let result = unsafe {
        RegGetValueW(
            root,
            key.as_ptr(),
            value_ptr,
            flags,
            &mut value_type,
            buffer.as_mut_ptr().cast(),
            &mut capacity,
        )
    };
    if result != 0 {
        return None;
    }
    Some(
        String::from_utf16_lossy(&buffer)
            .trim_end_matches('\0')
            .to_owned(),
    )
}

#[cfg(windows)]
fn icon_path(value: &str) -> Option<PathBuf> {
    let mut path = value.trim().trim_matches('"').to_owned();
    if let Some((candidate, index)) = path.rsplit_once(',')
        && index.trim().parse::<i32>().is_ok()
    {
        path = candidate.trim_matches('"').to_owned();
    }
    (!path.is_empty()).then(|| PathBuf::from(expand_environment(&path)))
}

#[cfg(windows)]
fn expand_environment(value: &str) -> String {
    let mut expanded = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('%') {
        expanded.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('%') else {
            expanded.push_str(&rest[start..]);
            return expanded;
        };
        let key = &after[..end];
        let replacement = std::env::vars_os()
            .find(|(name, _)| name.to_string_lossy().eq_ignore_ascii_case(key))
            .map(|(_, value)| value.to_string_lossy().into_owned());
        if let Some(replacement) = replacement {
            expanded.push_str(&replacement);
        } else {
            expanded.push('%');
            expanded.push_str(key);
            expanded.push('%');
        }
        rest = &after[end + 1..];
    }
    expanded.push_str(rest);
    expanded
}
