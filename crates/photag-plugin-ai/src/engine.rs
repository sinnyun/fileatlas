//! Lightweight local AI tag suggestions using MobileNetV2 (ONNX).
//!
//! On first use the ~14 MB model is downloaded from the ONNX Model Zoo
//! into the PhoTag data directory. Inference runs in a background thread
//! so the UI stays responsive; results arrive through an `mpsc` channel.
//!
//! Adapted from PhoTag commit 36cee242a78c0a8cd9ac6b0d63b64265aa0add90.
//! Upstream license: `vendor/photag-source/LICENSE`.

use std::path::PathBuf;
use std::sync::{mpsc, Mutex};

use super::TagSuggestion;

static MODEL_DOWNLOAD_LOCK: Mutex<()> = Mutex::new(());

/// Where the model file lives on disk.
fn model_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("PhoTag").join("models")
}

fn model_path() -> PathBuf {
    model_dir().join("mobilenetv2-7.onnx")
}

pub fn model_exists() -> bool {
    model_path().is_file()
}

/// Download MobileNetV2 ONNX (~14 MB) from the ONNX Model Zoo.
/// Returns Ok on success or an error message.
pub fn download_model(progress_tx: Option<mpsc::Sender<String>>) -> Result<(), String> {
    let _guard = MODEL_DOWNLOAD_LOCK
        .lock()
        .map_err(|_| "model download lock was poisoned".to_string())?;
    let dir = model_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("create dir: {}", e))?;

    let url = "https://github.com/onnx/models/raw/main/validated/vision/classification/mobilenet/model/mobilenetv2-7.onnx";
    if let Some(ref tx) = progress_tx {
        tx.send("Downloading AI model (~14 MB)...".into()).ok();
    }

    let resp = ureq::get(url)
        .call()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let mut reader = resp.into_reader();
    let tmp = dir.join("mobilenetv2-7.onnx.tmp");
    let mut file = std::fs::File::create(&tmp).map_err(|e| format!("create file: {}", e))?;
    if let Err(error) = std::io::copy(&mut reader, &mut file) {
        drop(file);
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("download: {}", error));
    }
    if let Err(error) = file.sync_all() {
        drop(file);
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("flush model: {}", error));
    }
    drop(file);

    let destination = model_path();
    // Windows does not replace an existing file with `rename`. Keep an
    // existing model recoverable until the new download has been published.
    if destination.exists() {
        let backup = dir.join("mobilenetv2-7.onnx.previous");
        if backup.exists() {
            std::fs::remove_file(&backup).map_err(|e| format!("remove old backup: {}", e))?;
        }
        std::fs::rename(&destination, &backup).map_err(|e| format!("move existing model: {}", e))?;
        if let Err(error) = std::fs::rename(&tmp, &destination) {
            let _ = std::fs::rename(&backup, &destination);
            let _ = std::fs::remove_file(&tmp);
            return Err(format!("publish model: {}", error));
        }
        let _ = std::fs::remove_file(&backup);
    } else if let Err(error) = std::fs::rename(&tmp, &destination) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("publish model: {}", error));
    }

    if let Some(ref tx) = progress_tx {
        tx.send("Model downloaded successfully!".into()).ok();
    }
    Ok(())
}

// ── Image analysis helpers ─────────────────────────────────────────────

/// Multi-signal image analysis used for mandatory tag groups and
/// to augment the neural network output.
struct ImageAnalysis {
    /// Brightness of the center 50 % crop (0–255).
    center_brightness: f32,
    /// 75th-percentile brightness across the whole image (0–255).
    p75_brightness: f32,
    /// Average brightness (0–255).
    avg_brightness: f32,
    /// Warm-vs-cool ratio: >1 means warm tint, <1 means cool.
    warmth: f32,
    /// Average colour saturation (0–1).
    saturation: f32,
}

fn analyze_image(image_path: &str) -> Option<ImageAnalysis> {
    let img = image::open(image_path).ok()?;
    let small = img
        .resize_exact(128, 128, image::imageops::FilterType::Triangle)
        .to_rgb8();
    let (w, h) = (128u32, 128u32);

    let mut all_brightness: Vec<f32> = Vec::with_capacity((w * h) as usize);
    let mut center_sum: f64 = 0.0;
    let mut center_count: u32 = 0;
    let mut r_sum: f64 = 0.0;
    let mut b_sum: f64 = 0.0;
    let mut sat_sum: f64 = 0.0;
    let total = (w * h) as f64;

    // Center crop bounds (middle 50 %)
    let cx0 = w / 4;
    let cx1 = w * 3 / 4;
    let cy0 = h / 4;
    let cy1 = h * 3 / 4;

    for y in 0..h {
        for x in 0..w {
            let p = small.get_pixel(x, y);
            let (r, g, b) = (p[0] as f64, p[1] as f64, p[2] as f64);
            let lum = (r + g + b) / 3.0;
            all_brightness.push(lum as f32);

            if x >= cx0 && x < cx1 && y >= cy0 && y < cy1 {
                center_sum += lum;
                center_count += 1;
            }

            r_sum += r;
            b_sum += b;

            // Simple saturation: (max - min) / max
            let max_c = r.max(g).max(b);
            let min_c = r.min(g).min(b);
            if max_c > 0.0 {
                sat_sum += (max_c - min_c) / max_c;
            }
        }
    }

    // Sort for percentile
    all_brightness.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p75_idx = (all_brightness.len() as f64 * 0.75) as usize;
    let p75 = all_brightness.get(p75_idx).copied().unwrap_or(128.0);
    let avg = all_brightness.iter().sum::<f32>() / all_brightness.len().max(1) as f32;
    let center_avg = if center_count > 0 {
        (center_sum / center_count as f64) as f32
    } else {
        avg
    };

    // Warmth: red-channel / blue-channel ratio
    let warmth = if b_sum > 0.0 {
        (r_sum / b_sum) as f32
    } else {
        1.0
    };
    let saturation = (sat_sum / total) as f32;

    Some(ImageAnalysis {
        center_brightness: center_avg,
        p75_brightness: p75,
        avg_brightness: avg,
        warmth,
        saturation,
    })
}

/// Run MobileNetV2 inference on the image at `image_path`.
/// Returns the top-N tag suggestions sorted by confidence (descending),
/// plus the full 1000-class probability vector for mandatory group use.
fn run_inference(image_path: &str, top_n: usize) -> Result<(Vec<TagSuggestion>, Vec<f32>), String> {
    use tract_onnx::prelude::*;

    let model_file = model_path();
    if !model_file.is_file() {
        return Err("Model not downloaded yet".into());
    }

    // Load and optimize the model
    let model = tract_onnx::onnx()
        .model_for_path(&model_file)
        .map_err(|e| format!("load model: {}", e))?
        .with_input_fact(0, f32::fact([1, 3, 224, 224]).into())
        .map_err(|e| format!("set input: {}", e))?
        .into_optimized()
        .map_err(|e| format!("optimize: {}", e))?
        .into_runnable()
        .map_err(|e| format!("make runnable: {}", e))?;

    // Load image — centre-crop to square before resizing for better
    // framing (stretch distorts aspect and confuses the model).
    let raw = image::open(image_path).map_err(|e| format!("open image: {}", e))?;
    let (iw, ih) = (raw.width(), raw.height());
    let side = iw.min(ih);
    let cx = (iw - side) / 2;
    let cy = (ih - side) / 2;
    let cropped = raw.crop_imm(cx, cy, side, side);
    let img = cropped
        .resize_exact(224, 224, image::imageops::FilterType::Triangle)
        .to_rgb8();

    // ImageNet normalization constants
    let mean = [0.485f32, 0.456, 0.406];
    let std_dev = [0.229f32, 0.224, 0.225];

    let input = ndarray::Array4::from_shape_fn((1, 3, 224, 224), |(_, c, y, x)| {
        let pixel = img.get_pixel(x as u32, y as u32);
        (pixel[c] as f32 / 255.0 - mean[c]) / std_dev[c]
    });

    let result = model
        .run(tvec!(input.into_tvalue()))
        .map_err(|e| format!("inference: {}", e))?;

    let output = result[0]
        .to_array_view::<f32>()
        .map_err(|e| format!("output: {}", e))?;

    // Softmax
    let logits: Vec<f32> = output.iter().copied().collect();
    let max_logit = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = logits.iter().map(|&x| (x - max_logit).exp()).collect();
    let sum: f32 = exps.iter().sum();
    let probs: Vec<f32> = exps.iter().map(|&x| x / sum).collect();

    // Get top indices sorted by probability
    let mut indexed: Vec<(usize, f32)> = probs.iter().copied().enumerate().collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Map ImageNet classes to user-friendly tag names, dedup
    let mut seen = std::collections::HashSet::new();
    let mut suggestions = Vec::new();
    for (idx, conf) in indexed {
        if conf < 0.005 {
            break;
        } // lower threshold to catch more
        if let Some(tag) = super::imagenet::imagenet_to_tag(idx) {
            if seen.insert(tag.to_string()) {
                suggestions.push(TagSuggestion {
                    label: tag.to_string(),
                    confidence: conf,
                    group_name: None,
                });
                if suggestions.len() >= top_n {
                    break;
                }
            }
        }
    }
    Ok((suggestions, probs))
}

// ── Mandatory tag group analysis ───────────────────────────────────────

/// Concept indices: groups of ImageNet classes whose summed probability
/// acts as a signal for abstract concepts like "outdoor" or "people".
fn concept_score(probs: &[f32], concept: &str) -> f32 {
    let indices: &[usize] = match concept {
        // Outdoor / nature / landscape classes
        "outdoor" | "day" | "outside" => &[
            970, 971, 972, 973, 974, 975, 976, 977, 978, 979, 980, 981, 982, 983, 984, 985, 986,
            987, 988, 989, // outdoor objects
            404, 444, 472, 510, 554, 628, 724, 780, 814, 871, 914,
            // animals often photographed outdoors
            7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 339, 340, 341, 350, 351, 352, 353,
        ],
        // Indoor objects / scenes
        "indoor" | "inside" => &[
            508, 527, 532, 559, 579, 620, 664, 681, 737, 765, 831, 832, 487, 508, 509, 530, 621,
            639, 643, 689, 703, 831,
        ],
        // People-related classes
        "people" | "person" | "portrait" | "selfie" | "face" => &[
            445, 452, 459, 474, 504, 515, 530, 540, 601, 610, 614, 617, 629, 638, 639, 643, 655,
            689, 710, 834, 837, 840, 846, 858, 869, 900, 906, // glasses, watch, necklace
            837, 531, 635,
        ],
        // Vehicle classes
        "vehicle" | "car" | "transport" => &[
            404, 407, 408, 436, 444, 466, 468, 511, 547, 555, 569, 609, 627, 654, 656, 671, 705,
            717, 734, 751, 814, 817, 820, 829, 864, 895,
        ],
        // Food
        "food" | "meal" | "cooking" => &[
            924, 925, 926, 927, 928, 929, 930, 931, 932, 933, 934, 935, 936, 937, 938, 939, 940,
            941, 942, 943, 944, 945, 946, 947, 948, 949, 950, 951, 952, 953, 954, 955, 956, 957,
            958, 959, 960, 961, 962, 963, 964, 965, 966, 967,
        ],
        // Animal
        "animal" | "pet" | "wildlife" => &[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 200,
            210, 220, 230, 240, 250, 260, 270, 280, 281, 282, 283, 284, 285, 291, 292, 293, 294,
            295, 330, 331, 332, 339, 340, 341, 350, 351, 352, 353,
        ],
        _ => return 0.0,
    };
    indices
        .iter()
        .filter(|&&i| i < probs.len())
        .map(|&i| probs[i])
        .sum()
}

/// Score a single user tag against both image properties and the NN
/// output. Higher score = better match for this tag.
fn score_tag(tag: &str, analysis: &ImageAnalysis, probs: &[f32]) -> f32 {
    let lower = tag.to_lowercase();
    let mut score: f32 = 0.0;

    // ── Brightness / lighting keywords ──────────────────────────────
    // Use centre brightness and 75th percentile — robust against dark
    // backgrounds around a well-lit subject.
    let bright_signal = analysis.center_brightness.max(analysis.p75_brightness);
    let is_bright = bright_signal > 115.0; // well-lit threshold
    let is_very_dark = analysis.p75_brightness < 60.0;

    if lower.contains("day") || lower.contains("bright") || lower.contains("daylight") {
        score += if is_bright { 1.0 } else { 0.1 };
    }
    if lower.contains("night") || lower.contains("dark") {
        score += if is_very_dark {
            1.0
        } else if !is_bright {
            0.5
        } else {
            0.05
        };
    }
    if lower.contains("artificial") || lower.contains("indoor light") {
        // Warm tint + moderate brightness → artificial light
        score += if analysis.warmth > 1.15 && !is_very_dark {
            0.8
        } else {
            0.2
        };
    }
    if lower.contains("natural") || lower.contains("sunlight") {
        score += if analysis.warmth < 1.15 && is_bright {
            0.8
        } else {
            0.2
        };
    }
    if lower.contains("golden") || lower.contains("warm") {
        score += if analysis.warmth > 1.2 && is_bright {
            0.9
        } else {
            0.15
        };
    }

    // ── Indoor / outdoor ────────────────────────────────────────────
    if lower.contains("outdoor") || lower.contains("outside") {
        score += concept_score(probs, "outdoor");
    }
    if lower.contains("indoor") || lower.contains("inside") {
        score += concept_score(probs, "indoor");
    }

    // ── Subject-type keywords ───────────────────────────────────────
    for concept in ["people", "animal", "food", "vehicle"] {
        if lower.contains(concept)
            || (concept == "people" && (lower.contains("person") || lower.contains("portrait")))
        {
            score += concept_score(probs, concept);
        }
    }

    // ── Colour keywords ─────────────────────────────────────────────
    if lower.contains("colorful") || lower.contains("colourful") || lower.contains("vibrant") {
        score += if analysis.saturation > 0.4 { 0.8 } else { 0.1 };
    }
    if lower.contains("muted") || lower.contains("monochrome") || lower.contains("b&w") {
        score += if analysis.saturation < 0.15 { 0.8 } else { 0.1 };
    }

    score
}

/// Analyze an image against mandatory tag groups using both pixel-level
/// analysis and the neural network class probabilities.
pub fn analyze_mandatory_groups(
    image_path: &str,
    groups: &[photag_plugin_settings::AiTagGroup],
    probs: &[f32],
) -> Vec<TagSuggestion> {
    if groups.is_empty() {
        return Vec::new();
    }

    let analysis = analyze_image(image_path).unwrap_or(ImageAnalysis {
        center_brightness: 128.0,
        p75_brightness: 128.0,
        avg_brightness: 128.0,
        warmth: 1.0,
        saturation: 0.3,
    });

    let mut results = Vec::new();
    for group in groups {
        if group.tags.is_empty() {
            continue;
        }

        // Score every tag in the group; pick the highest-scoring one.
        let mut best_tag = &group.tags[0];
        let mut best_score: f32 = -1.0;
        for tag in &group.tags {
            let s = score_tag(tag, &analysis, probs);
            if s > best_score {
                best_score = s;
                best_tag = tag;
            }
        }

        results.push(TagSuggestion {
            label: best_tag.clone(),
            confidence: best_score.clamp(0.0, 1.0),
            group_name: Some(group.name.clone()),
        });
    }
    results
}

/// Spawn inference on a background thread. Returns a receiver for the result.
pub fn suggest_tags_async(
    image_path: String,
    top_n: usize,
    mandatory_groups: Vec<photag_plugin_settings::AiTagGroup>,
) -> mpsc::Receiver<Result<Vec<TagSuggestion>, String>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        // Run neural network first — we need the probabilities for
        // mandatory group scoring too.
        let result = run_inference(&image_path, top_n);
        match result {
            Ok((mut suggestions, probs)) => {
                // Analyze mandatory groups using both image + NN output
                let mandatory = analyze_mandatory_groups(&image_path, &mandatory_groups, &probs);
                let mut combined = mandatory;
                combined.append(&mut suggestions);
                tx.send(Ok(combined)).ok();
            }
            Err(e) => {
                tx.send(Err(e)).ok();
            }
        }
    });
    rx
}

/// Spawn model download on a background thread. Returns a receiver for progress/result.
pub fn download_model_async() -> mpsc::Receiver<Result<(), String>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = download_model(None);
        tx.send(result).ok();
    });
    rx
}
