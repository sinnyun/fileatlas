use std::collections::HashMap;
use std::path::{Path, PathBuf};

use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY};

pub(super) struct DiscoveredProgram {
    pub key: String,
    pub name: String,
    pub launch_path: PathBuf,
    pub uninstall_command: Option<String>,
    pub category_name: String,
    pub source: String,
    pub installed_on: Option<String>,
    pub size_bytes: Option<i64>,
}

struct InstalledProgram {
    installed_on: Option<String>,
    size_bytes: Option<i64>,
    name: String,
    launch_path: Option<PathBuf>,
    uninstall_command: Option<String>,
}

pub(super) fn discover() -> Result<Vec<DiscoveredProgram>, String> {
    let installed = installed_programs();
    let mut by_name: HashMap<String, InstalledProgram> = HashMap::new();
    for app in installed {
        let normalized = normalize_name(&app.name);
        if !normalized.is_empty() {
            by_name.entry(normalized).or_insert(app);
        }
    }
    let mut results = HashMap::<String, DiscoveredProgram>::new();
    for (path, name) in start_menu_shortcuts() {
        let normalized = normalize_name(&name);
        let matching = by_name.get(&normalized);
        let launch_path = matching
            .and_then(|item| item.launch_path.as_ref())
            .filter(|candidate| is_launch_target(candidate))
            .cloned()
            .unwrap_or(path.clone());
        let key = format!("path:{}", normalize_path(&launch_path));
        let (uninstall_command, display_name) = matching
            .map(|item| (item.uninstall_command.clone(), item.name.clone()))
            .unwrap_or((None, name));
        let category_name = classify_program(&display_name, &launch_path);
        results.entry(key.clone()).or_insert(DiscoveredProgram {
            key,
            name: display_name,
            launch_path,
            uninstall_command,
            category_name,
            installed_on: matching.and_then(|p| p.installed_on.clone()),
            size_bytes: matching.and_then(|p| p.size_bytes),
            source: if matching.is_some() {
                "installed"
            } else {
                "shortcut"
            }
            .into(),
        });
    }
    for item in by_name.into_values() {
        let Some(path) = item
            .launch_path
            .filter(|path| path.exists() && is_launch_target(path))
        else {
            continue;
        };
        let key = format!("path:{}", normalize_path(&path));
        let category_name = classify_program(&item.name, &path);
        results.entry(key.clone()).or_insert(DiscoveredProgram {
            key,
            name: item.name,
            launch_path: path,
            uninstall_command: item.uninstall_command,
            category_name,
            installed_on: item.installed_on,
            size_bytes: item.size_bytes,
            source: "installed".into(),
        });
    }
    let mut values = results.into_values().collect::<Vec<_>>();
    values.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(values)
}

fn classify_program(name: &str, path: &Path) -> String {
    let text = format!(
        "{} {}",
        name.to_lowercase(),
        path.to_string_lossy().to_lowercase()
    );
    let matches = |terms: &[&str]| terms.iter().any(|term| text.contains(term));
    let category = if matches(&[
        "visual studio",
        "vscode",
        "vs code",
        "jetbrains",
        "intellij",
        "android studio",
        "pycharm",
        "webstorm",
        "rider",
        "rustdesk",
        "python",
        "anaconda",
        "miniconda",
        "node.js",
        "nodejs",
        "git",
        "github desktop",
        "docker",
        "postman",
        "insomnia",
        "powershell",
        "terminal",
        "windows sdk",
        "developer command prompt",
        "开发",
        "编程",
    ]) {
        "开发工具"
    } else if matches(&[
        "photoshop",
        "illustrator",
        "indesign",
        "premiere",
        "after effects",
        "lightroom",
        "affinity",
        "figma",
        "blender",
        "autocad",
        "sketchup",
        "krita",
        "comfyui",
        "davinci resolve",
        "设计",
        "绘图",
        "剪映",
        "美图",
    ]) {
        "设计创作"
    } else if matches(&[
        "wechat",
        "weixin",
        "微信",
        "qq",
        "discord",
        "telegram",
        "slack",
        "teams",
        "钉钉",
        "飞书",
        "企业微信",
        "messenger",
    ]) {
        "通讯社交"
    } else if matches(&[
        "microsoft office",
        "office",
        "word",
        "excel",
        "powerpoint",
        "outlook",
        "onenote",
        "wps",
        "libreoffice",
        "acrobat",
        "foxit",
        "pdf",
        "notion",
        "obsidian",
        "文档",
        "办公",
    ]) {
        "办公软件"
    } else if matches(&[
        "chrome",
        "firefox",
        "edge",
        "browser",
        "浏览器",
        "vpn",
        "network",
        "网络",
    ]) {
        "网络浏览"
    } else if matches(&[
        "vlc",
        "potplayer",
        "foobar",
        "spotify",
        "itunes",
        "musicbee",
        "obs studio",
        "audacity",
        "media player",
        "播放器",
        "音乐",
        "视频",
    ]) {
        "影音播放"
    } else if matches(&[
        "steam",
        "epic games",
        "riot client",
        "battle.net",
        "ubisoft connect",
        "xbox",
        "game",
        "游戏",
    ]) {
        "游戏"
    } else if matches(&[
        "driver",
        "驱动",
        "update",
        "updater",
        "uninstall",
        "setup",
        "utility",
        "工具",
        "管理工具",
        "7-zip",
        "7zip",
        "defender",
        "antivirus",
        "nvidia",
        "amd software",
        "intel",
        "realtek",
        "device manager",
        "control panel",
    ]) {
        "系统工具"
    } else {
        "其他"
    };
    category.to_owned()
}

fn installed_programs() -> Vec<InstalledProgram> {
    let mut apps = Vec::new();
    let locations = [
        (
            HKEY_CURRENT_USER,
            "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        ),
        (
            HKEY_LOCAL_MACHINE,
            "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        ),
    ];
    for (hive, path) in locations {
        let root = RegKey::predef(hive);
        for flags in [KEY_READ, KEY_READ | KEY_WOW64_32KEY] {
            let Ok(key) = root.open_subkey_with_flags(path, flags) else {
                continue;
            };
            for child in key.enum_keys().flatten() {
                let Ok(item) = key.open_subkey_with_flags(child, flags) else {
                    continue;
                };
                let Ok(name) = item.get_value::<String, _>("DisplayName") else {
                    continue;
                };
                if name.trim().is_empty()
                    || item.get_value::<u32, _>("SystemComponent").unwrap_or(0) == 1
                {
                    continue;
                }
                let uninstall_command = item
                    .get_value::<String, _>("QuietUninstallString")
                    .ok()
                    .or_else(|| item.get_value::<String, _>("UninstallString").ok());
                let launch_path = item
                    .get_value::<String, _>("DisplayIcon")
                    .ok()
                    .and_then(|value| icon_executable(&value))
                    .or_else(|| {
                        item.get_value::<String, _>("InstallLocation")
                            .ok()
                            .map(PathBuf::from)
                            .filter(|path| path.is_file())
                    });
                let installed_on =
                    item.get_value::<String, _>("InstallDate")
                        .ok()
                        .and_then(|date| {
                            let parsed =
                                chrono::NaiveDate::parse_from_str(date.trim(), "%Y%m%d").ok()?;
                            Some(parsed.format("%Y-%m-%d").to_string())
                        });
                let size_bytes = item
                    .get_value::<u32, _>("EstimatedSize")
                    .ok()
                    .filter(|size| *size > 0)
                    .map(|size| i64::from(size) * 1024);
                apps.push(InstalledProgram {
                    installed_on,
                    size_bytes,
                    name,
                    launch_path,
                    uninstall_command,
                });
            }
        }
    }
    apps
}

fn icon_executable(value: &str) -> Option<PathBuf> {
    let value = value.trim().trim_matches('"');
    let path = if let Some((path, index)) = value.rsplit_once(',') {
        if index.trim().parse::<i32>().is_ok() {
            path.trim().trim_matches('"')
        } else {
            value
        }
    } else {
        value
    };
    let path = expand_environment(path);
    let path = PathBuf::from(path);
    (path.is_file() && is_launch_target(&path)).then_some(path)
}

fn is_launch_target(path: &Path) -> bool {
    path.extension().is_some_and(|extension| {
        ["exe", "com", "bat", "cmd", "lnk"]
            .iter()
            .any(|supported| extension.eq_ignore_ascii_case(supported))
    })
}

fn expand_environment(input: &str) -> String {
    let mut output = String::new();
    let mut rest = input;
    while let Some(start) = rest.find('%') {
        output.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('%') else {
            output.push_str(&rest[start..]);
            return output;
        };
        let key = &after[..end];
        if let Some(value) = std::env::var_os(key) {
            output.push_str(&value.to_string_lossy());
        } else {
            output.push('%');
            output.push_str(key);
            output.push('%');
        }
        rest = &after[end + 1..];
    }
    output.push_str(rest);
    output
}

fn start_menu_shortcuts() -> Vec<(PathBuf, String)> {
    let mut roots = Vec::new();
    if let Some(appdata) = std::env::var_os("APPDATA") {
        roots.push(PathBuf::from(appdata).join("Microsoft\\Windows\\Start Menu\\Programs"));
    }
    if let Some(program_data) = std::env::var_os("ProgramData") {
        roots.push(PathBuf::from(program_data).join("Microsoft\\Windows\\Start Menu\\Programs"));
    }
    let mut output = Vec::new();
    for root in roots {
        collect_shortcuts(&root, 0, &mut output);
    }
    output
}

fn collect_shortcuts(directory: &Path, depth: usize, output: &mut Vec<(PathBuf, String)>) {
    if depth > 8 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_shortcuts(&path, depth + 1, output);
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
        {
            if let Some(name) = path
                .file_stem()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
            {
                output.push((path, name));
            }
        }
    }
}

fn normalize_name(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\").to_lowercase()
}
