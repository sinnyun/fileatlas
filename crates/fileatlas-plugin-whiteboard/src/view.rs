use crate::extensions::CanvasExtensionService;
use crate::{PANEL, WhiteboardService, hooks, images::ImageCache, store};
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
use std::{cell::RefCell, rc::Rc, sync::Arc};

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
    extensions: Arc<CanvasExtensionService>,
    window: &mut Window,
    app: &mut App,
) -> AnyView {
    app.new(|cx| {
        let images = Rc::new(RefCell::new(ImageCache::new(
            Arc::clone(&thumbnails),
            Arc::clone(&resources),
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
            changed
        });
        let tool_task = refresh::listen(cx, tool_wake, |_: &mut Host, _| true);
        let mut host = Host {
            service,
            thumbnails,
            resources,
            quicklook,
            extensions,
            name,
            board: None,
            active_id: None,
            images,
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
    pub extensions: Arc<CanvasExtensionService>,
    pub name: Entity<InputState>,
    pub board: Option<Entity<WhiteboardView>>,
    pub active_id: Option<i64>,
    pub images: Rc<RefCell<ImageCache>>,
    pub message: Option<String>,
    _wake: gpui::Task<()>,
    _images: gpui::Task<()>,
    _tools: gpui::Task<()>,
}
impl Host {
    pub fn sync(&mut self, cx: &mut Context<Self>) {
        let library = self.service.snapshot();
        if self.active_id != library.active_id
            || (self.board.is_none() && library.active_id.is_some())
        {
            self.active_id = library.active_id;
            self.images.borrow_mut().clear();
            self.board = library
                .active_id
                .and_then(|id| library.boards.iter().find(|b| b.id == id))
                .map(|record| {
                    let scene =
                        serde_json::from_str::<fileatlas_canvas_engine::Scene>(&record.scene)
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
        let active_tool = self.board.as_ref().map(|board| board.read(cx).tool());
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
        let layers = if let Some(board) = self.board.clone() {
            let selected = board.read(cx).selected_ids().to_vec();
            let properties = selected.first().and_then(|id| {
                let canvas = board.read(cx);
                let element = canvas.elements().iter().find(|element| element.id == *id)?;
                let [x0, y0, x1, y1] = canvas.element_bounds(*id)?;
                Some(format!(
                    "{}\n位置  X {:.0}  Y {:.0}\n大小  {:.0} × {:.0}",
                    layer_label(element),
                    x0,
                    y0,
                    x1 - x0,
                    y1 - y0
                ))
            });
            let rows = board
                .read(cx)
                .elements()
                .iter()
                .enumerate()
                .rev()
                .map(|(index, element)| {
                    let id = element.id;
                    let label = layer_label(element);
                    let is_selected = selected.contains(&id);
                    let select_board = board.clone();
                    let up_board = board.clone();
                    let down_board = board.clone();
                    let select_button =
                        components::action(format!("canvas-layer-select-{id}"), label, false);
                    let select_button = if is_selected {
                        select_button
                            .workbench_surface(SurfaceRole::Selection)
                            .workbench_foreground(ForegroundRole::Selection)
                    } else {
                        select_button
                    };
                    div()
                        .flex()
                        .items_center()
                        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                        .child(select_button.on_click(cx.listener(move |_, _, _, cx| {
                            select_board.update(cx, |canvas, cx| canvas.select_element(id, cx));
                            cx.notify();
                        })))
                        .child(
                            components::action(format!("canvas-layer-up-{id}"), "上移", false)
                                .on_click(cx.listener(move |_, _, window, cx| {
                                    up_board.update(cx, |canvas, cx| {
                                        canvas.move_element_to(id, index + 1, window, cx)
                                    });
                                })),
                        )
                        .child(
                            components::action(format!("canvas-layer-down-{id}"), "下移", false)
                                .on_click(cx.listener(move |_, _, window, cx| {
                                    down_board.update(cx, |canvas, cx| {
                                        canvas.move_element_to(
                                            id,
                                            index.saturating_sub(1),
                                            window,
                                            cx,
                                        )
                                    });
                                })),
                        )
                })
                .collect::<Vec<_>>();
            div()
                .id("canvas-layers")
                .w(px(240.))
                .h_full()
                .overflow_y_scrollbar()
                .flex()
                .flex_col()
                .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                .workbench_region(true)
                .p(px(fileatlas_ui_bridge::theme::space_standard()))
                .child(components::heading_block("图层", "置顶的图层显示在最前"))
                .children(rows)
                .child(components::heading_block("属性", "选中对象"))
                .child(properties.unwrap_or_else(|| "未选择图层".into()))
                .into_any_element()
        } else {
            div().w(px(220.)).h_full().into_any_element()
        };
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
                    .items_center()
                    .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                    .child(div().w(px(200.)).child(components::text_input(&self.name)))
                    .child(
                        components::icon_button(
                            "whiteboard-new",
                            gpui_kit::assets::IconName::FilePlus,
                        )
                        .tooltip("新建画板")
                        .on_click(cx.listener(|v, _, _, cx| v.create(cx))),
                    )
                    .child(
                        components::icon_button(
                            "whiteboard-rename",
                            gpui_kit::assets::IconName::Pencil,
                        )
                        .tooltip("重命名")
                        .on_click(cx.listener(|v, _, _, cx| v.rename(cx))),
                    )
                    .child(
                        components::icon_button(
                            "whiteboard-import",
                            gpui_kit::assets::IconName::FileInput,
                        )
                        .tooltip("导入画板")
                        .on_click(cx.listener(|v, _, _, cx| v.import(cx))),
                    )
                    .child(
                        components::icon_button(
                            "whiteboard-export",
                            gpui_kit::assets::IconName::FileOutput,
                        )
                        .tooltip("导出画板")
                        .on_click(cx.listener(|v, _, _, cx| v.export(cx))),
                    )
                    .child(
                        components::icon_button(
                            "whiteboard-remove",
                            gpui_kit::assets::IconName::Trash,
                        )
                        .tooltip("移除画板")
                        .on_click(cx.listener(|v, _, _, cx| v.remove(cx))),
                    )
                    .child(
                        components::icon_button(
                            "whiteboard-refresh",
                            gpui_kit::assets::IconName::RefreshCw,
                        )
                        .tooltip("刷新图片")
                        .on_click(cx.listener(|v, _, _, cx| {
                            v.images.borrow_mut().clear();
                            if let Some(board) = &v.board {
                                board.update(cx, |_, cx| cx.notify());
                            }
                            cx.notify();
                        })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                    .child(
                        div()
                            .id("whiteboard-library")
                            .w(px(180.))
                            .h_full()
                            .overflow_y_scrollbar()
                            .flex()
                            .flex_col()
                            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                            .workbench_region(true)
                            .p(px(fileatlas_ui_bridge::theme::space_standard()))
                            .child(components::heading_block("画板", "工作区"))
                            .children(library.boards.iter().map(|record| {
                                let id = record.id;
                                div()
                                    .id(("whiteboard-item", id as usize))
                                    .w_full()
                                    .workbench_nav(self.active_id == Some(id))
                                    .child(div().truncate().child(record.title.clone()))
                                    .on_click(cx.listener(move |v, _, _, cx| {
                                        let result = v.service.activate(id);
                                        v.result(result, cx);
                                    }))
                            }))
                            .child(components::heading_block("资源", "文件与视频卡片"))
                            .children(library.links.iter().map(|file| {
                                let id = file.id;
                                let uri = &file.uri;
                                let name = uri.rsplit('/').next().unwrap_or(uri);
                                let label = if ["mp4", "m4v", "mov", "mkv", "webm", "avi"]
                                    .iter()
                                    .any(|ext| {
                                        name.to_ascii_lowercase().ends_with(&format!(".{ext}"))
                                    }) {
                                    format!("视频 · {name}")
                                } else {
                                    name.to_owned()
                                };
                                div().child(
                                    components::action(
                                        format!("canvas-resource-{id}"),
                                        label,
                                        false,
                                    )
                                    .on_click(cx.listener(
                                        move |view, _, _, cx| {
                                            view.open_file(id, cx);
                                        },
                                    )),
                                )
                            })),
                    )
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
                                    .top(px(fileatlas_ui_bridge::theme::space_standard()))
                                    .flex()
                                    .justify_end()
                                    .pr(px(fileatlas_ui_bridge::theme::space_standard()))
                                    .child(style_controls),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .left(px(0.))
                                    .right(px(0.))
                                    .bottom(px(fileatlas_ui_bridge::theme::space_expanded()))
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                                    .child(history_controls)
                                    .child(
                                        div()
                                            .flex()
                                            .flex_wrap()
                                            .items_center()
                                            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                                            .workbench_region(false)
                                            .p(px(fileatlas_ui_bridge::theme::space_compact()))
                                            .children([toolbar.into_any_element()])
                                            .children(extension_buttons),
                                    ),
                            ),
                    )
                    .child(layers),
            )
            .child(
                div()
                    .flex()
                    .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
                    .workbench_assistant()
                    .child(state)
                    .when(status.error.is_some(), |el| {
                        el.child(
                            components::action("whiteboard-retry-save", "重试保存", false)
                                .on_click(cx.listener(|v, _, _, cx| {
                                    let service = Arc::clone(&v.service);
                                    cx.spawn(async move |this, cx| {
                                        let result =
                                            crate::background(cx, move || service.flush()).await;
                                        let _ = this.update(cx, |v, cx| v.result(result, cx));
                                    })
                                    .detach();
                                })),
                        )
                    })
                    .child(
                        self.message
                            .clone()
                            .or_else(|| self.images.borrow().errors().first().cloned())
                            .unwrap_or_default(),
                    ),
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
        ElementKind::Image(image) => format!(
            "图片 · {}",
            image.src.rsplit('/').next().unwrap_or(&image.src)
        ),
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
