use std::path::Path;

/// Write tags directly into the photo file (JPEG) or a sidecar (everything else).
/// `tag_paths` are the DB `full_path` values, e.g. "Animals>Birds".
/// Passing an empty slice removes any previously-written tags.
///
/// The original file's modification time is preserved across the write so
/// "date taken" UI based on `modified_at` (when EXIF date is missing) doesn't
/// jump every time the user touches a tag.
pub fn write_metadata(photo_path: &str, tag_paths: &[String]) -> std::io::Result<()> {
    let path = Path::new(photo_path);
    // Snapshot the original times so we can restore them after the write.
    let original_times = std::fs::metadata(path)
        .ok()
        .map(|m| (m.accessed().ok(), m.modified().ok()));

    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let result = match ext.as_str() {
        "jpg" | "jpeg" => write_jpeg_iptc(path, tag_paths),
        _ => write_xmp_sidecar(photo_path, tag_paths),
    };

    // Best-effort restore of access/mod times. Failures are silent — the
    // metadata write itself succeeded; mtime restoration is just a polish.
    if let Some((accessed, modified)) = original_times {
        if let (Some(a), Some(m)) = (accessed, modified) {
            if let Ok(file) = std::fs::OpenOptions::new().write(true).open(path) {
                let _ = file.set_times(std::fs::FileTimes::new().set_accessed(a).set_modified(m));
            }
        }
    }

    result
}

// ── JPEG: embed IPTC keywords directly into the file (APP13 segment) ────────
//
// Structure: FF ED [2-byte len] "Photoshop 3.0\0" [8BIM block]
// 8BIM block for IPTC: "8BIM" 04 04 [pascal name] [4-byte data len] [IPTC data]
// IPTC keyword entry:  1C 02 19 [2-byte len] [utf-8 keyword bytes]
//
// Writing modifies file bytes in-place (via temp+rename). The file-system
// watcher ignores Modify(Data) events, so no thumbnail re-generation is
// triggered.

fn write_jpeg_iptc(path: &Path, tag_paths: &[String]) -> std::io::Result<()> {
    let data = std::fs::read(path)?;
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return Ok(()); // not a valid JPEG, skip silently
    }

    let new_app13: Option<Vec<u8>> = if tag_paths.is_empty() {
        None
    } else {
        Some(build_app13(tag_paths))
    };

    let mut out = Vec::with_capacity(data.len() + new_app13.as_ref().map_or(0, |b| b.len() + 4));
    out.extend_from_slice(&data[..2]); // SOI

    let segments = parse_jpeg_segments(&data);
    let mut app13_written = false;

    for seg in &segments {
        if seg.marker == 0xED {
            // Replace (or drop) any existing APP13
            if !app13_written {
                if let Some(ref block) = new_app13 {
                    append_segment(&mut out, 0xED, block);
                }
                app13_written = true;
            }
            // always skip the old segment bytes
        } else {
            if seg.marker == 0xDA && !app13_written {
                // No prior APP13 found — insert ours before the image data
                if let Some(ref block) = new_app13 {
                    append_segment(&mut out, 0xED, block);
                }
                app13_written = true;
            }
            out.extend_from_slice(&data[seg.start..seg.end]);
        }
    }

    // Edge case: no SOS marker in file (very unusual) — still append if needed
    if !app13_written {
        if let Some(ref block) = new_app13 {
            append_segment(&mut out, 0xED, block);
        }
    }

    // Atomic write: write to a hidden temp file then rename over the original.
    // The temp file is dot-prefixed so the FS watcher ignores its creation,
    // and the rename fires a Modify(Rename) event that the watcher handles
    // harmlessly (the thumbnail is still on disk and is found immediately).
    let fname = path.file_name().unwrap_or_default().to_string_lossy();
    let tmp = path.with_file_name(format!(".{}.fileatlas_tmp", fname));
    std::fs::write(&tmp, &out)?;
    std::fs::rename(&tmp, path)
}

// ── JPEG parsing ─────────────────────────────────────────────────────────────

struct Seg {
    marker: u8,
    start: usize, // byte offset of the 0xFF marker byte
    end: usize,   // first byte after this segment
}

/// Parse JPEG segments starting after the SOI marker (byte 2).
/// SOS and everything following it is returned as a single opaque segment
/// (scan data has no length field and may contain arbitrary bytes).
fn parse_jpeg_segments(data: &[u8]) -> Vec<Seg> {
    let mut segs = Vec::new();
    let mut pos = 2;

    while pos + 1 < data.len() {
        if data[pos] != 0xFF {
            break; // corrupt stream
        }
        // Skip fill bytes (0xFF padding before marker)
        if data[pos + 1] == 0xFF {
            pos += 1;
            continue;
        }
        if data[pos + 1] == 0x00 {
            pos += 2;
            continue;
        }

        let marker = data[pos + 1];
        let seg_start = pos;

        // SOS: treat everything from here to EOF as one blob
        if marker == 0xDA {
            segs.push(Seg {
                marker,
                start: seg_start,
                end: data.len(),
            });
            break;
        }

        // EOI and restart markers: 2 bytes, no length field
        if marker == 0xD9 || (0xD0..=0xD7).contains(&marker) {
            segs.push(Seg {
                marker,
                start: seg_start,
                end: pos + 2,
            });
            pos += 2;
            if marker == 0xD9 {
                break;
            }
            continue;
        }

        // Regular segments: 2-byte length field (includes the 2 length bytes)
        if pos + 3 >= data.len() {
            break;
        }
        let seg_len = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
        if seg_len < 2 || pos + 2 + seg_len > data.len() {
            break;
        }
        segs.push(Seg {
            marker,
            start: seg_start,
            end: pos + 2 + seg_len,
        });
        pos += 2 + seg_len;
    }

    segs
}

// ── APP13 / IPTC builders ────────────────────────────────────────────────────

fn build_app13(tag_paths: &[String]) -> Vec<u8> {
    let iptc = build_iptc_keywords(tag_paths);

    // 8BIM resource block
    let mut block = Vec::new();
    block.extend_from_slice(b"8BIM");
    block.extend_from_slice(&[0x04, 0x04]); // resource type: IPTC-NAA
    block.extend_from_slice(&[0x00, 0x00]); // resource name: empty Pascal string, padded to even
    block.extend_from_slice(&(iptc.len() as u32).to_be_bytes());
    block.extend_from_slice(&iptc);
    if iptc.len() % 2 != 0 {
        block.push(0x00); // pad resource data to even length
    }

    // Full APP13 payload
    let mut app13 = Vec::new();
    app13.extend_from_slice(b"Photoshop 3.0\0"); // 14 bytes
    app13.extend_from_slice(&block);
    app13
}

/// Build IPTC record 2, dataset 25 (Keywords) entries.
/// Each entry: 1C 02 19 [big-endian u16 len] [utf-8 keyword]
/// We use the leaf tag name (last component of "Parent>Child") so the keywords
/// are plain strings readable by any IPTC-aware tool.
fn build_iptc_keywords(tag_paths: &[String]) -> Vec<u8> {
    let mut iptc = Vec::new();
    for fp in tag_paths {
        let keyword = fp.rsplit('>').next().unwrap_or(fp.as_str()).trim();
        if keyword.is_empty() {
            continue;
        }
        let kw = keyword.as_bytes();
        iptc.push(0x1C); // tag marker
        iptc.push(0x02); // record 2
        iptc.push(0x19); // dataset 25 = Keywords
        iptc.extend_from_slice(&(kw.len() as u16).to_be_bytes());
        iptc.extend_from_slice(kw);
    }
    iptc
}

fn append_segment(out: &mut Vec<u8>, marker: u8, payload: &[u8]) {
    let len = (payload.len() + 2) as u16; // length field includes itself
    out.push(0xFF);
    out.push(marker);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(payload);
}

// ── XMP sidecar fallback (non-JPEG formats) ──────────────────────────────────
//
// Writes a .xmp file next to the photo with dc:subject (leaf names) and
// lr:hierarchicalSubject (full paths with | separator for Lightroom).

fn write_xmp_sidecar(photo_path: &str, tag_paths: &[String]) -> std::io::Result<()> {
    let sidecar = Path::new(photo_path).with_extension("xmp");

    if tag_paths.is_empty() {
        if sidecar.exists() {
            std::fs::remove_file(&sidecar)?;
        }
        return Ok(());
    }

    let dc_items: String = tag_paths
        .iter()
        .map(|fp| {
            let name = fp.rsplit('>').next().unwrap_or(fp.as_str());
            format!("          <rdf:li>{}</rdf:li>\n", escape_xml(name))
        })
        .collect();

    let lr_items: String = tag_paths
        .iter()
        .map(|fp| {
            let lr_path = fp.replace('>', "|");
            format!("          <rdf:li>{}</rdf:li>\n", escape_xml(&lr_path))
        })
        .collect();

    let xmp = format!(
        r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
        xmlns:dc="http://purl.org/dc/elements/1.1/"
        xmlns:lr="http://ns.adobe.com/lightroom/1.0/">
      <dc:subject>
        <rdf:Bag>
{dc_items}        </rdf:Bag>
      </dc:subject>
      <lr:hierarchicalSubject>
        <rdf:Bag>
{lr_items}        </rdf:Bag>
      </lr:hierarchicalSubject>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#
    );

    std::fs::write(&sidecar, xmp)
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::write_metadata;
    use std::time::{Duration, SystemTime};

    fn temp_root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "fileatlas-xmp-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn sidecar_round_trips_hierarchical_tags_and_preserves_source_file() {
        let root = temp_root();
        std::fs::create_dir_all(&root).unwrap();
        let photo = root.join("sample.png");
        let original = b"synthetic image payload";
        std::fs::write(&photo, original).unwrap();

        write_metadata(
            photo.to_str().unwrap(),
            &["People>A&B <C\"D\"'".to_string()],
        )
        .unwrap();

        let sidecar = std::fs::read_to_string(root.join("sample.xmp")).unwrap();
        assert!(
            sidecar.contains("<rdf:li>A&amp;B &lt;C&quot;D&quot;&apos;</rdf:li>"),
            "{sidecar}"
        );
        assert!(sidecar.contains("<rdf:li>People|A&amp;B &lt;C&quot;D&quot;&apos;</rdf:li>"));
        assert_eq!(std::fs::read(&photo).unwrap(), original);

        write_metadata(photo.to_str().unwrap(), &[]).unwrap();
        assert!(!root.join("sample.xmp").exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn jpeg_write_embeds_and_removes_iptc_while_restoring_modified_time() {
        let root = temp_root();
        std::fs::create_dir_all(&root).unwrap();
        let photo = root.join("sample.jpg");
        // Minimal segment sequence with an opaque scan payload.
        let jpeg = [0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x02, 0x11, 0x22, 0xFF, 0xD9];
        std::fs::write(&photo, jpeg).unwrap();
        let expected_time = SystemTime::now() - Duration::from_secs(3_600);
        std::fs::File::options()
            .write(true)
            .open(&photo)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(expected_time))
            .unwrap();

        write_metadata(photo.to_str().unwrap(), &["Places>Oslo".to_string()]).unwrap();
        let tagged = std::fs::read(&photo).unwrap();
        assert_eq!(tagged[0..2], [0xFF, 0xD8]);
        assert!(tagged.windows(3).any(|window| window == [0xFF, 0xED, 0x00]));
        assert!(tagged.windows(4).any(|window| window == b"Oslo"));
        assert_eq!(
            std::fs::metadata(&photo)
                .unwrap()
                .modified()
                .unwrap()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            expected_time
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );

        write_metadata(photo.to_str().unwrap(), &[]).unwrap();
        let cleared = std::fs::read(&photo).unwrap();
        assert!(!cleared.windows(2).any(|window| window == [0xFF, 0xED]));
        assert!(cleared.ends_with(&[0x11, 0x22, 0xFF, 0xD9]));
        let _ = std::fs::remove_dir_all(root);
    }
}
