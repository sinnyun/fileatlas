const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const normalizeWindowsPath = value => fs.realpathSync.native(value).toLowerCase();

module.exports = async ({ call, payload, delay, step, profile }) => {
  const invoke = async (name, args = {}) => payload(await call(name, args));
  const readJson = async url => {
    const response = await fetch(url);
    if (!response.ok) throw Error(`Eagle API HTTP ${response.status}: ${url}`);
    const result = await response.json();
    if (result.status !== 'success') throw Error(result.message ?? 'Eagle API request failed');
    return result.data;
  };
  const api = 'http://127.0.0.1:41595';
  const page = await readJson(`${api}/api/v2/item/get?limit=50`);
  const candidates = page.data.filter(item =>
    !item.isDeleted && !item.noThumbnail && /^(png|jpe?g|webp|gif|bmp|tiff?)$/i.test(item.ext));
  let original;
  for (const item of candidates) {
    const thumbnailResponse = await fetch(`${api}/api/item/thumbnail?id=${encodeURIComponent(item.id)}`);
    if (!thumbnailResponse.ok) continue;
    const thumbnailData = await thumbnailResponse.json();
    if (thumbnailData.status !== 'success') continue;
    const thumbnail = decodeURIComponent(thumbnailData.data);
    const directory = path.dirname(thumbnail);
    if (!fs.existsSync(directory)) continue;
    const matches = fs.readdirSync(directory)
      .map(name => path.join(directory, name))
      .filter(file => file !== thumbnail && fs.existsSync(file) && fs.statSync(file).isFile())
      .filter(file => path.extname(file).slice(1).toLowerCase() === item.ext.toLowerCase())
      .filter(file => fs.statSync(file).size === item.size);
    if (matches.length === 1) {
      original = { item, path: matches[0] };
      break;
    }
  }
  assert(original, 'Eagle must expose at least one selected-independent image with a uniquely matchable original');

  const board = await invoke('whiteboard.create', { title: 'Eagle 图片来源验证' });
  await delay(800);
  await invoke('whiteboard.add_images', { id: board.id, paths: [original.path], at: [520, 360] });
  let state;
  for (let i = 0; i < 40; i++) {
    state = await invoke('whiteboard.state');
    const record = state.library.boards.find(item => item.id === board.id);
    const scene = record ? JSON.parse(record.scene) : null;
    if (scene?.elements.some(element => element.kind?.image)) break;
    await delay(150);
  }
  const record = state.library.boards.find(item => item.id === board.id);
  assert(record, 'created board should exist');
  const scene = JSON.parse(record.scene);
  const image = scene.elements.find(element => element.kind?.image);
  const source = record.eagle_sources?.find(item => item.item_id === original.item.id);
  if (!image) await step({ tool: 'screenshot_window', capture: 'eagle-drop-failed' });
  assert(image, `Eagle image should be placed through the canvas image placement path; board=${JSON.stringify(record)}`);
  assert(source, 'board should persist Eagle source metadata');
  assert.equal(source.name, original.item.name);
  assert.equal(source.original_uri, image.kind.image.src);
  assert.equal(normalizeWindowsPath(source.original_path), normalizeWindowsPath(original.path));
  assert.match(source.api_url, /api\/v2\/item\/get\?id=/);
  assert.equal(source.folders.length, original.item.folders?.length ?? 0);
  const preview = record.image_previews?.find(item => item.uri === image.kind.image.src);
  assert(preview, 'board should persist a thumbnail reference for the dropped image');
  assert.equal(preview.file_name, path.basename(preview.file_name));
  const previewPath = path.join(profile, 'whiteboards', 'previews', String(board.id), preview.file_name);
  assert(fs.existsSync(previewPath), `board-owned thumbnail should exist: ${previewPath}`);
  const liveUi = await invoke('get_ui_state');
  const screenScale = liveUi.window_status.geometry.scale_factor;
  await invoke('whiteboard.flush');
  await step({ tool: 'screenshot_window', capture: 'eagle-image-properties' });

  const readCamera = async () => {
    const live = await invoke('whiteboard.state');
    const active = live.library.boards.find(item => item.id === board.id);
    return active && JSON.parse(active.scene).camera;
  };
  const readCache = () => invoke('whiteboard.cache_stats');
  const waitForCache = async predicate => {
    let cache;
    for (let i = 0; i < 40; i++) {
      cache = await readCache();
      if (predicate(cache.cache)) return cache.cache;
      await delay(150);
    }
    throw Error(`image cache did not reach expected state: ${JSON.stringify(cache?.cache)}`);
  };
  const thumbnailCache = await waitForCache(cache => cache.thumbnail_decoded_images > 0);
  assert.equal(thumbnailCache.original_decoded_images, 0,
    'canvas should initially decode the persisted thumbnail without the original');
  await invoke('whiteboard.scroll', {
    id: board.id, delta_x: 0, delta_y: 400, control: false, at: [500, 400],
  });
  await delay(200);
  await invoke('whiteboard.scroll', {
    id: board.id, delta_x: 0, delta_y: 160, control: false, at: [500, 400],
  });
  let highResolutionCamera;
  for (let i = 0; i < 30; i++) {
    await delay(100);
    highResolutionCamera = await readCamera();
    if (highResolutionCamera?.zoom > 2.7) break;
  }
  assert(highResolutionCamera?.zoom > 2.7, 'canvas should zoom far enough to show a high physical pixel area');
  const imageWidth = image.kind.image.w;
  const imageHeight = image.kind.image.h;
  const highVisibleArea = imageWidth * highResolutionCamera.zoom * screenScale
    * imageHeight * highResolutionCamera.zoom * screenScale;
  assert(highVisibleArea >= 500_000, `high zoom should cross the physical-pixel threshold: ${highVisibleArea}`);
  await delay(500);
  const originalCache = await waitForCache(cache => cache.original_decoded_images > 0);
  assert(originalCache.original_decoded_bytes > 0, 'decoded original memory should be reported');
  assert(originalCache.original_decode_requests > 0, 'original decode requests should be counted');
  await step({ tool: 'screenshot_window', capture: 'canvas-high-visible-pixel-area' });
  await invoke('whiteboard.scroll', {
    id: board.id, delta_x: -2500, delta_y: 0, control: true, at: [500, 400],
  });
  await delay(500);
  const pannedOutCamera = await readCamera();
  assert.equal(pannedOutCamera.zoom, highResolutionCamera.zoom,
    'panning out of view should not alter image display scale');
  await invoke('whiteboard.scroll', {
    id: board.id, delta_x: 2500, delta_y: 0, control: true, at: [500, 400],
  });
  await delay(250);
  const pannedBackCamera = await readCamera();
  assert(Math.abs(pannedBackCamera.x - highResolutionCamera.x) < 0.01,
    'pan out and back should restore the same camera position');
  const panReturnCache = await waitForCache(cache => cache.original_decoded_images > 0);
  assert(panReturnCache.original_memory_hits > originalCache.original_memory_hits,
    'returning to the image should reuse the decoded original');
  await step({ tool: 'screenshot_window', capture: 'canvas-original-after-pan-return' });
  await invoke('whiteboard.scroll', {
    id: board.id, delta_x: 0, delta_y: -186, control: false, at: [500, 400],
  });
  let thumbnailCamera;
  for (let i = 0; i < 30; i++) {
    await delay(100);
    thumbnailCamera = await readCamera();
    if (thumbnailCamera?.zoom < 1.6) break;
  }
  assert(thumbnailCamera?.zoom < 1.6, 'canvas should zoom back below the thumbnail screen-area threshold');
  const thumbnailVisibleArea = imageWidth * thumbnailCamera.zoom * screenScale
    * imageHeight * thumbnailCamera.zoom * screenScale;
  assert(thumbnailVisibleArea <= 350_000, `lower zoom should return below the thumbnail threshold: ${thumbnailVisibleArea}`);
  await delay(350);
  const downgradedCache = await waitForCache(cache => cache.thumbnail_quality_images > 0);
  assert(downgradedCache.original_decoded_images > 0,
    'thumbnail downgrade should retain the original in the warm memory cache');
  await step({ tool: 'screenshot_window', capture: 'canvas-thumbnail-screen-area' });

  await invoke('whiteboard.scroll', {
    id: board.id, delta_x: -1800, delta_y: 0, control: true, at: [500, 400],
  });
  const evictionBaseline = await readCache();
  assert.equal(evictionBaseline.cache.original_cache_ttl_ms, 30_000);
  assert.equal(evictionBaseline.cache.idle_cache_budget_bytes, 128 * 1024 * 1024);
  let evictedCache = evictionBaseline;
  for (let i = 0; i < 5 && evictedCache.cache.original_ttl_evictions === 0; i++) {
    await delay(8_000);
    evictedCache = await readCache();
  }
  assert(evictedCache.cache.original_ttl_evictions > 0,
    `idle original should be evicted after TTL; age=${evictedCache.cache.oldest_idle_original_age_ms} ms`);
  assert.equal(evictedCache.cache.original_decoded_images, 0,
    'idle original should be released after its retention timeout');

  const otherBoard = await invoke('whiteboard.create', { title: '缩略图切换验证' });
  await delay(350);
  await invoke('whiteboard.open', { id: board.id });
  await delay(500);
  const reopened = await invoke('whiteboard.state');
  assert.equal(reopened.library.active_id, board.id);
  assert(fs.existsSync(previewPath), 'thumbnail should still be present after switching away and back');
  await step({ tool: 'screenshot_window', capture: 'canvas-board-switched-back' });
  const result = {
    passed: true,
    board_id: board.id,
    eagle_item_id: source.item_id,
    source_path: source.original_path,
    original_uri: source.original_uri,
    eagle_api_url: source.api_url,
    folder_count: source.folders.length,
    scene_image_src: image.kind.image.src,
    preview: { path: previewPath, width: preview.width, height: preview.height },
    quality_zoom: {
      scale_factor: screenScale,
      high_resolution: highResolutionCamera.zoom,
      high_visible_pixel_area: Math.round(highVisibleArea),
      panned_out_and_back: true,
      original_cache_hits_after_pan: panReturnCache.original_memory_hits,
      thumbnail: thumbnailCamera.zoom,
      thumbnail_visible_pixel_area: Math.round(thumbnailVisibleArea),
      original_ttl_evictions: evictedCache.cache.original_ttl_evictions,
      original_decoded_after_ttl: evictedCache.cache.original_decoded_images,
    },
    cache_stats: {
      thumbnail_initial: thumbnailCache,
      original_loaded: originalCache,
      reused_after_pan: panReturnCache,
      retained_after_thumbnail_downgrade: downgradedCache,
      after_ttl_eviction: evictedCache.cache,
    },
    switched_via_board_id: otherBoard.id,
    profile,
    cases: ['Eagle item resolved against live local API', 'original file linked into canvas scene', 'thumbnail decoded before original', 'cache MCP reports decoded memory and policy', 'full image screen area selects quality independently of viewport clipping', 'pan out and back reuses decoded original', 'thumbnail downgrade retains warm original', 'offscreen original expires and is evicted after TTL', 'board switch restores persisted thumbnail preview', 'item name, path, URI, API URL and folders persisted on board'],
  };
  fs.writeFileSync(path.join(profile, 'eagle-canvas-drop-result.json'), JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result));
};
