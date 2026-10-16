//! File-version presentation. The explicit isolated demo owns an in-memory
//! graph; production still has no snapshot backend or restore operation.
use super::*;
use gpui_kit::prelude::FluentBuilder;

#[derive(Clone)]
pub(super) struct Version {
    pub label: String,
    pub branch: bool,
    pub modified: i64,
    pub size: i64,
    pub dimensions: String,
    pub thumbnail: Option<PathBuf>,
}

pub(crate) struct DemoHistory {
    pub(super) versions: Vec<Version>,
    pub(super) selected: usize,
    pub(super) active: usize,
    pub(super) filter: u8,
}

impl DemoHistory {
    pub(crate) fn publish(&self, surfaces: &photag_ui_bridge::UiSurfaceState) {
        for (key, value) in [
            ("enabled", "true".to_string()),
            ("selected", self.versions[self.selected].label.clone()),
            ("active", self.versions[self.active].label.clone()),
            ("count", self.versions.len().to_string()),
            ("filter", self.filter.to_string()),
        ] {
            let _ = surfaces.set_value(&format!("gallery.inspector.demo-{key}"), &value);
        }
    }
    pub(crate) fn from_environment() -> Option<Self> {
        let enabled = std::env::var("PHOTAG_DEMO_INSPECTOR").ok().as_deref() == Some("1")
            && std::env::var("PHOTAG_DEV_PLUGINS")
                .is_ok_and(|v| v.split(',').any(|v| v.trim() == "dev-test"))
            && std::env::var_os("PHOTAG_DEV_TEST_DIR").is_some();
        if !enabled {
            return None;
        }
        let paths = std::env::var("PHOTAG_DEMO_HISTORY_THUMBS")
            .ok()
            .or_else(|| std::env::var("PHOTAG_DEMO_HISTORY_THUMB").ok())
            .unwrap_or_default()
            .split(';')
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        let versions = [
            ("v1", false, 1792119180, 32665, "320 × 200"),
            ("v0", false, 1792111200, 28672, "320 × 200"),
            ("v0.1", true, 1792110300, 25190, "320 × 200"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (label, branch, modified, size, dimensions))| Version {
            label: label.into(),
            branch,
            modified,
            size,
            dimensions: dimensions.into(),
            thumbnail: paths.get(i).or(paths.first()).cloned(),
        })
        .collect();
        Some(Self {
            versions,
            selected: 0,
            active: 0,
            filter: 0,
        })
    }
}

impl GalleryView {
    pub(super) fn inspector_history(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(media) = self.inspector_media() else {
            return inspector_message("Select one file to view its version history.");
        };
        let demo = self.inspector_history_demo.as_ref();
        let versions = demo.map(|d| d.versions.clone()).unwrap_or_else(|| {
            vec![Version {
                label: photag_ui_bridge::i18n::tr("Working copy"),
                branch: false,
                modified: media.modified_at,
                size: media.file_size,
                dimensions: dimensions(&media),
                thumbnail: media.thumb_path.as_ref().map(PathBuf::from),
            }]
        });
        let selected = demo.map_or(0, |d| d.selected).min(versions.len() - 1);
        let active = demo.map_or(0, |d| d.active);
        let filter = demo.map_or(0, |d| d.filter);
        let selected_version = &versions[selected];
        let weak = cx.entity().downgrade();
        let branch = Button::new("inspector-history-branch")
            .outline()
            .w_full()
            .h(px(36.))
            .label(photag_ui_bridge::i18n::tr(match filter {
                1 => "Main branch",
                2 => "Alternate branch",
                _ => "All branches",
            }))
            .icon(theme::icon(IconName::ChevronDown))
            .disabled(demo.is_none())
            .dropdown_menu(move |mut menu, _, _| {
                for (filter, label) in [
                    (0, "All branches"),
                    (1, "Main branch"),
                    (2, "Alternate branch"),
                ] {
                    let weak = weak.clone();
                    menu = menu.item(
                        PopupMenuItem::new(photag_ui_bridge::i18n::tr(label)).on_click(
                            move |_, _, cx| {
                                let _ = weak.update(cx, |view, cx| {
                                    view.demo_history_action("filter", filter, cx)
                                });
                            },
                        ),
                    );
                }
                menu
            });
        div()
            .v_flex()
            .w_full()
            .min_w_0()
            .gap(px(9.))
            .p(px(16.))
            .child(
                div()
                    .h_flex()
                    .w_full()
                    .gap(px(10.))
                    .child(
                        Button::new("inspector-history-create")
                            .primary()
                            .flex_1()
                            .h(px(36.))
                            .icon(theme::icon(IconName::Plus))
                            .label(photag_ui_bridge::i18n::tr("Create version"))
                            .disabled(demo.is_none() || versions.len() >= 16)
                            .tooltip(photag_ui_bridge::i18n::tr(if demo.is_some() {
                                "Preview sample versions without changing files."
                            } else {
                                "Version snapshots are not connected yet."
                            }))
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.demo_history_action("create", 0, cx)
                            })),
                    )
                    .child(
                        Button::new("inspector-history-switch")
                            .outline()
                            .flex_1()
                            .h(px(36.))
                            .label(photag_ui_bridge::i18n::tr("Switch version"))
                            .disabled(demo.is_none() || selected == active)
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.demo_history_action("switch", 0, cx)
                            })),
                    ),
            )
            .child(branch)
            .child(div().h(px(1.)).w_full().bg(rgba(theme::SEPARATOR_LIGHT)))
            .child(history_tree::render(
                &versions,
                selected,
                active,
                filter,
                demo.is_some(),
                cx,
            ))
            .child(
                well()
                    .min_h(px(60.))
                    .items_center()
                    .justify_center()
                    .child(
                        theme::icon(IconName::Plus)
                            .size(px(23.))
                            .text_color(rgb(theme::TEXT_MUTED)),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child(photag_ui_bridge::i18n::tr(if versions.len() > 1 {
                                "No more versions"
                            } else {
                                "No earlier versions"
                            })),
                    ),
            )
            .child(div().h(px(1.)).w_full().bg(rgba(theme::SEPARATOR_LIGHT)))
            .child(
                well()
                    .gap(px(7.))
                    .child(section_title("Selected version"))
                    .child(
                        div()
                            .h_flex()
                            .w_full()
                            .min_w_0()
                            .items_start()
                            .gap(px(10.))
                            .child(history_tree::thumbnail(
                                selected_version.thumbnail.clone(),
                                72.,
                                54.,
                            ))
                            .child(
                                div()
                                    .v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .child(info_row("Version", selected_version.label.clone()))
                                    .child(info_row(
                                        "Modified",
                                        card::format_timestamp(selected_version.modified),
                                    ))
                                    .child(info_row(
                                        "File size",
                                        card::format_bytes(selected_version.size),
                                    ))
                                    .child(info_row(
                                        "Image dimensions",
                                        selected_version.dimensions.clone(),
                                    )),
                            ),
                    ),
            )
            .when(demo.is_some(), |body| {
                body.child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(theme::TEXT_MUTED))
                        .child(photag_ui_bridge::i18n::tr(
                            "Sample versions · changes stay in this preview.",
                        )),
                )
            })
            .into_any_element()
    }

    pub(super) fn demo_history_action(
        &mut self,
        action: &str,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(demo) = self.inspector_history_demo.as_mut() else {
            return;
        };
        let changed = match action {
            "select" if index < demo.versions.len() && demo.selected != index => {
                demo.selected = index;
                true
            }
            "switch" if demo.active != demo.selected => {
                demo.active = demo.selected;
                true
            }
            "filter" if index <= 2 && demo.filter != index as u8 => {
                demo.filter = index as u8;
                if index != 0 && demo.versions[demo.selected].branch != (index == 2) {
                    if let Some(selected) =
                        demo.versions.iter().position(|v| v.branch == (index == 2))
                    {
                        demo.selected = selected;
                    }
                }
                true
            }
            "create" if demo.versions.len() < 16 => {
                let mut version = demo.versions[demo.active].clone();
                version.label = format!("v{}", demo.versions.len() - 1);
                version.modified = chrono::Utc::now().timestamp();
                version.branch = false;
                demo.versions.insert(0, version);
                demo.selected = 0;
                demo.active = 0;
                demo.filter = 0;
                true
            }
            _ => false,
        };
        if changed {
            let _ = self.surfaces.set_value(
                "gallery.inspector.demo-selected",
                &demo.versions[demo.selected].label,
            );
            let _ = self.surfaces.set_value(
                "gallery.inspector.demo-active",
                &demo.versions[demo.active].label,
            );
            let _ = self.surfaces.set_value(
                "gallery.inspector.demo-count",
                &demo.versions.len().to_string(),
            );
            let _ = self
                .surfaces
                .set_value("gallery.inspector.demo-filter", &demo.filter.to_string());
            cx.notify();
        }
    }
}

fn dimensions(media: &photag_plugin_database::db::queries::MediaItem) -> String {
    media
        .width
        .zip(media.height)
        .map(|(w, h)| format!("{w} × {h}"))
        .unwrap_or_else(|| "—".into())
}
