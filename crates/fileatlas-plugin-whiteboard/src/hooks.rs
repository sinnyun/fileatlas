//! The upstream host hooks. Editing, drawing, input and history stay in Zorite.
use crate::{
    images::{self, ImageCache},
    store::WhiteboardService,
    view::Host,
};
use fileatlas_canvas_engine::{Font, FontPick, WhiteboardView};
use gpui::*;
use std::{cell::RefCell, path::PathBuf, rc::Rc, sync::Arc};

pub fn install(
    board: &mut Entity<WhiteboardView>,
    id: i64,
    host: WeakEntity<Host>,
    service: Arc<WhiteboardService>,
    images: Rc<RefCell<ImageCache>>,
    cx: &mut App,
) {
    let target = board.downgrade();
    board.update(cx, |view, _| {
        let store = Arc::clone(&service);
        view.set_on_change(Rc::new(move |json, _, _| {
            if let Err(error) = store.save_scene(id, json) {
                tracing::error!(%error, "whiteboard scene save failed");
            }
        }));
        view.set_on_image(Rc::new(
            move |source, rotation, size, visible, first, _, _| {
                let mut images = images.borrow_mut();
                if first {
                    images.begin_visibility_frame();
                }
                images.update_visibility(source, rotation, visible);
                visible
                    .then(|| images.get(source, rotation, size))
                    .flatten()
            },
        ));
        view.set_on_copy(Rc::new(|text, _, app| {
            app.write_to_clipboard(ClipboardItem::new_string(text))
        }));
        let paste_host = host.clone();
        let paste_target = target.clone();
        view.set_on_paste(Rc::new(move |_, app| {
            if let Some(text) = app.read_from_clipboard().and_then(|item| item.text()) {
                if serde_json::from_str::<Vec<fileatlas_canvas_engine::Element>>(&text).is_ok() {
                    return Some(text);
                }
            }
            if let Some(target) = paste_target.upgrade() {
                let _ = paste_host.update(app, |v, cx| v.paste_image(id, target, cx));
            }
            None
        }));
        let image_host = host.clone();
        let image_target = target.clone();
        view.set_on_place_image(Rc::new(move |x, y, _, app| {
            if let Some(target) = image_target.upgrade() {
                let _ = image_host.update(app, |v, cx| v.place_images(id, target, None, x, y, cx));
            }
        }));
        let drop_host = host.clone();
        let drop_target = target.clone();
        view.set_on_drop_files(Rc::new(move |paths, x, y, _, app| {
            if let Some(target) = drop_target.upgrade() {
                let _ = drop_host.update(app, |v, cx| {
                    v.place_images(id, target, Some(paths), x, y, cx)
                });
            }
        }));
        let file_host = host.clone();
        let file_target = target.clone();
        view.set_on_place_embed(Rc::new(move |x, y, _, app| {
            if let Some(target) = file_target.upgrade() {
                let _ = file_host.update(app, |v, cx| v.place_file(id, target, x, y, cx));
            }
        }));
        let open_host = host.clone();
        view.set_on_open(Rc::new(move |file_id, _, app| {
            let _ = open_host.update(app, |v, cx| v.open_file(file_id, cx));
        }));
        let template_host = host.clone();
        view.set_on_save_template(Rc::new(move |elements, _, app| {
            let _ = template_host.update(app, |v, cx| v.save_template(elements, cx));
        }));
        let template_store = Arc::clone(&service);
        view.set_on_delete_template(Rc::new(move |template_id, _, _| {
            if let Err(error) = template_store.delete_template(template_id) {
                tracing::warn!(%error, "whiteboard template removal failed");
            }
        }));
        let colors_store = Arc::clone(&service);
        view.set_on_save_colors(Rc::new(move |colors, _, _| {
            if let Err(error) = colors_store.save_colors(colors) {
                tracing::warn!(%error, "whiteboard colors save failed");
            }
        }));
        let toolbar_store = Arc::clone(&service);
        view.set_on_move_toolbar(Rc::new(move |pos, vertical, _, _| {
            if let Err(error) = toolbar_store.update_board(id, |b| {
                b.toolbar_pos = pos;
                b.toolbar_vertical = vertical;
            }) {
                tracing::warn!(%error, "whiteboard toolbar save failed");
            }
        }));
        let font_host = host.clone();
        let font_target = target.clone();
        view.set_on_pick_font(Rc::new(move |pick, _, app| {
            if let Some(target) = font_target.upgrade() {
                let _ = font_host.update(app, |v, cx| v.pick_font(id, target, pick, cx));
            }
        }));
    });
}
impl Host {
    pub fn place_images(
        &mut self,
        id: i64,
        target: Entity<WhiteboardView>,
        paths: Option<Vec<PathBuf>>,
        x: f32,
        y: f32,
        cx: &mut Context<Self>,
    ) {
        let thumbnails = Arc::clone(&self.thumbnails);
        let service = Arc::clone(&self.service);
        let resources = Arc::clone(&self.resources);
        let preview_store = Arc::clone(&service);
        let eagle_links = self.eagle_links.clone();
        cx.spawn(async move |this, cx| {
            let paths = match paths {
                Some(paths) => paths,
                None => {
                    let Some(files) = rfd::AsyncFileDialog::new()
                        .add_filter(
                            "Images",
                            &["png", "jpg", "jpeg", "webp", "gif", "bmp", "tiff", "avif"],
                        )
                        .pick_files()
                        .await
                    else {
                        return;
                    };
                    files.into_iter().map(|f| f.path().to_path_buf()).collect()
                }
            };
            let result = crate::background(cx, move || {
                let images =
                    images::linked_images(paths, &thumbnails, &resources, eagle_links.as_deref())?;
                let candidates = images
                    .iter()
                    .filter_map(|image| image.preview.clone())
                    .collect::<Vec<_>>();
                let previews = preview_store.persist_image_previews(id, &candidates)?;
                Ok::<_, String>((images, previews))
            })
            .await;
            let _ = this.update(cx, |host, cx| {
                let result = result.and_then(|(images, previews)| {
                    let eagle_sources = images
                        .iter()
                        .filter_map(|image| image.eagle_source.clone())
                        .collect::<Vec<_>>();
                    let scene = target.update(cx, |v, cx| {
                        for (i, image) in images.into_iter().enumerate() {
                            v.add_image_at(
                                image.uri.clone(),
                                image.width as f32,
                                image.height as f32,
                                x + i as f32 * 24.,
                                y + i as f32 * 24.,
                                cx,
                            );
                        }
                        v.scene().to_json()
                    });
                    service.save_scene_assets(id, scene, eagle_sources, previews.clone())?;
                    host.images.borrow_mut().add_previews(id, &previews);
                    Ok(())
                });
                host.result(result, cx);
            });
        })
        .detach();
    }
    fn paste_image(&mut self, id: i64, target: Entity<WhiteboardView>, cx: &mut Context<Self>) {
        let directory = self.service.root.join("pasted-images");
        let target_clone = target.clone();
        cx.spawn(async move |this, cx| {
            let result = crate::background(cx, move || {
                let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
                let Ok(bitmap) = clipboard.get_image() else {
                    return Ok(None);
                };
                let image = image::RgbaImage::from_raw(
                    bitmap.width as u32,
                    bitmap.height as u32,
                    bitmap.bytes.into_owned(),
                )
                .ok_or("剪贴板图片数据无效")?;
                std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
                let stamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e| e.to_string())?
                    .as_nanos();
                let path = directory.join(format!("clipboard-{stamp}.png"));
                image.save(&path).map_err(|e| e.to_string())?;
                Ok::<_, String>(Some(path))
            })
            .await;
            let _ = this.update(cx, |v, cx| match result {
                Ok(Some(path)) => {
                    let [x, y] = target_clone.read(cx).viewport_center();
                    v.place_images(id, target, Some(vec![path]), x, y, cx);
                }
                Ok(None) => {}
                Err(error) => v.result(Err(error), cx),
            });
        })
        .detach();
    }
    pub(super) fn place_file(
        &mut self,
        id: i64,
        target: Entity<WhiteboardView>,
        x: f32,
        y: f32,
        cx: &mut Context<Self>,
    ) {
        let service = Arc::clone(&self.service);
        let resources = Arc::clone(&self.resources);
        cx.spawn(async move |this, cx| {
            let Some(file) = rfd::AsyncFileDialog::new().pick_file().await else {
                return;
            };
            let path = file.path().to_owned();
            let link_service = Arc::clone(&service);
            let result = crate::background(cx, move || {
                let link = resources.link_file(path)?;
                let title = match link.kind {
                    fileatlas_contracts::resources::ResourceKind::Video => {
                        format!("视频 · {}", link.title)
                    }
                    fileatlas_contracts::resources::ResourceKind::Audio => {
                        format!("音频 · {}", link.title)
                    }
                    _ => link.title,
                };
                link_service
                    .link_resource(id, &link.uri)
                    .map(|file_id| (file_id, title))
            })
            .await;
            let _ = this.update(cx, |v, cx| {
                let result = result.and_then(|(file_id, title)| {
                    let scene = target.update(cx, |b, cx| {
                        b.add_embed(file_id, title, x, y, cx);
                        b.scene().to_json()
                    });
                    service.save_scene(id, scene)
                });
                v.result(result, cx);
            });
        })
        .detach();
    }
    pub(super) fn open_file(&mut self, file_id: i64, cx: &mut Context<Self>) {
        let uri = self.service.linked_file(file_id);
        let Some(uri) = uri else {
            self.result(Err("此卡片的宿主文件链接不可用".into()), cx);
            return;
        };
        let resources = Arc::clone(&self.resources);
        let quicklook = Arc::clone(&self.quicklook);
        cx.spawn(async move |this, cx| {
            let result = crate::background(cx, move || {
                if resources.open_with_provider(&uri)? {
                    return Ok(());
                }
                let path = resources.local_path(&uri)?;
                quicklook.open(path).map_err(|e| e.to_string())
            })
            .await;
            let _ = this.update(cx, |v, cx| v.result(result, cx));
        })
        .detach();
    }
    fn save_template(&mut self, elements: String, cx: &mut Context<Self>) {
        let service = Arc::clone(&self.service);
        cx.spawn(async move |this, cx| {
            let Some(file) = rfd::AsyncFileDialog::new()
                .set_file_name("template.json")
                .add_filter("Zorite Template", &["json"])
                .save_file()
                .await
            else {
                return;
            };
            let result = crate::background(cx, move || {
                crate::store::atomic_write(file.path(), elements.as_bytes())?;
                let name = file
                    .path()
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Template")
                    .to_owned();
                service.save_template(name, elements)
            })
            .await;
            let _ = this.update(cx, |v, cx| v.result(result, cx));
        })
        .detach();
    }
    fn pick_font(
        &mut self,
        id: i64,
        target: Entity<WhiteboardView>,
        pick: FontPick,
        cx: &mut Context<Self>,
    ) {
        if matches!(pick, FontPick::Default) {
            let result = self.service.update_board(id, |b| b.font_path = None);
            cx.defer(move |cx| target.update(cx, |v, cx| v.set_font(Font::default(), cx)));
            self.result(result, cx);
            return;
        }
        cx.spawn(async move |this, cx| {
            if let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter("Fonts", &["ttf", "otf"])
                .pick_file()
                .await
            {
                let _ = this.update(cx, |v, cx| {
                    v.load_font(target, id, file.path().to_string_lossy().into_owned(), cx)
                });
            }
        })
        .detach();
    }
    pub fn load_font(
        &mut self,
        target: Entity<WhiteboardView>,
        id: i64,
        path: String,
        cx: &mut Context<Self>,
    ) {
        let service = Arc::clone(&self.service);
        cx.spawn(async move |this, cx| {
            let source = path.clone();
            let result =
                crate::background(cx, move || std::fs::read(source).map_err(|e| e.to_string()))
                    .await;
            let _ = this.update(cx, |v, cx| {
                let result = result.and_then(|bytes| {
                    let font = Font::from_bytes(bytes, 0).ok_or("无法读取字体")?;
                    target.update(cx, |b, cx| b.set_font(font, cx));
                    service.update_board(id, |b| b.font_path = Some(path))
                });
                v.result(result, cx);
            });
        })
        .detach();
    }
}
