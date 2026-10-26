use crate::extensions::CanvasExtensionService;
use crate::{
    PANEL, WhiteboardService, hooks,
    images::{ImageCache, ImageCacheStats},
    store,
};
use fileatlas_canvas_engine::{
    Element as CanvasElement, ElementKind, Tool, ToolGroup, WhiteboardStyle, WhiteboardView,
};
use fileatlas_contracts::canvas::CanvasToolIcon;
use fileatlas_ui_bridge::{
    components::{
        self, ForegroundRole, SurfaceRole, WorkbenchForeground as _, WorkbenchPaint as _,
        WorkbenchStyle as _, WorkbenchTypography as _,
    },
    panel_catalog::PanelCatalog,
    refresh,
};
use gpui_kit::{
    component::{input::InputState, scroll::ScrollableElement},
    prelude::FluentBuilder as _,
    *,
};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

mod sidebars;

pub(super) struct Rail {
    pub panels: Arc<PanelCatalog>,
}
impl Render for Rail {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        components::rail_shortcut(
            "whiteboard-open",
            "画布",
            fileatlas_ui_bridge::theme::icon(gpui_kit::assets::IconName::Square),
            false,
        )
        .on_click(cx.listener(|this, _, _, _| {
            if let Err(e) = this.panels.open(PANEL) {
                tracing::error!(%e, "whiteboard panel open failed");
            }
        }))
    }
}
pub(super) fn panel(
    service: Arc<WhiteboardService>,
    thumbnails: Arc<fileatlas_plugin_thumbnails::ThumbnailService>,
    resources: Arc<fileatlas_plugin_media_resources::MediaResourceService>,
    quicklook: Arc<fileatlas_plugin_quicklook::QuickLookService>,
    eagle_links: Option<Arc<fileatlas_plugin_eagle_links::EagleLinkService>>,
    extensions: Arc<CanvasExtensionService>,
    image_cache_stats: Arc<Mutex<ImageCacheStats>>,
    window: &mut Window,
    app: &mut App,
) -> AnyView {
    app.new(|cx| {
        let images = Rc::new(RefCell::new(ImageCache::new(
            Arc::clone(&thumbnails),
            Arc::clone(&resources),
            Arc::clone(&service),
            Arc::clone(&image_cache_stats),
        )));
        let image_wake = images.borrow().wake.clone();
        let tool_wake = extensions.wake();
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("画板名称"));
        let wake = refresh::listen(cx, service.wake(), |view: &mut Host, cx| {
            view.sync(cx);
            true
        });
        let image_task = refresh::listen(cx, image_wake, |view: &mut Host, cx| {
            let changed = view.images.borrow_mut().poll();
            if changed {
                if let Some(board) = &view.board {
                    board.update(cx, |_, cx| cx.notify());
                }
            }
            let transitioning = view.images.borrow().has_active_transitions();
            let downgrade_pending = view.images.borrow().has_pending_downgrades();
            if (transitioning || downgrade_pending) && !view.image_poll_active {
                view.image_poll_active = true;
                refresh::poll(cx, std::time::Duration::from_millis(16), |view, cx| {
                    let changed = view.images.borrow_mut().poll();
                    if changed {
                        if let Some(board) = &view.board {
                            board.update(cx, |_, cx| cx.notify());
                        }
                    }
                    let cache = view.images.borrow();
                    let active = cache.has_active_transitions() || cache.has_pending_downgrades();
                    drop(cache);
                    if !active {
                        view.image_poll_active = false;
                    }
                    refresh::PollResult::until(changed, active)
                });
            }
            let has_cached_originals = view.images.borrow().has_cached_originals();
            if has_cached_originals && !view.image_cache_poll_active {
                view.image_cache_poll_active = true;
                refresh::poll(cx, std::time::Duration::from_secs(1), |view, cx| {
                    let changed = view.images.borrow_mut().poll();
                    if changed {
                        if let Some(board) = &view.board {
                            board.update(cx, |_, cx| cx.notify());
                        }
                    }
                    let active = view.images.borrow().has_cached_originals();
                    if !active {
                        view.image_cache_poll_active = false;
                    }
                    refresh::PollResult::until(changed, active)
                });
            }
            changed
        });
        let tool_task = refresh::listen(cx, tool_wake, |_: &mut Host, _| true);
        let mut host = Host {
            service,
            thumbnails,
            resources,
            quicklook,
            eagle_links,
            extensions,
            name,
            board: None,
            active_id: None,
            left_panel: None,
            right_panel: None,
            images,
            image_poll_active: false,
            image_cache_poll_active: false,
            message: None,
            _wake: wake,
            _images: image_task,
            _tools: tool_task,
        };
        host.sync(cx);
        host
    })
    .into()
}
pub(super) struct Host {
    pub service: Arc<WhiteboardService>,
    pub thumbnails: Arc<fileatlas_plugin_thumbnails::ThumbnailService>,
    pub resources: Arc<fileatlas_plugin_media_resources::MediaResourceService>,
    pub quicklook: Arc<fileatlas_plugin_quicklook::QuickLookService>,
    pub eagle_links: Option<Arc<fileatlas_plugin_eagle_links::EagleLinkService>>,
    pub extensions: Arc<CanvasExtensionService>,
    pub name: Entity<InputState>,
    pub board: Option<Entity<WhiteboardView>>,
    pub active_id: Option<i64>,
    left_panel: Option<sidebars::LeftPanel>,
    right_panel: Option<sidebars::RightPanel>,
    pub images: Rc<RefCell<ImageCache>>,
    image_poll_active: bool,
    image_cache_poll_active: bool,
    pub message: Option<String>,
    _wake: gpui::Task<()>,
    _images: gpui::Task<()>,
    _tools: gpui::Task<()>,
}
impl Host {
    fn toggle_left_panel(&mut self, panel: sidebars::LeftPanel, cx: &mut Context<Self>) {
        self.left_panel = (self.left_panel != Some(panel)).then_some(panel);
        cx.notify();
    }
    fn toggle_right_panel(&mut self, panel: sidebars::RightPanel, cx: &mut Context<Self>) {
        self.right_panel = (self.right_panel != Some(panel)).then_some(panel);
        cx.notify();
    }
    pub fn sync(&mut self, cx: &mut Context<Self>) {
        let library = self.service.snapshot();
        if self.active_id != library.active_id
            || (self.board.is_none() && library.active_id.is_some())
        {
            self.active_id = library.active_id;
            let record = library
                .active_id
                .and_then(|id| library.boards.iter().find(|b| b.id == id))
                .cloned();
            if let Some(record) = &record {
                self.images
                    .borrow_mut()
                    .set_board(record.id, &record.image_previews);
            } else {
                self.images.borrow_mut().clear();
            }
            self.board = record.map(|record| {
                let scene = serde_json::from_str::<fileatlas_canvas_engine::Scene>(&record.scene)
                    .unwrap_or_default();
                let mut view =
                    cx.new(|cx| WhiteboardView::new_embedded(scene, Rc::new(native_style), cx));
                hooks::install(
                    &mut view,
                    record.id,
                    cx.entity().downgrade(),
                    Arc::clone(&self.service),
                    Rc::clone(&self.images),
                    cx,
                );
                if let Some(font_path) = &record.font_path {
                    self.load_font(view.clone(), record.id, font_path.clone(), cx);
                }
                view
            });
        }
        if let Some(board) = &self.board {
            let templates = self.service.templates();
            if let Some(record) = self
                .active_id
                .and_then(|id| library.boards.iter().find(|b| b.id == id))
            {
                board.update(cx, |v, cx| {
                    v.set_templates(templates, cx);
                    v.set_saved_colors(library.colors, cx);
                    v.set_toolbar_pos(record.toolbar_pos, cx);
                    v.set_toolbar_vertical(record.toolbar_vertical, cx);
                });
            }
        }
        for (id, paths, at) in self.service.take_images() {
            if self.active_id == Some(id) {
                if let Some(target) = self.board.clone() {
                    let [x, y] = at.unwrap_or_else(|| target.read(cx).viewport_center());
                    self.place_images(id, target, Some(paths), x, y, cx);
                }
            } else {
                self.message = Some("图片添加取消：目标画板已经切换".into());
            }
        }
        for input in self.service.take_canvas_scrolls() {
            if self.active_id != Some(input.board_id) {
                continue;
            }
            if let Some(board) = self.board.clone() {
                let scene = board.update(cx, |view, cx| {
                    view.scroll_canvas(input.dx, input.dy, input.control, input.at, cx);
                    view.scene().to_json()
                });
                if let Err(error) = self.service.save_scene(input.board_id, scene) {
                    tracing::error!(%error, board_id = input.board_id, "MCP canvas wheel save failed");
                }
            }
        }
        for action in self.service.take_panel_actions() {
            match action {
                store::CanvasPanelAction::LeftBoards => {
                    self.left_panel = Some(sidebars::LeftPanel::Boards)
                }
                store::CanvasPanelAction::LeftResources => {
                    self.left_panel = Some(sidebars::LeftPanel::Resources)
                }
                store::CanvasPanelAction::LeftClose => self.left_panel = None,
                store::CanvasPanelAction::RightLayers => {
                    self.right_panel = Some(sidebars::RightPanel::Layers)
                }
                store::CanvasPanelAction::RightObject => {
                    self.right_panel = Some(sidebars::RightPanel::Object)
                }
                store::CanvasPanelAction::RightClose => self.right_panel = None,
            }
            cx.notify();
        }
        for action in self.service.take_tool_group_actions() {
            if let Some(board) = self.board.clone() {
                board.update(cx, |canvas, cx| canvas.toggle_tool_group(action.group, cx));
            }
        }
        for action in self.service.take_toolbar_menu_actions() {
            if let Some(board) = self.board.clone() {
                board.update(cx, |canvas, cx| match action {
                    store::CanvasToolbarMenuAction::Color => canvas.toggle_color_picker(cx),
                    store::CanvasToolbarMenuAction::Font => canvas.toggle_font_picker(cx),
                    store::CanvasToolbarMenuAction::Templates => canvas.toggle_template_gallery(cx),
                });
            }
        }
    }
    pub fn result(&mut self, result: store::Result<()>, cx: &mut Context<Self>) {
        self.message = result.err();
        cx.notify();
    }
    fn create(&mut self, cx: &mut Context<Self>) {
        let name = self.name.read(cx).value().to_string();
        let result = self.service.create(&name).map(|_| ());
        self.result(result, cx);
    }
    fn rename(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.active_id {
            let result = self.service.rename(id, &self.name.read(cx).value());
            self.result(result, cx);
        }
    }
    fn remove(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.active_id else {
            return;
        };
        let service = Arc::clone(&self.service);
        cx.spawn(async move |this, cx| {
            let confirmed = rfd::AsyncMessageDialog::new().set_title("移除画板")
                .set_description("删除此画板及其绘图记录，此操作无法撤销。原始图片、外部文件和已导出的画板文件均保留。")
                .set_buttons(rfd::MessageButtons::YesNo).show().await == rfd::MessageDialogResult::Yes;
            if confirmed { let result = service.remove(id); let _ = this.update(cx, |v, cx| v.result(result, cx)); }
        }).detach();
    }
    fn import(&mut self, cx: &mut Context<Self>) {
        let service = Arc::clone(&self.service);
        cx.spawn(async move |this, cx| {
            let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter("Zorite Scene", &["json"])
                .pick_file()
                .await
            else {
                return;
            };
            let result = crate::background(cx, move || service.import(file.path()))
                .await
                .map(|_| ());
            let _ = this.update(cx, |v, cx| v.result(result, cx));
        })
        .detach();
    }
    fn export(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.active_id else {
            return;
        };
        let service = Arc::clone(&self.service);
        cx.spawn(async move |this, cx| {
            let Some(file) = rfd::AsyncFileDialog::new()
                .set_file_name("whiteboard.json")
                .add_filter("Zorite Scene", &["json"])
                .save_file()
                .await
            else {
                return;
            };
            let result = crate::background(cx, move || service.export(id, file.path())).await;
            let _ = this.update(cx, |v, cx| v.result(result, cx));
        })
        .detach();
    }
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let library = self.service.snapshot();
        let status = self.service.status();
        let state = if let Some(error) = &status.error {
            format!("保存失败：{error}")
        } else if status.revision != status.saved_revision {
            "正在保存…".into()
        } else {
            "已保存".into()
        };
        let board_title = self
            .active_id
            .and_then(|id| library.boards.iter().find(|item| item.id == id))
            .map(|item| item.title.as_str())
            .unwrap_or("未选择画板");
        let active_tool = self.board.as_ref().map(|board| board.read(cx).tool());
        let left_rail = sidebars::left_rail(self, board_title, cx);
        let right_rail = sidebars::right_rail(self, cx);
        let left_drawer = self
            .left_panel
            .map(|panel| sidebars::left_drawer(self, &library, panel, cx));
        let right_drawer = self
            .right_panel
            .map(|panel| sidebars::right_drawer(self, &library, panel, cx));
        let toolbar = div()
            .flex()
            .items_center()
            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
            .children(
                [
                    (
                        Tool::Select,
                        "canvas-tool-select",
                        "选择",
                        gpui_kit::assets::IconName::MousePointer2,
                    ),
                    (
                        Tool::Pan,
                        "canvas-tool-pan",
                        "平移",
                        gpui_kit::assets::IconName::Hand,
                    ),
                    (
                        Tool::Text,
                        "canvas-tool-text",
                        "文字",
                        gpui_kit::assets::IconName::Type,
                    ),
                ]
                .into_iter()
                .map(|(tool, id, label, icon)| {
                    let active = active_tool == Some(tool);
                    let button = components::icon_button(id, icon).tooltip(label);
                    let button = if active {
                        button
                            .workbench_surface(SurfaceRole::Selection)
                            .workbench_foreground(ForegroundRole::Selection)
                    } else {
                        button
                    };
                    button.on_click(cx.listener(move |view, _, _, cx| {
                        if let Some(board) = &view.board {
                            board.update(cx, |canvas, cx| canvas.set_tool(tool, cx));
                        }
                        cx.notify();
                    }))
                }),
            )
            .children(
                [
                    (
                        ToolGroup::Shapes,
                        "canvas-group-shapes",
                        "形状",
                        gpui_kit::assets::IconName::Shapes,
                    ),
                    (
                        ToolGroup::Lines,
                        "canvas-group-lines",
                        "笔与连接线",
                        gpui_kit::assets::IconName::Waypoints,
                    ),
                    (
                        ToolGroup::PagesImages,
                        "canvas-group-assets",
                        "资源工具",
                        gpui_kit::assets::IconName::Image,
                    ),
                ]
                .into_iter()
                .map(|(group, id, label, icon)| {
                    let active = active_tool.is_some_and(|tool| group.contains(tool));
                    let button = components::icon_button(id, icon).tooltip(label);
                    let button = if active {
                        button
                            .workbench_surface(SurfaceRole::Selection)
                            .workbench_foreground(ForegroundRole::Selection)
                    } else {
                        button
                    };
                    button.on_click(cx.listener(move |view, _, _, cx| {
                        if let Some(board) = &view.board {
                            board.update(cx, |canvas, cx| canvas.toggle_tool_group(group, cx));
                        }
                        cx.notify();
                    }))
                }),
            );
        let style_controls = div()
            .flex()
            .items_center()
            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
            .workbench_region(false)
            .p(px(fileatlas_ui_bridge::theme::space_compact()))
            .child(
                components::icon_button("canvas-color", gpui_kit::assets::IconName::Palette)
                    .tooltip("颜色")
                    .on_click(cx.listener(|view, _, _, cx| {
                        if let Some(board) = &view.board {
                            board.update(cx, |canvas, cx| canvas.toggle_color_picker(cx));
                        }
                        cx.notify();
                    })),
            )
            .child(
                components::icon_button("canvas-font", gpui_kit::assets::IconName::Type)
                    .tooltip("字体")
                    .on_click(cx.listener(|view, _, _, cx| {
                        if let Some(board) = &view.board {
                            board.update(cx, |canvas, cx| canvas.toggle_font_picker(cx));
                        }
                        cx.notify();
                    })),
            )
            .child(
                components::icon_button(
                    "canvas-templates",
                    gpui_kit::assets::IconName::LayoutTemplate,
                )
                .tooltip("模板")
                .on_click(cx.listener(|view, _, _, cx| {
                    if let Some(board) = &view.board {
                        board.update(cx, |canvas, cx| canvas.toggle_template_gallery(cx));
                    }
                    cx.notify();
                })),
            );
        let history_controls = div()
            .flex()
            .items_center()
            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
            .workbench_region(false)
            .p(px(fileatlas_ui_bridge::theme::space_compact()))
            .child(
                components::icon_button("canvas-undo", gpui_kit::assets::IconName::Undo2)
                    .tooltip("撤销")
                    .on_click(cx.listener(|view, _, window, cx| {
                        if let Some(board) = &view.board {
                            board.update(cx, |canvas, cx| canvas.undo(window, cx));
                        }
                    })),
            )
            .child(
                components::icon_button("canvas-redo", gpui_kit::assets::IconName::Redo2)
                    .tooltip("重做")
                    .on_click(cx.listener(|view, _, window, cx| {
                        if let Some(board) = &view.board {
                            board.update(cx, |canvas, cx| canvas.redo(window, cx));
                        }
                    })),
            );
        let extension_buttons = self.extensions.tools().into_iter().map(|tool| {
            let contribution_id = tool.id.clone();
            let command_label = tool.label.clone();
            let action = cx.listener(move |view, _, _, cx| {
                if let Err(error) = view.extensions.invoke(&contribution_id) {
                    view.message = Some(error);
                    cx.notify();
                }
            });
            if let Some(icon) = tool.icon {
                components::icon_button(format!("canvas-extension-{}", tool.id), canvas_icon(icon))
                    .tooltip(command_label)
                    .on_click(action)
                    .into_any_element()
            } else {
                components::action(format!("canvas-extension-{}", tool.id), tool.label, false)
                    .on_click(action)
                    .into_any_element()
            }
        });
        let canvas_toolbar = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
            .workbench_region(false)
            .p(px(fileatlas_ui_bridge::theme::space_compact()))
            .child(toolbar)
            .children(extension_buttons);
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
            .p(px(fileatlas_ui_bridge::theme::space_standard()))
            .workbench_body()
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                    .child(left_rail)
                    .when_some(left_drawer, |el, drawer| el.child(drawer))
                    .child(
                        div()
                            .id("whiteboard-native")
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .overflow_hidden()
                            .relative()
                            .on_drop::<fileatlas_contracts::FileManagerFileDragPayload>(
                                cx.listener(
                                    |v,
                                     payload: &fileatlas_contracts::FileManagerFileDragPayload,
                                     _,
                                     cx| {
                                        if payload.allows_drop() {
                                            if let (Some(id), Some(target)) =
                                                (v.active_id, v.board.clone())
                                            {
                                                let [x, y] = target.read(cx).viewport_center();
                                                v.place_images(
                                                    id,
                                                    target,
                                                    Some(
                                                        payload
                                                            .paths
                                                            .iter()
                                                            .map(std::path::PathBuf::from)
                                                            .collect(),
                                                    ),
                                                    x,
                                                    y,
                                                    cx,
                                                );
                                            }
                                        }
                                    },
                                ),
                            )
                            .when_some(self.board.clone(), |el, board| el.child(board))
                            .when(self.board.is_none(), |el| {
                                el.child(components::heading_block(
                                    "画板",
                                    "新建或打开一个画板文件",
                                ))
                            })
                            .child(
                                div()
                                    .absolute()
                                    .left(px(0.))
                                    .right(px(0.))
                                    .bottom(px(
                                        fileatlas_canvas_engine::EMBEDDED_TOOLBAR_BOTTOM_OFFSET,
                                    ))
                                    .flex()
                                    .justify_center()
                                    .child(
                                        div()
                                            .flex()
                                            .flex_wrap()
                                            .items_center()
                                            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                                            .child(history_controls)
                                            .child(canvas_toolbar)
                                            .child(style_controls),
                                    ),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .left(px(0.))
                                    .right(px(0.))
                                    .bottom(px(fileatlas_ui_bridge::theme::space_standard()))
                                    .flex()
                                    .justify_center()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                                            .workbench_assistant()
                                            .child(state)
                                            .when(status.error.is_some(), |el| {
                                                el.child(
                                                    components::action(
                                                        "whiteboard-retry-save",
                                                        "重试保存",
                                                        false,
                                                    )
                                                    .on_click(cx.listener(|v, _, _, cx| {
                                                        let service = Arc::clone(&v.service);
                                                        cx.spawn(async move |this, cx| {
                                                            let result =
                                                                crate::background(cx, move || {
                                                                    service.flush()
                                                                })
                                                                .await;
                                                            let _ = this.update(cx, |v, cx| {
                                                                v.result(result, cx)
                                                            });
                                                        })
                                                        .detach();
                                                    })),
                                                )
                                            })
                                            .child(
                                                self.message
                                                    .clone()
                                                    .or_else(|| {
                                                        self.images
                                                            .borrow()
                                                            .errors()
                                                            .first()
                                                            .cloned()
                                                    })
                                                    .unwrap_or_default(),
                                            ),
                                    ),
                            ),
                    )
                    .when_some(right_drawer, |el, drawer| el.child(drawer))
                    .child(right_rail),
            )
    }
}
fn canvas_icon(icon: CanvasToolIcon) -> gpui_kit::assets::IconName {
    use gpui_kit::assets::IconName;
    match icon {
        CanvasToolIcon::Select => IconName::MousePointer2,
        CanvasToolIcon::Hand => IconName::Hand,
        CanvasToolIcon::Pencil => IconName::Pencil,
        CanvasToolIcon::Shapes => IconName::Shapes,
        CanvasToolIcon::Connector => IconName::Waypoints,
        CanvasToolIcon::Text => IconName::Type,
        CanvasToolIcon::Image => IconName::Image,
        CanvasToolIcon::File => IconName::File,
    }
}
fn layer_label(element: &CanvasElement) -> String {
    match &element.kind {
        ElementKind::Draw(_) => "画笔".into(),
        ElementKind::Rect(_) => "矩形".into(),
        ElementKind::Ellipse(_) => "椭圆".into(),
        ElementKind::Diamond(_) => "菱形".into(),
        ElementKind::Triangle(_) => "三角形".into(),
        ElementKind::RoundRect(_) => "圆角矩形".into(),
        ElementKind::Star(_) => "星形".into(),
        ElementKind::Hexagon(_) => "六边形".into(),
        ElementKind::Line(_) => "线条".into(),
        ElementKind::Arrow(_) => "箭头".into(),
        ElementKind::Text(text) => format!(
            "文字 · {}",
            text.content.chars().take(16).collect::<String>()
        ),
        ElementKind::Embed(embed) => format!("文件 · {}", embed.title),
        ElementKind::Image(image) => {
            format!("图片 · {}", sidebars::resource_name(&image.src))
        }
    }
}
/// Only maps upstream-required host colors. All tools/chrome remain upstream.
fn native_style() -> WhiteboardStyle {
    use fileatlas_ui_bridge::theme as t;
    let c = |v| gpui::rgb(v).into();
    WhiteboardStyle {
        bg: c(t::surface_base()),
        grid: c(t::separator()),
        text: c(t::text_secondary()),
        ink: c(t::text_primary()),
        panel: c(t::surface_elevated()),
        panel_strong: c(t::surface_modal()),
        accent: c(t::selection_color()),
        selection: c(t::accent()),
        swatches: vec![
            c(t::accent()),
            c(t::text_primary()),
            c(t::success()),
            c(t::warning()),
            c(t::danger()),
            c(t::info()),
        ],
    }
}
