//! Lightweight local AI tag suggestions using MobileNetV2 (ONNX).
//!
//! On first use the ~14 MB model is downloaded from the ONNX Model Zoo
//! into the PhoTag data directory. Inference runs in a background thread
//! so the UI stays responsive; results arrive through an `mpsc` channel.

use std::path::PathBuf;
use std::sync::mpsc;

/// A single tag suggestion with its confidence score (0.0 – 1.0).
#[derive(Debug, Clone)]
pub struct TagSuggestion {
    pub label: String,
    pub confidence: f32,
    /// If set, this suggestion came from a mandatory group.
    pub group_name: Option<String>,
}

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
    std::io::copy(&mut reader, &mut file).map_err(|e| format!("download: {}", e))?;
    std::fs::rename(&tmp, model_path()).map_err(|e| format!("rename: {}", e))?;

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
    let small = img.resize_exact(128, 128, image::imageops::FilterType::Triangle).to_rgb8();
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
    let center_avg = if center_count > 0 { (center_sum / center_count as f64) as f32 } else { avg };

    // Warmth: red-channel / blue-channel ratio
    let warmth = if b_sum > 0.0 { (r_sum / b_sum) as f32 } else { 1.0 };
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
    let raw = image::open(image_path)
        .map_err(|e| format!("open image: {}", e))?;
    let (iw, ih) = (raw.width(), raw.height());
    let side = iw.min(ih);
    let cx = (iw - side) / 2;
    let cy = (ih - side) / 2;
    let cropped = raw.crop_imm(cx, cy, side, side);
    let img = cropped.resize_exact(224, 224, image::imageops::FilterType::Triangle).to_rgb8();

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
        if conf < 0.005 { break; } // lower threshold to catch more
        if let Some(tag) = imagenet_to_tag(idx) {
            if seen.insert(tag.to_string()) {
                suggestions.push(TagSuggestion {
                    label: tag.to_string(),
                    confidence: conf,
                    group_name: None,
                });
                if suggestions.len() >= top_n { break; }
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
            970, 971, 972, 973, 974, 975, 976, 977, 978, 979, 980,
            981, 982, 983, 984, 985, 986, 987, 988, 989,
            // outdoor objects
            404, 444, 472, 510, 554, 628, 724, 780, 814, 871, 914,
            // animals often photographed outdoors
            7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
            339, 340, 341, 350, 351, 352, 353,
        ],
        // Indoor objects / scenes
        "indoor" | "inside" => &[
            508, 527, 532, 559, 579, 620, 664, 681, 737, 765, 831, 832,
            487, 508, 509, 530, 621, 639, 643, 689, 703, 831,
        ],
        // People-related classes
        "people" | "person" | "portrait" | "selfie" | "face" => &[
            445, 452, 459, 474, 504, 515, 530, 540, 601, 610, 614,
            617, 629, 638, 639, 643, 655, 689, 710, 834, 837, 840,
            846, 858, 869, 900, 906,
            // glasses, watch, necklace
            837, 531, 635,
        ],
        // Vehicle classes
        "vehicle" | "car" | "transport" => &[
            404, 407, 408, 436, 444, 466, 468, 511, 547, 555, 569,
            609, 627, 654, 656, 671, 705, 717, 734, 751, 814, 817,
            820, 829, 864, 895,
        ],
        // Food
        "food" | "meal" | "cooking" => &[
            924, 925, 926, 927, 928, 929, 930, 931, 932, 933, 934,
            935, 936, 937, 938, 939, 940, 941, 942, 943, 944, 945,
            946, 947, 948, 949, 950, 951, 952, 953, 954, 955, 956,
            957, 958, 959, 960, 961, 962, 963, 964, 965, 966, 967,
        ],
        // Animal
        "animal" | "pet" | "wildlife" => &[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            151, 152, 153, 154, 155, 156, 157, 158, 159, 160,
            200, 210, 220, 230, 240, 250, 260, 270, 280, 281, 282,
            283, 284, 285, 291, 292, 293, 294, 295,
            330, 331, 332, 339, 340, 341, 350, 351, 352, 353,
        ],
        _ => return 0.0,
    };
    indices.iter()
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
    let is_bright = bright_signal > 115.0;  // well-lit threshold
    let is_very_dark = analysis.p75_brightness < 60.0;

    if lower.contains("day") || lower.contains("bright") || lower.contains("daylight") {
        score += if is_bright { 1.0 } else { 0.1 };
    }
    if lower.contains("night") || lower.contains("dark") {
        score += if is_very_dark { 1.0 } else if !is_bright { 0.5 } else { 0.05 };
    }
    if lower.contains("artificial") || lower.contains("indoor light") {
        // Warm tint + moderate brightness → artificial light
        score += if analysis.warmth > 1.15 && !is_very_dark { 0.8 } else { 0.2 };
    }
    if lower.contains("natural") || lower.contains("sunlight") {
        score += if analysis.warmth < 1.15 && is_bright { 0.8 } else { 0.2 };
    }
    if lower.contains("golden") || lower.contains("warm") {
        score += if analysis.warmth > 1.2 && is_bright { 0.9 } else { 0.15 };
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
        if lower.contains(concept) || (concept == "people" && (lower.contains("person") || lower.contains("portrait"))) {
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
    groups: &[crate::settings::AiTagGroup],
    probs: &[f32],
) -> Vec<TagSuggestion> {
    if groups.is_empty() { return Vec::new(); }

    let analysis = analyze_image(image_path).unwrap_or(ImageAnalysis {
        center_brightness: 128.0,
        p75_brightness: 128.0,
        avg_brightness: 128.0,
        warmth: 1.0,
        saturation: 0.3,
    });

    let mut results = Vec::new();
    for group in groups {
        if group.tags.is_empty() { continue; }

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
    mandatory_groups: Vec<crate::settings::AiTagGroup>,
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
            Err(e) => { tx.send(Err(e)).ok(); }
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

// ── ImageNet class index → user-friendly tag mapping ───────────────────
// Curated mapping of ImageNet 1000 classes → useful photo tags.
// Returns None for classes without meaningful tags.

fn imagenet_to_tag(idx: usize) -> Option<&'static str> {
    Some(match idx {
        // ── Fish / aquatic ──────────────────────────────────────────
        0..=6 => "fish",
        // ── Birds ───────────────────────────────────────────────────
        7..=24 => "bird",
        // ── Amphibians / small creatures ────────────────────────────
        25..=32 => "frog",
        33..=68 => "sea creature",
        69..=70 => "crustacean",
        71..=76 => "spider",
        77..=99 => "insect",
        // ── Reptiles ────────────────────────────────────────────────
        100..=118 => "snake",
        119..=131 => "lizard",
        132..=142 => "dinosaur",
        143..=145 => "turtle",
        146..=150 => "lizard",
        // ── Dogs ────────────────────────────────────────────────────
        151..=268 => "dog",
        // ── Canids / wild ───────────────────────────────────────────
        269..=271 => "wolf",
        272..=274 => "fox",
        275 => "coyote",
        276..=280 => "animal",
        // ── Cats (domestic + big) ───────────────────────────────────
        281..=285 => "cat",
        286 => "cougar",
        287 => "lynx",
        288 => "leopard",
        289 => "snow leopard",
        290 => "jaguar",
        291 => "lion",
        292 => "tiger",
        293 => "cheetah",
        // ── Bears ───────────────────────────────────────────────────
        294..=297 => "bear",
        // ── Weasels / small mammals ─────────────────────────────────
        298..=299 => "animal",
        // ── Insects (continued) ─────────────────────────────────────
        300..=319 => "insect",
        320..=321 => "butterfly",
        322..=329 => "animal",
        // ── Rodents / rabbits ───────────────────────────────────────
        330..=332 => "rabbit",
        333..=338 => "animal",
        // ── Larger mammals ──────────────────────────────────────────
        339..=341 => "horse",
        342..=345 => "horse",
        346..=349 => "animal",
        350..=351 => "sheep",
        352..=353 => "cow",
        354..=355 => "pig",
        356..=364 => "animal",
        // ── Primates ────────────────────────────────────────────────
        365..=382 => "monkey",
        383..=384 => "animal",
        385..=386 => "elephant",
        387 => "animal",
        388 => "panda",
        389..=397 => "animal",
        // ── Music ───────────────────────────────────────────────────
        401..=402 => "music",
        404 => "airplane",
        407 => "car",           // ambulance
        408 => "truck",
        436 => "car",           // beach wagon
        440 => "drink",         // beer bottle
        444 => "bicycle",
        466 => "train",
        468 => "car",           // cab
        472 => "boat",          // canoe
        481 => "camera",
        483 => "castle",
        484 => "boat",          // catamaran
        486 => "music",         // cello
        487 => "phone",         // cell phone
        497 => "church",
        508 => "computer",      // keyboard
        510 => "ship",          // container ship
        511 => "car",           // convertible
        513 => "music",         // cornet
        527 => "computer",      // desktop
        532 => "furniture",     // dining table
        536 => "architecture",  // dome
        541..=542 => "music",   // drum
        546 => "music",         // electric guitar
        547 => "train",         // electric locomotive
        554 => "boat",          // fireboat
        555 => "truck",         // fire engine
        559 => "furniture",     // folding chair
        566 => "music",         // flute
        569 => "truck",         // fire truck
        574 => "sport",         // golf ball
        579 => "music",         // grand piano
        593 => "music",         // guitar
        609 => "car",           // jeep
        620 => "laptop",
        627 => "car",           // limousine
        628 => "boat",          // lifeboat
        642 => "music",         // marimba
        654 => "truck",         // moving van
        656 => "car",           // minivan
        663 => "architecture",  // mosque
        664 => "monitor",
        668 => "architecture",  // monastery
        671 => "motorcycle",
        681 => "computer",      // notebook
        687 => "music",         // organ
        698 => "architecture",  // palace
        705 => "car",           // passenger car
        717 => "car",           // pickup truck
        722 => "sport",         // ping-pong ball
        724 => "boat",          // pirate ship
        734 => "car",           // police van
        737 => "sport",         // pool table
        747 => "sport",         // punching bag
        751 => "car",           // racer
        765 => "furniture",     // rocking chair
        768 => "sport",         // running shoe
        776 => "music",         // sax
        780 => "boat",          // schooner
        795 => "skiing",        // ski
        796 => "sport",         // ski mask
        801 => "soccer",        // soccer ball
        814 => "boat",          // speedboat
        817 => "car",           // sports car
        820 => "train",         // steam locomotive
        829 => "train",         // streetcar
        850 => "toy",           // teddy bear
        852 => "tennis",        // tennis ball
        864 => "truck",         // tow truck
        871 => "boat",          // trimaran
        875 => "music",         // trumpet
        889 => "music",         // violin
        895 => "airplane",      // warplane
        914 => "boat",          // yawl
        // ── Person-related (clothing, accessories, body parts) ──────
        // Massive block: any ImageNet class where a person is typically
        // visible in the image — clothing, headwear, eyewear, grooming.
        445 | 452 | 459 | 474 | 504 | 515 | 530 | 540 => "people",
        601 | 610 | 614 | 617 | 629 | 638 | 639 | 643 => "people",
        655 | 689 | 710 | 834 | 837 | 840 | 846 | 858 => "people",
        869 | 900 | 906 => "people",
        // Glasses / eyewear — critical for selfie detection
        831..=832 => "people",  // studio couch (often with person)
        // Additional person-adjacent items in ImageNet:
        // 398: abaya, 399: academic gown, 403: altar boy (vestment)
        398..=400 => "people",
        // 414: apron, 425: backpack → typically worn by person
        414 => "people",
        425 => "people",
        // 431: baseball, 432: basketball → sport with people
        // 440: beer bottle → drink context, skip
        // 451: bib → baby
        451 => "people",
        // 456: bow tie
        456 => "people",
        // 469: cape, 479: cardigan (covered above 474)
        469 => "people",
        // 491: crash helmet, 497: church skip
        491 => "people",
        // 499: cloak
        499 => "people",
        // 509: confectionery → skip, 517: crutch → person
        517 => "people",
        // 518: cuirass (armor)
        518 => "people",
        // 529: dumbbell → person exercising
        529 => "people",
        // 531: ear → someone's ear visible (this is NOT glasses)
        // 548: embrace → hugging people
        // 552: face powder (cosmetics on face)
        552 => "people",
        // 589: fur coat
        589 => "people",
        // 608: jean / jeans → person wearing jeans
        608 => "people",
        // 618: lederhosen
        618 => "people",
        // 623: letter opener → skip, 626: lighter → skip
        // 650: military uniform
        650 => "people",
        // 696: pajama / pyjama
        696 => "people",
        // 725: pill bottle → skip, 753: racket → sport
        753 => "sport",
        // 770: running shoe → already in sport area
        // 783: scabbard (sword holder, worn)
        783 => "people",
        // 802: sombrero
        802 => "people",
        // 808: stethoscope → doctor / person
        808 => "people",
        // 824: stole (fur wrap)
        824 => "people",
        // 835: sunglasses (already in 846, but also here)
        // 842: sweatshirt
        842 => "people",
        // 856: swimming trunks → already in 840
        // 859: syringe → skip
        // 862: tank top / jersey
        // 866: toga
        866 => "people",
        // 888: veil
        888 => "people",
        // 910: wig → already in 900
        // ── Food ────────────────────────────────────────────────────
        924..=969 => "food",
        // ── Nature / landscape ──────────────────────────────────────
        970 => "mountain",
        971 => "nature",
        972 => "cliff",
        973 => "coast",
        974 => "nature",        // geyser
        975 => "lake",
        976 => "beach",
        977 => "valley",
        978 => "volcano",
        979 => "mountain",
        980 => "nature",
        981..=984 => "nature",
        // ── Flowers / plants ────────────────────────────────────────
        985..=986 => "flower",
        987 => "plant",
        988 => "nature",
        989 => "mushroom",
        990..=999 => "nature",
        // Default: no useful tag
        _ => return None,
    })
}
