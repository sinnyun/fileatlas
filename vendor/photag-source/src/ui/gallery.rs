use crate::app::AppState;
use crate::db::queries::{MediaItem, SortOrder};
use crate::theme;
use egui::{self, Color32, Rect, Rounding, Sense, TextureHandle, Ui, Vec2};
use image::ImageDecoder;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::{mpsc, Arc, Condvar, Mutex};

// ══════════════════════════════════════════════════════════════════
//  EXIF orientation
// ══════════════════════════════════════════════════════════════════

fn read_jpeg_exif_orientation(data: &[u8]) -> u8 {
    if data.len() < 12 || data[0] != 0xFF || data[1] != 0xD8 { return 1; }
    let mut pos = 2;
    while pos + 4 < data.len() {
        if data[pos] != 0xFF { return 1; }
        let marker = data[pos + 1];
        if marker == 0xE1 {
            let seg_len = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
            let seg_start = pos + 4;
            let seg_end = (pos + 2 + seg_len).min(data.len());
            if seg_end - seg_start < 14 { return 1; }
            if &data[seg_start..seg_start + 6] != b"Exif\0\0" { return 1; }
            let tiff_start = seg_start + 6;
            let tiff = &data[tiff_start..seg_end];
            if tiff.len() < 8 { return 1; }
            let le = tiff[0] == b'I' && tiff[1] == b'I';
            let read_u16 = |off: usize| -> u16 {
                if off + 2 > tiff.len() { return 0; }
                if le { u16::from_le_bytes([tiff[off], tiff[off + 1]]) }
                else { u16::from_be_bytes([tiff[off], tiff[off + 1]]) }
            };
            let ifd_offset = if le {
                u32::from_le_bytes([tiff[4], tiff[5], tiff[6], tiff[7]])
            } else {
                u32::from_be_bytes([tiff[4], tiff[5], tiff[6], tiff[7]])
            } as usize;
            if ifd_offset + 2 > tiff.len() { return 1; }
            let count = read_u16(ifd_offset) as usize;
            for i in 0..count {
                let entry = ifd_offset + 2 + i * 12;
                if entry + 12 > tiff.len() { break; }
                if read_u16(entry) == 0x0112 { return read_u16(entry + 8) as u8; }
            }
            return 1;
        }
        if marker == 0xDA { break; }
        let seg_len = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
        pos += 2 + seg_len;
    }
    1
}

fn apply_orientation(pixels: &[Color32], w: usize, h: usize, orientation: u8) -> (Vec<Color32>, usize, usize) {
    match orientation {
        1 => (pixels.to_vec(), w, h),
        2 => { let mut o = vec![Color32::BLACK; w*h]; for y in 0..h { for x in 0..w { o[y*w+x] = pixels[y*w+(w-1-x)]; }} (o, w, h) }
        3 => { let mut o = pixels.to_vec(); o.reverse(); (o, w, h) }
        4 => { let mut o = vec![Color32::BLACK; w*h]; for y in 0..h { for x in 0..w { o[y*w+x] = pixels[(h-1-y)*w+x]; }} (o, w, h) }
        5 => { let mut o = vec![Color32::BLACK; w*h]; for y in 0..h { for x in 0..w { o[x*h+y] = pixels[y*w+x]; }} (o, h, w) }
        6 => { let mut o = vec![Color32::BLACK; w*h]; for y in 0..h { for x in 0..w { o[x*h+(h-1-y)] = pixels[y*w+x]; }} (o, h, w) }
        7 => { let mut o = vec![Color32::BLACK; w*h]; for y in 0..h { for x in 0..w { o[(w-1-x)*h+(h-1-y)] = pixels[y*w+x]; }} (o, h, w) }
        8 => { let mut o = vec![Color32::BLACK; w*h]; for y in 0..h { for x in 0..w { o[(w-1-x)*h+y] = pixels[y*w+x]; }} (o, h, w) }
        _ => (pixels.to_vec(), w, h),
    }
}

// ══════════════════════════════════════════════════════════════════
//  Image decode — mmap + zune-jpeg fast path
// ══════════════════════════════════════════════════════════════════

fn decode_jpeg_from_bytes(data: &[u8], max_size: u32) -> Option<egui::ColorImage> {
    use zune_jpeg::zune_core::options::DecoderOptions;
    use zune_jpeg::zune_core::colorspace::ColorSpace;
    let orientation = read_jpeg_exif_orientation(data);
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGB);
    let cursor = std::io::Cursor::new(data);
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(cursor, options);
    decoder.decode_headers().ok()?;
    let info = decoder.info()?;
    let w = info.width as usize;
    let h = info.height as usize;
    let pixels = decoder.decode().ok()?;
    let src: Vec<Color32> = pixels.chunks_exact(3)
        .map(|c| Color32::from_rgba_unmultiplied(c[0], c[1], c[2], 255))
        .collect();
    let (oriented, ow, oh) = apply_orientation(&src, w, h, orientation);
    downscale_if_needed(oriented, ow, oh, max_size)
}

fn downscale_if_needed(pixels: Vec<Color32>, w: usize, h: usize, max_size: u32) -> Option<egui::ColorImage> {
    let max = max_size as usize;
    if w > max || h > max {
        let scale = max as f32 / w.max(h) as f32;
        let nw = ((w as f32) * scale).max(1.0) as usize;
        let nh = ((h as f32) * scale).max(1.0) as usize;
        let mut small = vec![Color32::BLACK; nw * nh];
        for y in 0..nh {
            let sy = y * h / nh;
            for x in 0..nw { small[y * nw + x] = pixels[sy * w + (x * w / nw)]; }
        }
        Some(egui::ColorImage { size: [nw, nh], pixels: small })
    } else {
        Some(egui::ColorImage { size: [w, h], pixels })
    }
}

/// Opt #7: mmap for files > 32KB, regular read for small files.
fn read_file_bytes(path: &Path) -> Option<Vec<u8>> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > 32768 {
        let file = std::fs::File::open(path).ok()?;
        let mmap = unsafe { memmap2::Mmap::map(&file) }.ok()?;
        Some(mmap.to_vec())
    } else {
        std::fs::read(path).ok()
    }
}

/// Decode from raw bytes (for worker threads loading from disk or re-decoding from byte cache).
fn decode_from_bytes(data: &[u8], path: &str, max_size: u32) -> Option<egui::ColorImage> {
    let ext = Path::new(path).extension()
        .map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if ext == "jpg" || ext == "jpeg" {
        if let Some(img) = decode_jpeg_from_bytes(data, max_size) { return Some(img); }
    }
    // Fallback: image crate (handles WebP, PNG, etc.)
    let cursor = std::io::Cursor::new(data);
    let reader = image::ImageReader::new(cursor).with_guessed_format().ok()?;
    let mut decoder = reader.into_decoder().ok()?;
    let orientation = decoder.orientation().unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = image::DynamicImage::from_decoder(decoder).ok()?;
    img.apply_orientation(orientation);
    let img = img.thumbnail(max_size, max_size);
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}

/// Load an image for the preview view at a target maximum dimension.
///
/// Fast path for JPEGs:
///   - mmap the file (avoids a full read+copy for large originals)
///   - decode with `jpeg-decoder` requesting a DCT-scaled output close to
///     `max_dim`. JPEG can be decoded at 1/1, 1/2, 1/4, or 1/8 of the
///     original size during the IDCT pass — that's typically 4–16× faster
///     than a full decode followed by a separate downsample, because the
///     decoder skips most of the high-frequency coefficients.
///   - apply EXIF orientation
///
/// Fallback (PNG / WebP / HEIC / etc.): `image` crate, then thumbnail.
///
/// `max_dim` is the longest edge we want; pass the screen's longest edge
/// so we don't waste GPU memory on pixels the user can't see anyway.
pub fn load_full_image_to(path: &str, max_dim: u32) -> Option<egui::ColorImage> {
    let p = Path::new(path);
    let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();

    if ext == "jpg" || ext == "jpeg" {
        if let Some(img) = decode_jpeg_scaled(p, max_dim) {
            return Some(img);
        }
        // fall through to the generic decoder if the fast path bailed
    }

    // Generic fallback for non-JPEG (or rare JPEGs the fast path declined).
    let reader = image::ImageReader::open(p).ok()?;
    let mut decoder = reader.into_decoder().ok()?;
    let orientation = decoder.orientation().unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = image::DynamicImage::from_decoder(decoder).ok()?;
    img.apply_orientation(orientation);
    if img.width() > max_dim || img.height() > max_dim {
        img = img.thumbnail(max_dim, max_dim);
    }
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}

/// JPEG fast path: mmap → jpeg-decoder with DCT scaling → EXIF orient.
fn decode_jpeg_scaled(path: &Path, max_dim: u32) -> Option<egui::ColorImage> {
    use jpeg_decoder::{Decoder as JDecoder, PixelFormat};

    // Read raw bytes (mmap for big files — avoids the OS copying every
    // megabyte through a stdio buffer).
    let bytes = read_file_bytes(path)?;
    let orientation = read_jpeg_exif_orientation(&bytes);

    let mut dec = JDecoder::new(std::io::Cursor::new(&bytes));
    dec.read_info().ok()?;
    let info = dec.info()?;
    let nat_w = info.width as u32;
    let nat_h = info.height as u32;
    if nat_w == 0 || nat_h == 0 { return None; }

    // Ask the decoder for a DCT-scaled output. `scale(req_w, req_h)`
    // returns the actual dimensions it'll use (always one of /1, /2, /4,
    // /8 of natural). Slightly larger than max_dim is fine — the GPU
    // will sample down for free.
    let target = max_dim.max(64);
    let (sw, sh) = dec.scale(target.min(nat_w) as u16, target.min(nat_h) as u16).ok()?;
    let pixels = dec.decode().ok()?;
    let fmt = dec.info()?.pixel_format;

    // Convert to RGBA8.
    let n = (sw as usize) * (sh as usize);
    let mut rgba: Vec<Color32> = match fmt {
        PixelFormat::RGB24 => {
            if pixels.len() < n * 3 { return None; }
            pixels.chunks_exact(3)
                .map(|c| Color32::from_rgba_unmultiplied(c[0], c[1], c[2], 255))
                .collect()
        }
        PixelFormat::L8 => {
            if pixels.len() < n { return None; }
            pixels.iter().map(|&v| Color32::from_rgba_unmultiplied(v, v, v, 255)).collect()
        }
        // CMYK / L16 / RGBA: punt to the image-crate fallback.
        _ => return None,
    };

    // Apply EXIF orientation. The buffer here is at the scaled size, so
    // rotation/flip works on the smaller pixel set — much cheaper than
    // doing it on the full-resolution image.
    let (oriented, ow, oh) = apply_orientation(&rgba, sw as usize, sh as usize, orientation);
    rgba = oriented;
    Some(egui::ColorImage { size: [ow, oh], pixels: rgba })
}

// ══════════════════════════════════════════════════════════════════
//  Opt #4: Priority queue for worker pool
// ══════════════════════════════════════════════════════════════════

struct LoadRequest {
    id: i64,
    path: String,
    max_size: u32,
    priority: u32,  // lower = more important
}

impl PartialEq for LoadRequest { fn eq(&self, other: &Self) -> bool { self.priority == other.priority } }
impl Eq for LoadRequest {}
impl PartialOrd for LoadRequest {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }
}
impl Ord for LoadRequest {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.priority.cmp(&self.priority) // reverse: lower number = pop first
    }
}

struct PriorityQueue {
    heap: Mutex<BinaryHeap<LoadRequest>>,
    cond: Condvar,
}

impl PriorityQueue {
    fn new() -> Self { Self { heap: Mutex::new(BinaryHeap::new()), cond: Condvar::new() } }
    fn push(&self, req: LoadRequest) {
        self.heap.lock().unwrap().push(req);
        self.cond.notify_one();
    }
    fn pop(&self) -> LoadRequest {
        let mut heap = self.heap.lock().unwrap();
        loop {
            if let Some(req) = heap.pop() { return req; }
            heap = self.cond.wait(heap).unwrap();
        }
    }
}

/// Result from worker: decoded image + raw bytes for compressed cache.
struct LoadResult {
    id: i64,
    image: egui::ColorImage,
    raw_bytes: Arc<Vec<u8>>,
    path: String,
}

fn start_worker_pool(num_workers: usize) -> (Arc<PriorityQueue>, mpsc::Receiver<LoadResult>) {
    let queue = Arc::new(PriorityQueue::new());
    let (res_tx, res_rx) = mpsc::channel::<LoadResult>();

    for _ in 0..num_workers {
        let q = queue.clone();
        let tx = res_tx.clone();
        std::thread::spawn(move || {
            loop {
                let req = q.pop();
                if req.id == -1 { break; }
                let raw_bytes = match read_file_bytes(Path::new(&req.path)) {
                    Some(b) => b,
                    None => continue,
                };
                if let Some(image) = decode_from_bytes(&raw_bytes, &req.path, req.max_size) {
                    tx.send(LoadResult {
                        id: req.id,
                        image,
                        raw_bytes: Arc::new(raw_bytes),
                        path: req.path,
                    }).ok();
                }
            }
        });
    }

    (queue, res_rx)
}

// ══════════════════════════════════════════════════════════════════
//  TextureManager — individual textures + compressed byte cache
//  (no atlas — simpler and faster on most GPU drivers)
// ══════════════════════════════════════════════════════════════════

const MAX_TEXTURES: usize = 2000;
const MAX_BYTE_CACHE: usize = 30000; // ~30K × ~10KB avg = ~300MB
const MAX_PIXEL_CACHE: usize = 4000; // ~4K × ~100KB avg = ~400MB decoded pixels

pub struct TextureManager {
    // Tier 1: GPU textures (individual, fast)
    cache: HashMap<i64, TextureHandle>,
    lru_order: VecDeque<i64>,

    // Tier 1.5: Decoded pixel cache (instant scroll-back, no re-decode)
    pixel_cache: HashMap<i64, Arc<egui::ColorImage>>,
    pixel_lru: VecDeque<i64>,

    // Tier 2: Compressed byte cache (raw file bytes)
    byte_cache: HashMap<i64, (Arc<Vec<u8>>, String)>, // (bytes, path)
    byte_lru: VecDeque<i64>,

    // Opt #2: Micro-thumbnail textures (tiny, kept for lifetime of session)
    micro_textures: HashMap<i64, TextureHandle>,

    pending: HashSet<i64>,
    queue: Arc<PriorityQueue>,
    res_rx: mpsc::Receiver<LoadResult>,

    // Opt #1: Adaptive decode resolution
    current_thumb_size: u32,

    // Scroll tracking
    last_viewport_y: f32,
    scroll_direction: f32,

    // Opt #5: Adaptive batch upload
    upload_cap: usize,
}

impl TextureManager {
    pub fn new() -> Self {
        let (queue, res_rx) = start_worker_pool(6);
        Self {
            cache: HashMap::new(),
            lru_order: VecDeque::new(),
            pixel_cache: HashMap::new(),
            pixel_lru: VecDeque::new(),
            byte_cache: HashMap::new(),
            byte_lru: VecDeque::new(),
            micro_textures: HashMap::new(),
            pending: HashSet::new(),
            queue,
            res_rx,
            current_thumb_size: 160,
            last_viewport_y: 0.0,
            scroll_direction: 1.0,
            upload_cap: 60,
        }
    }

    /// Opt #1: Match decode resolution to display size (quantized to avoid churn).
    pub fn set_thumb_size(&mut self, size: f32) {
        let raw = (size * 1.1).ceil() as u32;
        self.current_thumb_size = if raw <= 128 { 128 }
            else if raw <= 192 { 192 }
            else if raw <= 256 { 256 }
            else { 320 };
    }

    /// Opt #5: Adaptive batch — process completed loads each frame.
    pub fn poll(&mut self, ctx: &egui::Context) {
        let start = std::time::Instant::now();
        let mut count = 0;

        while let Ok(result) = self.res_rx.try_recv() {
            // Tier 2: Store compressed bytes (Opt #3)
            self.insert_byte_cache(result.id, result.raw_bytes, &result.path);

            // Tier 1.5: Cache decoded pixels for instant scroll-back
            let pixels = Arc::new(result.image);
            self.insert_pixel_cache(result.id, pixels.clone());

            // Tier 1: Upload to GPU
            let handle = ctx.load_texture(
                format!("thumb_{}", result.id),
                (*pixels).clone(),
                egui::TextureOptions::LINEAR,
            );
            self.insert_gpu_lru(result.id, handle);
            self.pending.remove(&result.id);
            count += 1;
            if count >= self.upload_cap { break; }
        }

        if count > 0 {
            // Opt #5: Self-tune cap based on actual upload time
            let ms = start.elapsed().as_secs_f32() * 1000.0;
            if ms < 4.0 && self.upload_cap < 120 { self.upload_cap += 10; }
            else if ms > 8.0 && self.upload_cap > 20 { self.upload_cap -= 10; }
        }

        // Keep repainting while workers are decoding so we pick up results
        // on the next frame. Without this, decoded textures pile up in the
        // channel until the user moves their mouse or a timer fires.
        if !self.pending.is_empty() {
            ctx.request_repaint();
        }
    }

    fn insert_gpu_lru(&mut self, id: i64, handle: TextureHandle) {
        if self.cache.contains_key(&id) {
            self.lru_order.retain(|&x| x != id);
        }
        self.cache.insert(id, handle);
        self.lru_order.push_back(id);
        while self.cache.len() > MAX_TEXTURES {
            if let Some(old_id) = self.lru_order.pop_front() {
                self.cache.remove(&old_id);
                // Pixel data stays in byte_cache!
            }
        }
    }

    fn insert_byte_cache(&mut self, id: i64, data: Arc<Vec<u8>>, path: &str) {
        if self.byte_cache.contains_key(&id) {
            self.byte_lru.retain(|&x| x != id);
        }
        self.byte_cache.insert(id, (data, path.to_string()));
        self.byte_lru.push_back(id);
        while self.byte_cache.len() > MAX_BYTE_CACHE {
            if let Some(old_id) = self.byte_lru.pop_front() {
                self.byte_cache.remove(&old_id);
            }
        }
    }

    fn touch_lru(&mut self, id: i64) {
        if let Some(pos) = self.lru_order.iter().rposition(|&x| x == id) {
            if pos < self.lru_order.len().saturating_sub(100) {
                self.lru_order.remove(pos);
                self.lru_order.push_back(id);
            }
        }
    }

    fn insert_pixel_cache(&mut self, id: i64, data: Arc<egui::ColorImage>) {
        if self.pixel_cache.contains_key(&id) {
            self.pixel_lru.retain(|&x| x != id);
        }
        self.pixel_cache.insert(id, data);
        self.pixel_lru.push_back(id);
        while self.pixel_cache.len() > MAX_PIXEL_CACHE {
            if let Some(old_id) = self.pixel_lru.pop_front() {
                self.pixel_cache.remove(&old_id);
            }
        }
    }

    fn touch_pixel_lru(&mut self, id: i64) {
        if let Some(pos) = self.pixel_lru.iter().rposition(|&x| x == id) {
            if pos < self.pixel_lru.len().saturating_sub(100) {
                self.pixel_lru.remove(pos);
                self.pixel_lru.push_back(id);
            }
        }
    }

    /// Get cached texture or queue a load.
    pub fn get_or_load(&mut self, item: &MediaItem, priority: u32, ctx: &egui::Context) -> Option<&TextureHandle> {
        // Tier 1: GPU texture hit — instant
        if self.cache.contains_key(&item.id) {
            self.touch_lru(item.id);
            return self.cache.get(&item.id);
        }

        // Tier 1.5: Pixel cache hit — instant GPU upload, no decode needed
        if let Some(pixels) = self.pixel_cache.get(&item.id).cloned() {
            self.touch_pixel_lru(item.id);
            let handle = ctx.load_texture(
                format!("thumb_{}", item.id),
                (*pixels).clone(),
                egui::TextureOptions::LINEAR,
            );
            self.insert_gpu_lru(item.id, handle);
            return self.cache.get(&item.id);
        }

        // Tier 2+3: Queue to worker pool (async, never blocks render)
        // Never load full originals — only use thumbnail path
        if !self.pending.contains(&item.id) {
            let path = item.thumb_path.clone();
            if let Some(path) = path {
                self.pending.insert(item.id);
                let prio = if self.byte_cache.contains_key(&item.id) { 0 } else { priority };
                self.queue.push(LoadRequest {
                    id: item.id,
                    path,
                    max_size: self.current_thumb_size,
                    priority: prio,
                });
            }
        }

        None
    }

    /// Opt #2: Micro-thumbnail from DB for instant blurred placeholder.
    pub fn get_micro_texture(&mut self, item: &MediaItem, ctx: &egui::Context) -> Option<&TextureHandle> {
        if self.micro_textures.contains_key(&item.id) {
            return self.micro_textures.get(&item.id);
        }
        let data = item.micro_thumb.as_ref()?;
        if data.len() < 3 { return None; }
        let pixel_count = data.len() / 3;
        let (mw, mh) = if pixel_count == 64 { (8, 8) }
            else if pixel_count >= 48 { (8, (pixel_count / 8).max(1)) }
            else { return None; };
        let pixels: Vec<Color32> = data.chunks_exact(3)
            .take(mw * mh)
            .map(|c| Color32::from_rgb(c[0], c[1], c[2]))
            .collect();
        if pixels.len() < mw * mh { return None; }
        let image = egui::ColorImage { size: [mw, mh], pixels };
        let handle = ctx.load_texture(
            format!("micro_{}", item.id), image,
            egui::TextureOptions::LINEAR,
        );
        self.micro_textures.insert(item.id, handle);
        self.micro_textures.get(&item.id)
    }

    /// Pre-fetch items about to scroll into view.
    pub fn prefetch(&mut self, items: &[&MediaItem], base_priority: u32) {
        for (i, item) in items.iter().enumerate() {
            if self.cache.contains_key(&item.id) || self.pending.contains(&item.id) {
                continue;
            }
            let path = item.thumb_path.clone();
            if let Some(path) = path {
                let prio = if self.byte_cache.contains_key(&item.id) { 0 } else { base_priority + i as u32 };
                self.pending.insert(item.id);
                self.queue.push(LoadRequest {
                    id: item.id, path,
                    max_size: self.current_thumb_size,
                    priority: prio,
                });
            }
        }
    }

    /// Opt #6: Pre-warm first N items on folder switch.
    pub fn prewarm(&mut self, items: &[MediaItem]) {
        for (i, item) in items.iter().take(60).enumerate() {
            if self.cache.contains_key(&item.id) || self.pending.contains(&item.id) {
                continue;
            }
            let path = item.thumb_path.clone();
            if let Some(path) = path {
                let prio = if self.byte_cache.contains_key(&item.id) { 0 } else { i as u32 };
                self.pending.insert(item.id);
                self.queue.push(LoadRequest {
                    id: item.id, path,
                    max_size: self.current_thumb_size,
                    priority: prio,
                });
            }
        }
    }

    pub fn update_scroll(&mut self, viewport_y: f32) -> f32 {
        let delta = viewport_y - self.last_viewport_y;
        if delta.abs() > 0.5 {
            self.scroll_direction = self.scroll_direction * 0.7 + delta.signum() * 0.3;
        }
        self.last_viewport_y = viewport_y;
        self.scroll_direction
    }

    pub fn retain_only(&mut self, valid_ids: &HashSet<i64>) {
        self.cache.retain(|id, _| valid_ids.contains(id));
        self.lru_order.retain(|id| valid_ids.contains(id));
        self.pixel_cache.retain(|id, _| valid_ids.contains(id));
        self.pixel_lru.retain(|id| valid_ids.contains(id));
        // Keep byte_cache across folder switches — it's cheap
        self.pending.retain(|id| valid_ids.contains(id));
    }

    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Look up an already-uploaded GPU thumbnail by media id, without
    /// touching the LRU. Used by the preview view to paint an instant
    /// blurred placeholder while the full image decodes in the background.
    pub fn peek_thumb(&self, id: i64) -> Option<&TextureHandle> {
        self.cache.get(&id)
    }
}

// ══════════════════════════════════════════════════════════════════
//  Row-based layout (date-section dividers)
// ══════════════════════════════════════════════════════════════════

/// One row of the gallery layout — either a section header (date label)
/// or a row of thumbnail items. Each row knows its absolute `y` and
/// height so the renderer can virtualize against the viewport bounds.
struct GalleryRow {
    y: f32,
    h: f32,
    kind: RowKind,
}

enum RowKind {
    /// "April 2026" — full-width date section header with its item count.
    Header { label: String, count: usize },
    /// Indices into `state.media` for the items in this row (≤ `cols`).
    Items(Vec<usize>),
}

struct GalleryLayout {
    rows: Vec<GalleryRow>,
    total_height: f32,
}

/// Per-frame caches that survive across frames as long as their inputs
/// haven't changed. Owned by `PhoTagApp` and passed into `show_gallery` so
/// the gallery can skip rebuilding the row layout, the flat tag lookup,
/// and chip text-layout widths when nothing relevant changed.
///
/// Cache invalidation is keyed on `AppState::media_gen` and
/// `AppState::tag_tree_gen` (plus layout-shape inputs) — generations bump
/// at every mutation site, so consumers never see stale data.
pub struct GalleryCache {
    // ── Layout cache ─────────────────────────────────────────────────
    layout: Option<GalleryLayout>,
    layout_key: Option<LayoutKey>,
    // ── Flat tag lookup (id → bare Tag, no children) ─────────────────
    tag_lookup: HashMap<i64, crate::db::queries::Tag>,
    tag_lookup_gen: u64,
    // ── Chip derived data per tag id ─────────────────────────────────
    // (icon_w, name_w) — font-shaped chip text widths
    chip_widths: HashMap<i64, (f32, f32)>,
    // Resolved icon string + color after walking the parent chain.
    // Building these inline every frame was ~3-4 HashMap lookups per
    // chip per thumb per frame; now it's one lookup once per tag per
    // generation.
    chip_effective: HashMap<i64, (String, Color32)>,
    // "{full_path}>" prefix string used by the leaf-filter inside
    // draw_thumb_tag_chips. Cached so the per-chip `format!()` doesn't
    // run every frame.
    chip_path_prefix: HashMap<i64, String>,
    chip_widths_gen: u64,
}

#[derive(PartialEq, Clone)]
struct LayoutKey {
    media_gen: u64,
    sort: SortOrder,
    cols: usize,
    thumb_size_q: u32,
    gap_q: u32,
    show_date_dividers: bool,
    collapsed_months_gen: u64,
}

impl GalleryCache {
    pub fn new() -> Self {
        Self {
            layout: None,
            layout_key: None,
            tag_lookup: HashMap::new(),
            tag_lookup_gen: 0,
            chip_widths: HashMap::new(),
            chip_effective: HashMap::new(),
            chip_path_prefix: HashMap::new(),
            chip_widths_gen: 0,
        }
    }
}

const HEADER_H: f32 = 40.0;
const HEADER_GAP_TOP: f32 = 14.0;

fn build_gallery_layout(
    skel: &[crate::db::queries::MediaSkeleton],
    sort: SortOrder,
    cols: usize,
    thumb_size: f32,
    gap: f32,
    show_date_dividers: bool,
    collapsed_months: &HashSet<String>,
) -> GalleryLayout {
    let media_row_h = thumb_size + gap;
    let group_by_date = show_date_dividers
        && matches!(sort, SortOrder::NewestFirst | SortOrder::OldestFirst);

    let mut rows: Vec<GalleryRow> = Vec::new();
    let mut y: f32 = 0.0;

    if !group_by_date {
        // Flat grid, no headers. Indices stored here are GLOBAL indices
        // into `state.media_skel` (and conceptually into the full ordered
        // result set) — the renderer translates them to the windowed
        // `state.media` Vec at draw time.
        for chunk_start in (0..skel.len()).step_by(cols.max(1)) {
            let chunk_end = (chunk_start + cols).min(skel.len());
            let indices: Vec<usize> = (chunk_start..chunk_end).collect();
            rows.push(GalleryRow { y, h: media_row_h, kind: RowKind::Items(indices) });
            y += media_row_h;
        }
        return GalleryLayout { rows, total_height: y };
    }

    // Date-grouped layout. Drives off the skeleton's `effective_date`
    // (already SQL-COALESCEd) so we don't have to load full MediaItems
    // just to know each item's bucket.
    let mut current_label: Option<String> = None;
    let mut group_indices: Vec<usize> = Vec::new();
    let mut header_idx: Option<usize> = None;

    let flush = |rows: &mut Vec<GalleryRow>,
                 group: &mut Vec<usize>,
                 y: &mut f32,
                 cols: usize,
                 row_h: f32| {
        for chunk in group.chunks(cols.max(1)) {
            rows.push(GalleryRow { y: *y, h: row_h, kind: RowKind::Items(chunk.to_vec()) });
            *y += row_h;
        }
        group.clear();
    };

    for (i, s) in skel.iter().enumerate() {
        let label = format_month_label(s.effective_date);
        if Some(&label) != current_label.as_ref() {
            flush(&mut rows, &mut group_indices, &mut y, cols, media_row_h);
            if current_label.is_some() { y += HEADER_GAP_TOP; }
            header_idx = Some(rows.len());
            rows.push(GalleryRow { y, h: HEADER_H, kind: RowKind::Header { label: label.clone(), count: 0 } });
            y += HEADER_H;
            current_label = Some(label);
        }
        // Count every item in the month — even when the month is collapsed
        // (the header shows how much is hidden inside).
        if let Some(hi) = header_idx {
            if let RowKind::Header { count, .. } = &mut rows[hi].kind { *count += 1; }
        }
        if !collapsed_months.contains(current_label.as_ref().unwrap()) {
            group_indices.push(i);
        }
    }
    flush(&mut rows, &mut group_indices, &mut y, cols, media_row_h);

    GalleryLayout { rows, total_height: y }
}

fn format_month_label(secs: i64) -> String {
    if secs <= 0 { return "Unknown date".to_string(); }
    use chrono::{DateTime, Local};
    match DateTime::from_timestamp(secs, 0) {
        Some(utc) => {
            let dt: DateTime<Local> = utc.into();
            dt.format("%B %Y").to_string()
        }
        None => "Unknown date".to_string(),
    }
}

/// Paint a date section header: a rotating chevron, the month title in
/// Segoe UI Semibold, and a muted item count. The whole row is a click
/// target (collapse/expand) with a soft hover fill.
fn paint_date_header(ui: &Ui, rect: Rect, label: &str, count: usize, collapsed: bool, hovered: bool) {
    let cy = rect.min.y + rect.height() / 2.0 + 2.0;

    if hovered {
        ui.painter().rect_filled(
            rect.shrink2(egui::vec2(0.0, 3.0)),
            theme::rounding(theme::ROUND_MD),
            Color32::from_white_alpha(6),
        );
    }

    // Chevron — rotates 90° between collapsed (→) and expanded (↓),
    // driven by the same animation clock as the collapse toggle.
    let t = ui.ctx().animate_bool_with_time(
        egui::Id::new(("month_chevron", label)),
        !collapsed,
        0.15,
    );
    let ccx = rect.min.x + 12.0;
    let angle = t * std::f32::consts::FRAC_PI_2; // 0 = →, π/2 = ↓
    let r = 4.6;
    // Chevron "V" pointing right at angle 0; rotate around its center.
    let base = [
        egui::vec2(-r * 0.45, -r),
        egui::vec2(r * 0.55, 0.0),
        egui::vec2(-r * 0.45, r),
    ];
    let (sin, cos) = angle.sin_cos();
    let pts: Vec<egui::Pos2> = base.iter()
        .map(|v| egui::pos2(
            ccx + v.x * cos - v.y * sin,
            cy + v.x * sin + v.y * cos,
        ))
        .collect();
    ui.painter().add(egui::Shape::line(
        pts,
        egui::Stroke::new(1.6, theme::TEXT_SECONDARY),
    ));

    // Month title — real Semibold, not a fake bold.
    let title_galley = ui.painter().layout_no_wrap(
        label.to_string(),
        theme::semibold(theme::FONT_SIZE_TITLE),
        theme::TEXT_PRIMARY,
    );
    let title_x = rect.min.x + 28.0;
    ui.painter().galley(
        egui::pos2(title_x, cy - title_galley.size().y / 2.0),
        title_galley.clone(),
        theme::TEXT_PRIMARY,
    );

    // Item count — quiet metadata after the title.
    let count_text = if count == 1 { "1 item".to_string() } else { format!("{} items", count) };
    ui.painter().text(
        egui::pos2(title_x + title_galley.size().x + 10.0, cy + 1.5),
        egui::Align2::LEFT_CENTER,
        count_text,
        egui::FontId::proportional(theme::FONT_SIZE_SMALL),
        theme::TEXT_MUTED,
    );
}

// ══════════════════════════════════════════════════════════════════
//  Gallery Grid
// ══════════════════════════════════════════════════════════════════

/// Translate keyboard input into selection changes. Handles arrows /
/// Home / End / Ctrl+A / Enter / Delete / Backspace.
///
/// Selection model:
///   • plain arrow / Home / End  → set selection to a single new item,
///     reset the shift-anchor to that item.
///   • Shift+arrow / Shift+Home/End → extend a range from the current
///     anchor (preserved across consecutive shift-presses) to the new
///     cursor position.
///   • Ctrl+A → select every item in the entire filtered set.
///   • Enter → open preview for the first selected item.
///   • Delete / Backspace (with non-empty selection) → request bulk
///     delete confirmation.
fn handle_gallery_keys(
    ctx: &egui::Context,
    state: &mut AppState,
    cols: usize,
    total: usize,
) {
    if total == 0 { return; }
    let mods = ctx.input(|i| i.modifiers);
    let cols = cols.max(1);

    // Ctrl+A: select all in filtered set, anchor at first item.
    if mods.ctrl && ctx.input(|i| i.key_pressed(egui::Key::A)) {
        state.selected_media_ids = state.media_skel.iter().map(|s| s.id).collect();
        state.selection_anchor = state.media_skel.first().map(|s| s.id);
        return;
    }

    // Rating hotkeys 0-5 — bulk-apply to every selected item. 0 clears
    // the rating. No effect when nothing is selected.
    if !mods.ctrl && !mods.alt && !state.selected_media_ids.is_empty() {
        let rating_pressed = [
            (egui::Key::Num0, 0),
            (egui::Key::Num1, 1),
            (egui::Key::Num2, 2),
            (egui::Key::Num3, 3),
            (egui::Key::Num4, 4),
            (egui::Key::Num5, 5),
        ].iter()
            .find(|(k, _)| ctx.input(|i| i.key_pressed(*k)))
            .map(|(_, v)| *v);
        if let Some(r) = rating_pressed {
            state.pending_action = Some(crate::app::PendingAction::SetRating {
                media_ids: state.selected_media_ids.clone(),
                rating: r,
            });
            return;
        }
    }


    // Delete / Backspace: open the bulk-delete confirmation modal.
    if (ctx.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)))
        && !state.selected_media_ids.is_empty()
    {
        state.confirm_bulk_delete = Some(state.selected_media_ids.clone());
        return;
    }

    // Escape: clear selection.
    if ctx.input(|i| i.key_pressed(egui::Key::Escape))
        && !state.selected_media_ids.is_empty()
    {
        state.selected_media_ids.clear();
        state.selection_anchor = None;
        return;
    }

    // Enter: open preview for first selected. Same effect as the
    // status-bar 👁 Open button.
    if ctx.input(|i| i.key_pressed(egui::Key::Enter))
        && !state.selected_media_ids.is_empty()
    {
        let id = state.selected_media_ids[0];
        if let Some(item) = state.media_by_id(id) {
            state.preview_media = Some(item);
        }
        return;
    }

    // Movement keys → compute target index.
    let target_delta: Option<MoveTarget> =
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft))   { Some(MoveTarget::Delta(-1)) }
        else if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight))  { Some(MoveTarget::Delta(1)) }
        else if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp))     { Some(MoveTarget::Delta(-(cols as isize))) }
        else if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown))   { Some(MoveTarget::Delta(cols as isize)) }
        else if ctx.input(|i| i.key_pressed(egui::Key::Home))        { Some(MoveTarget::First) }
        else if ctx.input(|i| i.key_pressed(egui::Key::End))         { Some(MoveTarget::Last) }
        else { None };

    let Some(target) = target_delta else { return };

    // "Cursor" position the user is moving from. Use the last item in the
    // current selection (most recently set by plain click/arrow) for the
    // most intuitive movement; fall back to the anchor; finally fall back
    // to position -1 (so a Right-arrow with no selection lands on item 0).
    let cur_idx: Option<usize> = state.selected_media_ids.last()
        .and_then(|&id| state.media_skel.iter().position(|s| s.id == id))
        .or_else(|| state.selection_anchor.and_then(|id| state.media_skel.iter().position(|s| s.id == id)));

    let target_idx: usize = match target {
        MoveTarget::First => 0,
        MoveTarget::Last  => total - 1,
        MoveTarget::Delta(d) => {
            let base = cur_idx.map(|i| i as isize).unwrap_or(-1);
            let new = base + d;
            new.clamp(0, total as isize - 1) as usize
        }
    };
    let target_id = state.media_skel[target_idx].id;

    if mods.shift {
        // Range from anchor to target. Anchor seeded from current
        // selection if missing.
        let anchor_id = state.selection_anchor
            .or_else(|| state.selected_media_ids.first().copied())
            .unwrap_or(target_id);
        let anchor_idx = state.media_skel.iter().position(|s| s.id == anchor_id)
            .unwrap_or(target_idx);
        if state.selection_anchor.is_none() {
            state.selection_anchor = Some(anchor_id);
        }
        let (lo, hi) = (anchor_idx.min(target_idx), anchor_idx.max(target_idx));
        state.selected_media_ids = state.media_skel[lo..=hi].iter().map(|s| s.id).collect();
    } else {
        state.selected_media_ids = vec![target_id];
        state.selection_anchor = Some(target_id);
    }
    state.scroll_to_idx = Some(target_idx);
}

enum MoveTarget {
    Delta(isize),
    First,
    Last,
}

pub fn show_gallery(
    ui: &mut Ui,
    state: &mut AppState,
    tex_mgr: &mut TextureManager,
    cache: &mut GalleryCache,
    show_date_dividers: bool,
) {
    let available = ui.available_size();
    let thumb_size = state.thumb_size;
    let gap = 12.0;
    let cols = ((available.x + gap) / (thumb_size + gap)).floor().max(1.0) as usize;
    // Layout drives off the skeleton (one entry per item in the full,
    // filtered result set) — NOT off `state.media`, which is just the
    // currently-loaded window.
    let total = state.media_skel.len();
    let row_height = thumb_size + gap;

    // Opt #1: Adaptive decode resolution
    tex_mgr.set_thumb_size(thumb_size);

    if total == 0 {
        // Proper empty state: a large quiet glyph, a semibold headline,
        // and a muted hint — centered in the card.
        let rect = ui.available_rect_before_wrap();
        let c = rect.center();
        ui.painter().text(
            egui::pos2(c.x, c.y - 44.0),
            egui::Align2::CENTER_CENTER,
            if theme::icons_available() { theme::ICON_PICTURE } else { "🖼" },
            theme::icon_font(52.0),
            Color32::from_white_alpha(26),
        );
        ui.painter().text(
            egui::pos2(c.x, c.y + 12.0),
            egui::Align2::CENTER_CENTER,
            "Nothing to show here",
            theme::semibold(theme::FONT_SIZE_HEADING),
            theme::TEXT_SECONDARY,
        );
        ui.painter().text(
            egui::pos2(c.x, c.y + 36.0),
            egui::Align2::CENTER_CENTER,
            "Add a folder from the sidebar, or loosen the active filters",
            egui::FontId::proportional(theme::FONT_SIZE_SMALL),
            theme::TEXT_MUTED,
        );
        return;
    }

    // Build the row layout once per frame. When sort is date-based we
    // interleave month-section headers between groups of media rows;
    // otherwise the gallery is a flat grid (no headers).
    //
    // Cached: only rebuild when one of the inputs that affects layout
    // actually changed. Window-drag and idle frames hit the cached path,
    // which avoids ~10k chrono ops + ~10k allocations per frame for large
    // libraries.
    let key = LayoutKey {
        media_gen: state.media_gen,
        sort: state.sort_order,
        cols,
        thumb_size_q: (thumb_size * 4.0) as u32,
        gap_q: (gap * 4.0) as u32,
        show_date_dividers,
        collapsed_months_gen: state.collapsed_months_gen,
    };
    if cache.layout_key.as_ref() != Some(&key) {
        cache.layout = Some(build_gallery_layout(
            &state.media_skel, state.sort_order, cols, thumb_size, gap, show_date_dividers,
            &state.collapsed_months,
        ));
        cache.layout_key = Some(key);
    }

    // ── Gallery keyboard nav ────────────────────────────────────────
    // Fires only when no modal/preview is active and no widget has
    // keyboard focus (so typing in the search field still works).
    let no_focus = ui.ctx().memory(|m| m.focused().is_none());
    let no_modal = state.preview_media.is_none()
        && !state.show_create_tag
        && state.confirm_delete.is_none()
        && state.confirm_bulk_delete.is_none()
        && state.rename_media.is_none()
        && state.new_folder_parent.is_none()
        && state.edit_tag.is_none();
    if no_focus && no_modal {
        handle_gallery_keys(ui.ctx(), state, cols, total);
    }
    // Pre-build the tag lookup cache too, before the closure borrows
    // overlap. Both caches are keyed on `tag_tree_gen` so they always
    // refresh together.
    if cache.tag_lookup_gen != state.tag_tree_gen {
        cache.tag_lookup.clear();
        fn flatten_tags(
            tags: &[crate::db::queries::Tag],
            out: &mut HashMap<i64, crate::db::queries::Tag>,
        ) {
            for t in tags {
                let mut bare = t.clone();
                bare.children.clear();
                out.insert(t.id, bare);
                flatten_tags(&t.children, out);
            }
        }
        flatten_tags(&state.tag_tree, &mut cache.tag_lookup);
        cache.tag_lookup_gen = state.tag_tree_gen;
    }
    if cache.chip_widths_gen != state.tag_tree_gen {
        cache.chip_widths.clear();
        cache.chip_effective.clear();
        cache.chip_path_prefix.clear();
        // Pre-resolve effective icon+color for every tag in one pass.
        // Each parent walk is O(depth); doing this once amortises the
        // cost across every visible chip on every frame until tags
        // change. Also fills the path-prefix cache used by the chip
        // leaf-filter so it doesn't have to `format!()` per chip.
        for (&id, tag) in cache.tag_lookup.iter() {
            // effective icon
            let icon = {
                let mut chosen: Option<String> = tag.icon.as_deref()
                    .filter(|s| !s.is_empty())
                    .map(String::from);
                let mut cur = tag.parent_id;
                while chosen.is_none() {
                    let Some(pid) = cur else { break };
                    match cache.tag_lookup.get(&pid) {
                        Some(p) => {
                            if let Some(s) = p.icon.as_deref().filter(|s| !s.is_empty()) {
                                chosen = Some(s.to_string());
                            }
                            cur = p.parent_id;
                        }
                        None => break,
                    }
                }
                chosen.unwrap_or_else(|| "🏷".to_string())
            };
            // effective color
            let color: Color32 = {
                use crate::ui::sidebar::tag_color32 as col;
                if tag.color.is_some() { col(tag) }
                else {
                    let mut found: Option<Color32> = None;
                    let mut cur = tag.parent_id;
                    while found.is_none() {
                        let Some(pid) = cur else { break };
                        match cache.tag_lookup.get(&pid) {
                            Some(p) => {
                                if p.color.is_some() { found = Some(col(p)); }
                                cur = p.parent_id;
                            }
                            None => break,
                        }
                    }
                    found.unwrap_or_else(|| col(tag))
                }
            };
            cache.chip_effective.insert(id, (icon, color));
            cache.chip_path_prefix.insert(id, format!("{}>", tag.full_path));
        }
        cache.chip_widths_gen = state.tag_tree_gen;
    }
    // Split-borrow disjoint cache fields so the closure can capture each
    // independently (Rust allows this when accessed via direct field syntax).
    let layout: &GalleryLayout = cache.layout.as_ref().expect("layout cache just populated");
    let tag_lookup: &HashMap<i64, crate::db::queries::Tag> = &cache.tag_lookup;
    let chip_widths: &mut HashMap<i64, (f32, f32)> = &mut cache.chip_widths;
    let chip_effective: &HashMap<i64, (String, Color32)> = &cache.chip_effective;
    let chip_path_prefix: &HashMap<i64, String> = &cache.chip_path_prefix;

    // Keyboard-nav scroll-into-view: translate the requested global idx
    // into a pending scroll offset that centres the row in the viewport.
    // Funnels through the existing `pending_gallery_scroll` mechanism so
    // the scroll-area-creation code below applies it once and clears it.
    if let Some(target_idx) = state.scroll_to_idx.take() {
        if let Some(row) = layout.rows.iter().find(|r| match &r.kind {
            RowKind::Items(idxs) => idxs.contains(&target_idx),
            _ => false,
        }) {
            let target_center_y = row.y + row.h * 0.5;
            let target_top = (target_center_y - available.y * 0.5).max(0.0);
            state.pending_gallery_scroll = Some(target_top);
        }
    }

    let mut scroll_area = egui::ScrollArea::vertical().auto_shrink(false);
    // Restore the saved scroll position on the first frame after a session
    // load. `take()` so subsequent frames don't keep snapping back.
    if let Some(off) = state.pending_gallery_scroll.take() {
        scroll_area = scroll_area.vertical_scroll_offset(off);
    }
    let scroll_out = scroll_area
        .show_viewport(ui, |ui, viewport| {
            ui.set_min_size(Vec2::new(available.x, layout.total_height));

            // ── Rubber-band drag-select ───────────────────────────────
            // Allocated FIRST (before thumbnails register their rects)
            // so subsequent thumb-rects win hit-testing for any click
            // that lands on them — this background only catches drags
            // that started on empty gallery space.
            let bg_rect = egui::Rect::from_min_size(
                ui.min_rect().min,
                Vec2::new(available.x, layout.total_height),
            );
            let bg_resp = ui.interact(
                bg_rect,
                ui.id().with("gallery_bg_dragselect"),
                Sense::click_and_drag(),
            );
            if bg_resp.clicked() && !bg_resp.dragged() {
                let mods = ui.ctx().input(|i| i.modifiers);
                if !mods.ctrl && !mods.shift {
                    state.selected_media_ids.clear();
                    state.selection_anchor = None;
                }
            }
            if bg_resp.drag_started() {
                if let Some(start) = bg_resp.interact_pointer_pos() {
                    let mods = ui.ctx().input(|i| i.modifiers);
                    let mode = if mods.ctrl {
                        crate::app::RubberbandMode::Add
                    } else {
                        crate::app::RubberbandMode::Replace
                    };
                    let initial = if mode == crate::app::RubberbandMode::Add {
                        state.selected_media_ids.clone()
                    } else {
                        Vec::new()
                    };
                    state.rubberband = Some(crate::app::RubberbandState {
                        start, end: start, initial_selection: initial, mode,
                    });
                    state.selection_anchor = None;
                }
            }
            if state.rubberband.is_some() && bg_resp.dragged() {
                if let Some(p) = ui.ctx().input(|i| i.pointer.hover_pos()) {
                    if let Some(rb) = state.rubberband.as_mut() { rb.end = p; }
                }
            }
            if bg_resp.drag_stopped() {
                state.rubberband = None;
            }
            // While the rubber-band is alive, recompute the selection
            // from the union of the band's rect and the per-thumb rects
            // (computed from layout — works for items outside the
            // virtualised viewport too).
            if let Some(rb) = state.rubberband.as_ref() {
                let band = egui::Rect::from_two_pos(rb.start, rb.end);
                let origin_x = ui.min_rect().min.x;
                let origin_y = ui.min_rect().min.y;
                let mut new_sel: Vec<i64> = if rb.mode == crate::app::RubberbandMode::Add {
                    rb.initial_selection.clone()
                } else {
                    Vec::new()
                };
                for row in &layout.rows {
                    let row_top = origin_y + row.y;
                    let row_bot = row_top + row.h;
                    if row_bot < band.min.y || row_top > band.max.y { continue; }
                    if let RowKind::Items(idxs) = &row.kind {
                        for (col, &gidx) in idxs.iter().enumerate() {
                            let tx = origin_x + col as f32 * (thumb_size + gap);
                            let trect = egui::Rect::from_min_size(
                                egui::pos2(tx, row_top),
                                Vec2::new(thumb_size, thumb_size),
                            );
                            if trect.intersects(band) {
                                let id = state.media_skel[gidx].id;
                                if !new_sel.contains(&id) { new_sel.push(id); }
                            }
                        }
                    }
                }
                state.selected_media_ids = new_sel;
                // Visual painting of the band rect happens AFTER all
                // thumbnails have rendered (see end of viewport closure)
                // so the translucent overlay sits on top of the cards.
            }

            // Find the visible row range. Linear scan is fine for ~hundreds
            // of rows; binary search would be a micro-optimization.
            let mut first_visible = layout.rows.len();
            for (i, r) in layout.rows.iter().enumerate() {
                if r.y + r.h >= viewport.min.y { first_visible = i; break; }
            }
            let mut last_visible = first_visible;
            for (i, r) in layout.rows.iter().enumerate().skip(first_visible) {
                if r.y > viewport.max.y { break; }
                last_visible = i + 1;
            }

            let viewport_center_y = (viewport.min.y + viewport.max.y) * 0.5;

            // Scroll pre-fetch — push 3 rows past the viewport in the
            // current scroll direction to the worker queue.
            let scroll_dir = tex_mgr.update_scroll(viewport.min.y);
            let pf_count = 3;
            let (pf_start, pf_end) = if scroll_dir >= 0.0 {
                (last_visible, (last_visible + pf_count).min(layout.rows.len()))
            } else {
                (first_visible.saturating_sub(pf_count), first_visible)
            };

            // Determine the global media-skeleton index range that the
            // visible + prefetch rows cover, then slide `state.media` so
            // those items are loaded.
            let mut min_global: Option<usize> = None;
            let mut max_global: Option<usize> = None;
            for r in &layout.rows[first_visible..last_visible] {
                if let RowKind::Items(idxs) = &r.kind {
                    for &g in idxs {
                        min_global = Some(min_global.map_or(g, |m: usize| m.min(g)));
                        max_global = Some(max_global.map_or(g, |m: usize| m.max(g)));
                    }
                }
            }
            for r in &layout.rows[pf_start..pf_end] {
                if let RowKind::Items(idxs) = &r.kind {
                    for &g in idxs {
                        min_global = Some(min_global.map_or(g, |m: usize| m.min(g)));
                        max_global = Some(max_global.map_or(g, |m: usize| m.max(g)));
                    }
                }
            }
            if let (Some(lo), Some(hi)) = (min_global, max_global) {
                state.ensure_media_window(lo, hi + 1);
            }

            // Now build the prefetch list using locally-resolved items.
            let mut prefetch_items: Vec<&MediaItem> = Vec::new();
            for r in &layout.rows[pf_start..pf_end] {
                if let RowKind::Items(idxs) = &r.kind {
                    for &g in idxs {
                        if let Some(local) = state.local_media_idx(g) {
                            prefetch_items.push(&state.media[local]);
                        }
                    }
                }
            }
            let visible_count = last_visible.saturating_sub(first_visible) as u32;
            tex_mgr.prefetch(&prefetch_items, visible_count + 1);

            let ctx = ui.ctx().clone();

            // Hoisted per-frame input lookups. Doing these once and
            // reusing across all visible thumbs (instead of per-thumb)
            // avoids repeatedly locking egui's input state — that lock
            // contention was a meaningful share of per-frame cost when
            // the UI is steady-state (window drag, idle hover).
            let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
            let dragging_tag_payload = egui::DragAndDrop::has_payload_of_type::<i64>(&ctx);
            let modifiers = ctx.input(|i| i.modifiers);

            // Selected-id O(1) lookup. The Vec scan was O(n) per thumb,
            // and grows quadratic when the user has many items selected.
            let selected_set: HashSet<i64> = state.selected_media_ids.iter().copied().collect();

            for row_layout in &layout.rows[first_visible..last_visible] {
                let row_y_screen = ui.min_rect().min.y + row_layout.y;
                let indices: &[usize] = match &row_layout.kind {
                    RowKind::Header { label, count } => {
                        let header_rect = Rect::from_min_size(
                            egui::pos2(ui.min_rect().min.x, row_y_screen),
                            Vec2::new(available.x, row_layout.h),
                        );
                        let collapsed = state.collapsed_months.contains(label);
                        // Interact BEFORE painting so the hover fill appears
                        // in the same frame the pointer arrives.
                        let resp = ui.interact(
                            header_rect,
                            ui.id().with(("month_hdr", label.as_str())),
                            Sense::click(),
                        );
                        paint_date_header(ui, header_rect, label, *count, collapsed, resp.hovered());
                        if resp.clicked() {
                            if collapsed {
                                state.collapsed_months.remove(label);
                            } else {
                                state.collapsed_months.insert(label.clone());
                            }
                            state.collapsed_months_gen += 1;
                        }
                        continue;
                    }
                    RowKind::Items(idxs) => idxs,
                };
                // Per-row image-loading priority: rows closer to the
                // viewport center get higher priority (lower number).
                let row_priority = ((row_layout.y + row_layout.h * 0.5
                                     - viewport_center_y).abs()
                                    / row_height).round() as u32;
                for (col, &global_idx) in indices.iter().enumerate() {
                    let x = col as f32 * (thumb_size + gap);
                    let rect = Rect::from_min_size(
                        egui::pos2(ui.min_rect().min.x + x, row_y_screen),
                        Vec2::new(thumb_size, thumb_size),
                    );
                    // Translate global skeleton index → local windowed index.
                    // If the item isn't in the loaded window yet (transient
                    // state during a fast scroll), paint a neutral placeholder
                    // card and skip the rest of the per-thumb logic — the
                    // window-load triggered above will populate it on the
                    // next frame.
                    let Some(local_idx) = state.local_media_idx(global_idx) else {
                        ui.painter().rect_filled(
                            rect,
                            Rounding::same(theme::THUMB_ROUND),
                            theme::SURFACE_ELEVATED,
                        );
                        continue;
                    };
                    let item = state.media[local_idx].clone();

                    // Per-thumbnail animations. Hover drives a subtle
                    // scale + shadow lift; selection drives the
                    // Google-Photos-style inset (image shrinks inward,
                    // revealing an accent backplate). Click target stays
                    // the original `rect` so layout doesn't shift.
                    let hover_id = ui.id().with(("thumb_hover", item.id));
                    let hovered_now = pointer_pos.map_or(false, |p| rect.contains(p));
                    let hover_t = ui.ctx().animate_bool_with_time(hover_id, hovered_now, 0.12);

                    let is_selected = selected_set.contains(&item.id);
                    let sel_t = ui.ctx().animate_bool_with_time(
                        ui.id().with(("thumb_sel", item.id)), is_selected, 0.14,
                    );

                    let scale = 1.0 + 0.018 * hover_t;
                    let visual_rect = Rect::from_center_size(rect.center(), rect.size() * scale);
                    let radius = theme::THUMB_ROUND;
                    let round = Rounding::same(radius);

                    // The image itself — inset inward while selected.
                    let img_rect = visual_rect.shrink(7.0 * sel_t);

                    // ── Soft shadow (real blur, not an offset box) ──────
                    let shadow_alpha = (42.0 + 60.0 * hover_t) as u8;
                    theme::paint_soft_shadow(
                        ui.painter(), visual_rect, round,
                        2.0 + 3.0 * hover_t,
                        10.0 + 8.0 * hover_t,
                        Color32::from_black_alpha(shadow_alpha),
                    );

                    // Accent backplate revealed behind a selected image.
                    if sel_t > 0.01 {
                        ui.painter().rect_filled(visual_rect, round, theme::accent_alpha(0.22 * sel_t));
                    }

                    let priority = row_priority;

                    // ── Thumbnail image — cover-cropped + rounded ───────
                    // Copy out (id, size) so the tex_mgr borrow ends before
                    // the micro-texture call below.
                    let fade_id = ui.id().with(("thumb_fade", item.id));
                    let main_tex = tex_mgr.get_or_load(&item, priority, &ctx)
                        .map(|t| (t.id(), t.size_vec2()));
                    if let Some((tex_id, tex_size)) = main_tex {
                        // Fade-in only happens when the placeholder was
                        // visible first (the anim registers `false` below);
                        // already-cached textures snap in instantly.
                        let fade = ui.ctx().animate_bool_with_time(fade_id, true, 0.22);
                        if fade < 0.999 {
                            ui.painter().rect_filled(img_rect, round, theme::SURFACE_ELEVATED);
                            if let Some((mid, msize)) = tex_mgr.get_micro_texture(&item, &ctx)
                                .map(|t| (t.id(), t.size_vec2()))
                            {
                                theme::paint_cover_image(
                                    ui.painter(), img_rect, msize, mid, round, Color32::WHITE,
                                );
                            }
                        }
                        theme::paint_cover_image(
                            ui.painter(), img_rect, tex_size, tex_id, round,
                            Color32::WHITE.gamma_multiply(fade),
                        );
                    } else {
                        // Register the fade animation at 0 while loading so
                        // the real texture eases in when it lands.
                        ui.ctx().animate_bool_with_time(fade_id, false, 0.22);
                        ui.painter().rect_filled(img_rect, round, theme::SURFACE_ELEVATED);
                        if let Some((mid, msize)) = tex_mgr.get_micro_texture(&item, &ctx)
                            .map(|t| (t.id(), t.size_vec2()))
                        {
                            theme::paint_cover_image(
                                ui.painter(), img_rect, msize, mid, round, Color32::WHITE,
                            );
                        } else {
                            let glyph = if item.media_type == "video" {
                                if theme::icons_available() { theme::ICON_VIDEO } else { "\u{1F3AC}" }
                            } else if item.media_type == "audio" {
                                if theme::icons_available() { theme::ICON_AUDIO } else { "\u{1F3B5}" }
                            } else if theme::icons_available() { theme::ICON_PICTURE } else { "\u{1F5BC}" };
                            ui.painter().text(
                                img_rect.center(), egui::Align2::CENTER_CENTER,
                                glyph,
                                theme::icon_font(22.0),
                                Color32::from_white_alpha(34),
                            );
                        }
                    }

                    // Gentle brightness lift while hovered.
                    if hover_t > 0.01 {
                        ui.painter().rect_filled(
                            img_rect, round,
                            Color32::from_white_alpha((9.0 * hover_t) as u8),
                        );
                    }

                    // ── Media-type badge — bottom-right play/audio disc ──
                    if item.media_type == "video" || item.media_type == "audio" {
                        let r = 11.0;
                        let c = img_rect.right_bottom() + Vec2::new(-r - 7.0, -r - 7.0);
                        ui.painter().circle_filled(c, r, Color32::from_black_alpha(150));
                        let (glyph, fallback, dx) = if item.media_type == "video" {
                            (theme::ICON_PLAY, "▶", 0.8)
                        } else {
                            (theme::ICON_AUDIO, "♪", 0.0)
                        };
                        ui.painter().text(
                            c + Vec2::new(dx, 0.0),
                            egui::Align2::CENTER_CENTER,
                            if theme::icons_available() { glyph } else { fallback },
                            theme::icon_font(9.5),
                            Color32::WHITE,
                        );
                    }

                    // ── Rating badge — bottom-left pill (star + number) ──
                    if item.rating > 0 {
                        let font = egui::FontId::proportional(10.0);
                        let num = item.rating.to_string();
                        let num_w = ui.fonts(|f| f.layout_no_wrap(num.clone(), font.clone(), Color32::WHITE).size().x);
                        let h = 18.0;
                        let star_w = 10.0;
                        let w = 8.0 + star_w + 4.0 + num_w + 8.0;
                        let badge_rect = Rect::from_min_size(
                            img_rect.left_bottom() + Vec2::new(7.0, -h - 7.0),
                            Vec2::new(w, h),
                        );
                        ui.painter().rect_filled(badge_rect, Rounding::same(h * 0.5), Color32::from_black_alpha(150));
                        ui.painter().text(
                            egui::pos2(badge_rect.min.x + 8.0, badge_rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            if theme::icons_available() { theme::ICON_STAR } else { "★" },
                            theme::icon_font(9.0),
                            theme::STAR,
                        );
                        ui.painter().text(
                            egui::pos2(badge_rect.max.x - 8.0, badge_rect.center().y),
                            egui::Align2::RIGHT_CENTER,
                            num, font, Color32::WHITE,
                        );
                    }

                    // ── Tag chips (top-left) ────────────────────────────
                    let mut chip_ctrl_clicked: Option<i64> = None;
                    if let Some(item_tags) = state.media_tags.get(&item.id) {
                        if !item_tags.is_empty() {
                            chip_ctrl_clicked = draw_thumb_tag_chips(
                                ui, img_rect, item.id, item_tags,
                                tag_lookup, chip_widths,
                                chip_effective, chip_path_prefix,
                                pointer_pos,
                                &modifiers,
                            );
                        }
                    }

                    // ── Selection ring + checkmark badge ────────────────
                    // Animated with `sel_t`: the ring fades in and the
                    // badge scales up as the image insets inward.
                    if sel_t > 0.01 {
                        ui.painter().rect_stroke(
                            visual_rect, round,
                            egui::Stroke::new(2.0, theme::accent_alpha(0.95 * sel_t)),
                        );
                        let badge_r = 10.0 * sel_t;
                        let badge_c = visual_rect.right_top() + Vec2::new(-13.0, 13.0);
                        ui.painter().circle_filled(badge_c, badge_r + 2.0, Color32::from_black_alpha((110.0 * sel_t) as u8));
                        ui.painter().circle_filled(badge_c, badge_r, theme::accent());
                        if sel_t > 0.5 {
                            ui.painter().text(
                                badge_c, egui::Align2::CENTER_CENTER,
                                if theme::icons_available() { theme::ICON_CHECK } else { "✓" },
                                theme::icon_font(10.0 * sel_t),
                                Color32::WHITE,
                            );
                        }
                    }

                    let resp = ui.allocate_rect(rect, Sense::click_and_drag());

                    // Drop target: if a tag is being dragged from the sidebar
                    // and gets released over this thumbnail, assign it.
                    // (Hoisted `dragging_tag_payload` and `pointer_pos` —
                    // skip the per-thumb DnD/input lookup entirely.)
                    if dragging_tag_payload {
                        let hot = pointer_pos.map_or(false, |p| rect.contains(p));
                        if hot {
                            ui.painter().rect_filled(
                                visual_rect, round,
                                theme::accent_alpha(0.14),
                            );
                            ui.painter().rect_stroke(
                                visual_rect, round,
                                egui::Stroke::new(2.5, theme::accent_hover()),
                            );
                        }
                    }
                    if let Some(payload) = resp.dnd_release_payload::<i64>() {
                        let tag_id = *payload;
                        let target_ids: Vec<i64> = if state.selected_media_ids.contains(&item.id) {
                            state.selected_media_ids.clone()
                        } else { vec![item.id] };
                        state.pending_action = Some(crate::app::PendingAction::AssignTags(
                            target_ids, vec![tag_id],
                        ));
                    }

                    // Ctrl+click on a tag chip → remove that tag (skip
                    // the normal thumbnail selection logic for this click).
                    if let Some(remove_tag_id) = chip_ctrl_clicked {
                        state.pending_action = Some(crate::app::PendingAction::RemoveTags(
                            vec![item.id], vec![remove_tag_id],
                        ));
                    } else if resp.clicked() {
                        if modifiers.shift {
                            // Range-select from the current anchor to this
                            // item. If no anchor yet, seed it from the
                            // first item already in the selection (or
                            // this item, which collapses to a 1-item
                            // selection).
                            let anchor_id = state.selection_anchor
                                .or_else(|| state.selected_media_ids.first().copied())
                                .unwrap_or(item.id);
                            if state.selection_anchor.is_none() {
                                state.selection_anchor = Some(anchor_id);
                            }
                            let a_idx = state.media_skel.iter().position(|s| s.id == anchor_id);
                            let t_idx = state.media_skel.iter().position(|s| s.id == item.id);
                            if let (Some(a), Some(t)) = (a_idx, t_idx) {
                                let (lo, hi) = (a.min(t), a.max(t));
                                state.selected_media_ids = state.media_skel[lo..=hi]
                                    .iter().map(|s| s.id).collect();
                            }
                        } else if modifiers.ctrl {
                            if is_selected { state.selected_media_ids.retain(|&id| id != item.id); }
                            else { state.selected_media_ids.push(item.id); }
                            // Update anchor so a subsequent Shift+click
                            // extends from this newly-toggled item.
                            state.selection_anchor = Some(item.id);
                        } else {
                            state.selected_media_ids = vec![item.id];
                            state.selection_anchor = Some(item.id);
                        }
                    }
                    if resp.double_clicked() {
                        state.preview_media = Some(item.clone());
                    }

                    // ── Drag-out to external apps ─────────────────────
                    if resp.drag_started() && !modifiers.shift && !modifiers.ctrl {
                        let drag_ids: Vec<i64> = if state.selected_media_ids.contains(&item.id) {
                            state.selected_media_ids.clone()
                        } else {
                            vec![item.id]
                        };
                        let paths: Vec<String> = drag_ids.iter()
                            .filter_map(|&id| {
                                state.media.iter().find(|m| m.id == id).map(|m| m.full_path.clone())
                            })
                            .collect();
                        if !paths.is_empty() {
                            crate::drag::start_file_drag(&paths);
                        }
                    }

                    // ── Right-click context menu (merged) ─────────────
                    let ctx_item_id = item.id;
                    let tags_for_menu = state.tag_tree.clone();
                    resp.context_menu(|ui| {
                        let selected_ids: Vec<i64> =
                            if state.selected_media_ids.contains(&ctx_item_id) {
                                state.selected_media_ids.clone()
                            } else { vec![ctx_item_id] };

                        ui.label(if selected_ids.len() == 1 {
                            "1 item selected".to_string()
                        } else {
                            format!("{} items selected", selected_ids.len())
                        });
                        ui.separator();

                        if ui.button("👁 Open preview").clicked() {
                            state.preview_media = Some(item.clone());
                            ui.close_menu();
                        }
                        if ui.button("🔗 Open in default app").clicked() {
                            state.pending_action = Some(crate::app::PendingAction::OpenExternal { media_id: ctx_item_id });
                            ui.close_menu();
                        }
                        if ui.button("📂 Show in Explorer").clicked() {
                            state.pending_action = Some(crate::app::PendingAction::ShowInExplorer { media_id: ctx_item_id });
                            ui.close_menu();
                        }
                        if ui.button("🤖 AI Suggest Tags").clicked() {
                            state.ai_request_media_id = Some(ctx_item_id);
                            ui.close_menu();
                        }
                        ui.separator();

                        fn show_tags_menu(
                            ui: &mut egui::Ui, tags: &[crate::db::queries::Tag],
                            media_ids: &[i64], action: &mut Option<crate::app::PendingAction>,
                        ) {
                            for tag in tags {
                                if ui.button(format!("🏷 {}", tag.name)).clicked() {
                                    *action = Some(crate::app::PendingAction::AssignTags(
                                        media_ids.to_vec(), vec![tag.id],
                                    ));
                                    ui.close_menu();
                                }
                                if !tag.children.is_empty() {
                                    show_tags_menu(ui, &tag.children, media_ids, action);
                                }
                            }
                        }
                        if !tags_for_menu.is_empty() {
                            ui.menu_button("Assign tag", |ui| {
                                show_tags_menu(ui, &tags_for_menu, &selected_ids, &mut state.pending_action);
                            });
                        }

                        ui.menu_button("➡ Move to", |ui| {
                            show_move_to_menu(ui, state, ctx_item_id);
                        });

                        ui.menu_button("⭐ Rate", |ui| {
                            for r in 0..=5 {
                                let label = if r == 0 { "☆ No rating".to_string() } else { "★".repeat(r as usize) };
                                if ui.button(label).clicked() {
                                    state.pending_action = Some(crate::app::PendingAction::SetRating { media_ids: selected_ids.clone(), rating: r });
                                    ui.close_menu();
                                }
                            }
                        });
                        ui.separator();
                        if ui.button("🗑 Delete").clicked() {
                            if state.selected_media_ids.contains(&ctx_item_id) && state.selected_media_ids.len() > 1 {
                                state.confirm_bulk_delete = Some(state.selected_media_ids.clone());
                            } else {
                                state.confirm_delete = Some(ctx_item_id);
                            }
                            ui.close_menu();
                        }
                    });

                    // ── Hover overlay ───────────────────────────────────
                    // Soft gradient fades in with the hover animation;
                    // surfaces filename + (dimensions · date) when there's
                    // room, otherwise just the filename.
                    if hover_t > 0.01 {
                        let strip_h = (img_rect.height() * 0.45).min(54.0).max(20.0);
                        let strip_rect = Rect::from_min_max(
                            egui::pos2(img_rect.min.x, img_rect.max.y - strip_h),
                            img_rect.max,
                        );
                        let bottom_alpha = (200.0 * hover_t) as u8;
                        paint_hover_scrim(ui.painter(), strip_rect, radius, bottom_alpha);
                        let text_alpha = (250.0 * hover_t) as u8;
                        let text_col = Color32::from_rgba_unmultiplied(255, 255, 255, text_alpha);
                        let meta_alpha = (200.0 * hover_t) as u8;
                        let meta_col = Color32::from_rgba_unmultiplied(220, 220, 220, meta_alpha);
                        let pad = 8.0;
                        if strip_h >= 36.0 {
                            // Two-line layout: meta is anchored to the
                            // bottom of the strip, filename sits a single
                            // line height above it. Using the same
                            // anchor (LEFT_BOTTOM) for both keeps them
                            // from overlapping regardless of font metrics.
                            let mut meta_parts: Vec<String> = Vec::new();
                            if let (Some(w), Some(h)) = (item.width, item.height) {
                                meta_parts.push(format!("{}×{}", w, h));
                            }
                            let date_secs = item.date_taken.unwrap_or(item.modified_at);
                            if date_secs > 0 {
                                meta_parts.push(format_short_date(date_secs));
                            }
                            let meta_baseline_y = strip_rect.max.y - pad;
                            if !meta_parts.is_empty() {
                                ui.painter().text(
                                    egui::pos2(strip_rect.min.x + pad, meta_baseline_y),
                                    egui::Align2::LEFT_BOTTOM,
                                    meta_parts.join(" · "),
                                    egui::FontId::proportional(10.0),
                                    meta_col,
                                );
                            }
                            // Filename sits one line + small gap above
                            // the metadata baseline.
                            let filename_baseline_y = if meta_parts.is_empty() {
                                meta_baseline_y
                            } else {
                                meta_baseline_y - 14.0
                            };
                            ui.painter().text(
                                egui::pos2(strip_rect.min.x + pad, filename_baseline_y),
                                egui::Align2::LEFT_BOTTOM,
                                &item.filename,
                                egui::FontId::proportional(11.5),
                                text_col,
                            );
                        } else {
                            // Single-line fallback for very small thumbs
                            ui.painter().text(
                                strip_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                &item.filename,
                                egui::FontId::proportional(10.0),
                                text_col,
                            );
                        }
                    }

                }
            }
            // Rubber-band overlay — painted last so it sits on top of
            // every thumbnail card in the same draw layer.
            if let Some(rb) = state.rubberband.as_ref() {
                let band = egui::Rect::from_two_pos(rb.start, rb.end);
                let accent = theme::accent();
                let fill = egui::Color32::from_rgba_unmultiplied(
                    accent.r(), accent.g(), accent.b(), 50,
                );
                ui.painter().rect_filled(band, egui::Rounding::same(2.0), fill);
                ui.painter().rect_stroke(
                    band, egui::Rounding::same(2.0),
                    egui::Stroke::new(1.0, accent),
                );
            }
        });
    // Capture the current scroll offset so on_exit can persist it.
    state.gallery_scroll = scroll_out.state.offset.y;
}

/// Build the recursive "Move to" submenu: every registered library folder,
/// plus every subdirectory under it, expanded lazily as the user navigates.
/// Each leaf row issues a `MoveMediaTo` pending action.
fn show_move_to_menu(ui: &mut Ui, state: &mut AppState, media_id: i64) {
    let folders = state.folders.clone();
    if folders.is_empty() {
        ui.colored_label(Color32::GRAY, "No library folders");
        return;
    }
    for folder in &folders {
        // Each root folder gets a "(here)" leaf for the root itself, plus a
        // submenu for its subdirs. Implemented as a top-level menu_button so
        // hovering it reveals the children.
        let label = format!("📁 {}", folder.name);
        ui.menu_button(label, |ui| {
            if ui.button("📥 Move here").clicked() {
                state.pending_action = Some(crate::app::PendingAction::MoveMediaTo {
                    media_id, dest_dir: folder.path.clone(),
                });
                ui.close_menu();
            }
            ui.separator();
            show_subdir_move_menu(ui, state, media_id, &folder.path);
        });
    }
}

fn show_subdir_move_menu(ui: &mut Ui, state: &mut AppState, media_id: i64, parent: &str) {
    let subdirs = state.get_subdirs(parent);
    if subdirs.is_empty() {
        ui.colored_label(Color32::GRAY, "(no subfolders)");
        return;
    }
    for (name, path, has_children) in subdirs {
        if has_children {
            ui.menu_button(format!("📂 {}", name), |ui| {
                if ui.button("📥 Move here").clicked() {
                    state.pending_action = Some(crate::app::PendingAction::MoveMediaTo {
                        media_id, dest_dir: path.clone(),
                    });
                    ui.close_menu();
                }
                ui.separator();
                show_subdir_move_menu(ui, state, media_id, &path);
            });
        } else {
            if ui.button(format!("📂 {}", name)).clicked() {
                state.pending_action = Some(crate::app::PendingAction::MoveMediaTo {
                    media_id, dest_dir: path.clone(),
                });
                ui.close_menu();
            }
        }
    }
}

/// Render a row of small tag chips inside the top-left of a thumbnail.
/// Shows up to 4 chips, then a "+N" pill for the overflow. A chip expands
/// horizontally to reveal the tag name when hovered.
///
/// Smart rollup behavior:
///   • If the assigned set contains both an ancestor and a descendant
///     (e.g. "People" and "People>Me"), the ancestor is hidden — the
///     descendant already implies it.
///   • Each surviving chip uses the icon/color of its tag if set, otherwise
///     walks up the parent chain (via `tag_lookup`) and uses the first
///     ancestor with a customised icon/color.
///   • Hovering a chip surfaces the full hierarchical path
///     ("People > Me") as a tooltip.
/// Draw tag chips on a thumbnail. Returns `Some(tag_id)` when the user
/// Ctrl+clicks a chip (= remove that tag), otherwise `None`.
fn draw_thumb_tag_chips(
    ui: &mut Ui,
    thumb_rect: Rect,
    item_id: i64,
    tags: &[crate::db::queries::Tag],
    _tag_lookup: &std::collections::HashMap<i64, crate::db::queries::Tag>,
    chip_widths: &mut std::collections::HashMap<i64, (f32, f32)>,
    chip_effective: &std::collections::HashMap<i64, (String, Color32)>,
    chip_path_prefix: &std::collections::HashMap<i64, String>,
    pointer_pos: Option<egui::Pos2>,
    modifiers: &egui::Modifiers,
) -> Option<i64> {
    // ── Step 1: filter to "leaves" within the assigned set ──
    // Uses the cached "{full_path}>" strings (one per tag, built once on
    // tag_tree change) so we don't `format!()` per chip per frame.
    let leaves: Vec<&crate::db::queries::Tag> = tags
        .iter()
        .filter(|t| {
            // Per-tag "display" toggle — hide chips for tags the user
            // doesn't want on thumbnails. Rollup still treats hidden
            // tags as absent, so an ancestor with display=true can
            // surface in their place.
            if !t.display { return false; }
            let Some(prefix) = chip_path_prefix.get(&t.id) else {
                // Tag not in cache (e.g. mid-rebuild); fall back to no-rollup.
                return true;
            };
            !tags.iter().any(|other| {
                other.display && other.full_path.as_str().starts_with(prefix.as_str())
            })
        })
        .collect();

    let mut ctrl_clicked_tag: Option<i64> = None;
    let primary_clicked = ui.ctx().input(|i| i.pointer.primary_clicked());

    let max_visible = 4;
    let pad = 4.0;
    let chip_h = 20.0;
    let gap = 3.0;
    let mut x = thumb_rect.min.x + pad;
    let y = thumb_rect.min.y + pad;
    let font = egui::FontId::proportional(13.0);
    let max_x = thumb_rect.max.x - pad;

    // Use the parent ui's font system (matches the preview chips) by going
    // through `ui.fonts(...)` for layout instead of `painter().layout_no_wrap`.
    // This ensures the same fallback chain (incl. our installed emoji fonts)
    // is consulted, fixing the "icon shows as circle" issue.
    // Default fallback for tags missing from the effective cache (e.g.
    // brief gap between assigning a brand-new tag and the next cache rebuild).
    let fallback_icon = "🏷";
    let fallback_color = Color32::from_rgb(120, 120, 130);

    for (i, tag) in leaves.iter().take(max_visible).enumerate() {
        // O(1) cached lookup — was previously two parent-chain walks per
        // chip per frame.
        let (icon, bg) = match chip_effective.get(&tag.id) {
            Some((s, c)) => (s.as_str(), *c),
            None => (fallback_icon, fallback_color),
        };
        let text_col = if (bg.r() as u32 + bg.g() as u32 + bg.b() as u32) > 380 {
            Color32::BLACK
        } else {
            Color32::WHITE
        };

        let chip_id = ui.id().with(("thumb_tag_chip", item_id, i));
        let was_hovered = ui.memory(|m| m.data.get_temp::<bool>(chip_id).unwrap_or(false));
        let t = ui.ctx().animate_bool_with_time(chip_id, was_hovered, 0.18);

        // Show the full hierarchy path (e.g. "People>Me") in the expanded
        // label so the user can disambiguate same-named leaves under
        // different parents. Borrow rather than clone — was previously a
        // String alloc per chip per frame.
        let display_name: &str = tag.full_path.as_str();
        // Cache font-shaped widths per tag id. egui's `f.layout_no_wrap`
        // is one of the more expensive per-frame operations and was
        // running ~2× per chip × ~30 visible thumbs every frame. Cache
        // is invalidated on `tag_tree_gen` so renames / icon edits take
        // effect immediately.
        let (icon_w, name_w) = *chip_widths.entry(tag.id).or_insert_with(|| {
            let iw = ui.fonts(|f| f.layout_no_wrap(icon.to_string(), font.clone(), text_col).size().x);
            let nw = ui.fonts(|f| f.layout_no_wrap(display_name.to_string(), font.clone(), text_col).size().x);
            (iw, nw)
        });
        let collapsed_w = (icon_w + 12.0).max(chip_h);
        let expanded_w = icon_w + name_w + 18.0;
        let chip_w = collapsed_w + (expanded_w - collapsed_w) * t;
        if x + collapsed_w > max_x { break; }

        // When collapsed, clamp to thumbnail; when fully expanded, allow
        // the pill to extend past the thumbnail edge so the full path is
        // readable without a separate tooltip.
        let chip_w = if t < 0.99 {
            chip_w.min(max_x - x)
        } else {
            chip_w
        };

        let chip_rect = Rect::from_min_size(egui::pos2(x, y), Vec2::new(chip_w, chip_h));

        // When expanding, paint on the Tooltip layer so the pill renders
        // above neighbouring thumbnails. Collapsed chips stay on the
        // normal layer to avoid unnecessary overdraw.
        let painter = if t > 0.01 {
            ui.ctx().layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                ui.id().with(("chip_fg", item_id, i)),
            ))
        } else {
            ui.painter().clone()
        };

        // Soft drop-shadow under the pill (no hard outline).
        let shadow_rect = chip_rect.translate(Vec2::new(0.0, 1.5));
        painter.rect_filled(shadow_rect, Rounding::same(chip_h * 0.5), Color32::from_black_alpha(60));
        painter.rect_filled(chip_rect, Rounding::same(chip_h * 0.5), bg);

        // Icon — slides from chip center (collapsed) to fixed left position (expanded).
        let icon_x_collapsed = chip_rect.center().x;
        let icon_x_expanded = chip_rect.min.x + 6.0 + icon_w * 0.5;
        let icon_x = icon_x_collapsed + (icon_x_expanded - icon_x_collapsed) * t;
        painter.text(
            egui::pos2(icon_x, chip_rect.center().y),
            egui::Align2::CENTER_CENTER, icon, font.clone(), text_col,
        );

        // Name fades in alongside the expansion.
        if t > 0.01 {
            let alpha = (t * 255.0) as u8;
            let col = Color32::from_rgba_unmultiplied(text_col.r(), text_col.g(), text_col.b(), alpha);
            let name_pos = egui::pos2(
                chip_rect.min.x + 6.0 + icon_w + 6.0 + name_w * 0.5,
                chip_rect.center().y,
            );
            let text_painter = painter.with_clip_rect(chip_rect);
            text_painter.text(name_pos, egui::Align2::CENTER_CENTER, &display_name, font.clone(), col);
        }

        // Use the hoisted pointer position — `ctx.input(|i| ...)` on
        // every chip × every visible thumb was a hidden hot-path.
        let now_hovered = pointer_pos.map_or(false, |p| chip_rect.contains(p));
        if now_hovered != was_hovered {
            ui.memory_mut(|m| m.data.insert_temp(chip_id, now_hovered));
            ui.ctx().request_repaint();
        }

        // Ctrl+click on a chip → remove this tag.
        if now_hovered && modifiers.ctrl {
            ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
            if primary_clicked {
                ctrl_clicked_tag = Some(tag.id);
            }
        }

        x += chip_w + gap;
    }

    let overflow = leaves.len().saturating_sub(max_visible);
    if overflow > 0 {
        let label = format!("+{}", overflow);
        let text_w = ui.fonts(|f| f.layout_no_wrap(label.clone(), font.clone(), Color32::WHITE).size().x);
        let chip_w = text_w + 10.0;
        if x + chip_w <= max_x {
            let chip_rect = Rect::from_min_size(egui::pos2(x, y), Vec2::new(chip_w, chip_h));
            ui.painter().rect_filled(chip_rect, Rounding::same(chip_h * 0.5), Color32::from_black_alpha(180));
            ui.painter().text(chip_rect.center(), egui::Align2::CENTER_CENTER, label, font, Color32::WHITE);
            let overflow_hovered = pointer_pos.map_or(false, |p| chip_rect.contains(p));
            if overflow_hovered {
                if modifiers.ctrl {
                    // Ctrl+hover on overflow: show interactive tooltip
                    // with each hidden tag clickable for removal.
                    ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
                    egui::show_tooltip(
                        ui.ctx(),
                        ui.layer_id(),
                        ui.id().with(("thumb_tag_overflow_tip", item_id)),
                        |ui| {
                            ui.label(egui::RichText::new("Ctrl+click to remove:").small());
                            for tag in leaves.iter().skip(max_visible) {
                                let name = tag.full_path.replace('>', " > ");
                                if ui.small_button(format!("✕ {}", name)).clicked() {
                                    ctrl_clicked_tag = Some(tag.id);
                                }
                            }
                        },
                    );
                } else {
                    let names: Vec<String> = leaves
                        .iter()
                        .skip(max_visible)
                        .map(|t| t.full_path.replace('>', " > "))
                        .collect();
                    egui::show_tooltip_text(
                        ui.ctx(),
                        ui.layer_id(),
                        ui.id().with(("thumb_tag_overflow_tip", item_id)),
                        &names.join("\n"),
                    );
                }
            }
        }
    }

    ctrl_clicked_tag
}

// ══════════════════════════════════════════════════════════════════
//  Hover-overlay helpers
// ══════════════════════════════════════════════════════════════════

/// Bottom info scrim for a hovered thumbnail: a vertical gradient whose
/// lowest few pixels are drawn as a bottom-rounded solid cap, so the scrim
/// respects the image's rounded corners instead of poking square corners
/// past them.
fn paint_hover_scrim(painter: &egui::Painter, rect: Rect, radius: f32, alpha: u8) {
    let cap_h = radius.min(rect.height() * 0.5);
    let a = alpha as f32;
    // Gradient alpha at the seam where the solid cap takes over.
    let a_seam = (a * (1.0 - cap_h / rect.height().max(1.0))) as u8;
    let grad_rect = Rect::from_min_max(
        rect.min,
        egui::pos2(rect.max.x, rect.max.y - cap_h),
    );
    paint_gradient_v(
        painter,
        grad_rect,
        Color32::from_black_alpha(0),
        Color32::from_black_alpha(a_seam),
    );
    let cap_rect = Rect::from_min_max(grad_rect.left_bottom(), rect.max);
    painter.add(egui::epaint::RectShape {
        rect: cap_rect,
        rounding: Rounding {
            nw: 0.0, ne: 0.0,
            sw: radius, se: radius,
        },
        fill: Color32::from_black_alpha(((a_seam as u16 + alpha as u16) / 2) as u8),
        stroke: egui::Stroke::NONE,
        blur_width: 0.0,
        fill_texture_id: egui::TextureId::default(),
        uv: Rect::ZERO,
    });
}

/// Paint a vertical-gradient quad. egui has no native gradient primitive,
/// so we build a 2-triangle Mesh with per-vertex colors. Used for the
/// hover overlay strip at the bottom of each thumbnail.
fn paint_gradient_v(painter: &egui::Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::Mesh(mesh));
}

/// Compact date string for the hover-overlay metadata line.
/// Format: "Mar 15, 2024" (no leading zero on day).
fn format_short_date(secs: i64) -> String {
    use chrono::{DateTime, Datelike, Local};
    let Some(utc) = DateTime::from_timestamp(secs, 0) else { return String::new(); };
    let dt: DateTime<Local> = utc.into();
    format!("{} {}, {}", dt.format("%b"), dt.day(), dt.year())
}
