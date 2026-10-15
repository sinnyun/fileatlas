//! Correlated start/result records per background job; no polling-time logging.
use super::{
    decode::DecodeJob,
    scaled_decode::{DecodeMetrics, millis},
};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

static JOB_ID: AtomicU64 = AtomicU64::new(1);

pub(super) fn run(job: &DecodeJob) -> Option<std::path::PathBuf> {
    let job_id = JOB_ID.fetch_add(1, Ordering::Relaxed);
    let started = Instant::now();
    let queue_ms = millis(started.saturating_duration_since(job.queued_at));
    let role = match job.max_dim {
        96 => "filmstrip_thumbnail",
        1600 => "progressive_preview",
        _ => "preview_cache",
    };
    tracing::debug!(target: "photag_plugin_preview", job_id, source=%job.source.display(),
        cache_path=%job.target.display(), role, urgent=job.urgent, max_dim=job.max_dim, queue_ms,
        "preview decode started");
    let check = Instant::now();
    let source = (!job.cancelled())
        .then(|| job.source.metadata().ok())
        .flatten();
    let file_bytes = source.as_ref().map(|m| m.len());
    let fresh = |path: &std::path::Path| {
        path.metadata()
            .ok()
            .zip(source.as_ref())
            .is_some_and(|(target, source)| {
                target
                    .modified()
                    .ok()
                    .zip(source.modified().ok())
                    .is_some_and(|(t, s)| t >= s)
            })
    };
    let reused = job.reuse_thumbnail.as_ref().is_some_and(|path| fresh(path));
    let output = if reused {
        job.reuse_thumbnail.clone().unwrap()
    } else {
        job.target.clone()
    };
    let cached = reused || fresh(&output);
    let cache_check_ms = millis(check.elapsed());
    let mut metrics = DecodeMetrics::default();
    let mut write_ms = 0.;
    let mut error = None;
    let mut error_stage = "none";
    let written = if job.cancelled() {
        false
    } else if cached {
        metrics.decoder = if reused {
            "gallery_thumbnail"
        } else {
            "cache_hit"
        };
        true
    } else {
        match super::scaled_decode::decode_preview(&job.source, job.max_dim, &mut metrics) {
            Ok(_) if job.cancelled() => false,
            Ok(image) => {
                let write = Instant::now();
                let result = write_cache(job, image);
                write_ms = millis(write.elapsed());
                if let Err(failure) = result {
                    error_stage = "cache_write";
                    error = Some(failure);
                    false
                } else {
                    true
                }
            }
            Err(failure) => {
                error_stage = metrics.error_stage;
                error = Some(failure);
                false
            }
        }
    };
    let duration_ms = millis(started.elapsed());
    let total_ms = ((queue_ms + duration_ms) * 1000.).round() / 1000.;
    macro_rules! record {
        ($level:expr) => {
            tracing::event!(target: "photag_plugin_preview", $level,
                job_id, source=%job.source.display(), cache_path=%output.display(),
                format=?job.source.extension().and_then(|s|s.to_str()), file_bytes,
                role, generation=job.generation, urgent=job.urgent, max_dim=job.max_dim, cached, reused, written,
                status=if job.cancelled() {"cancelled"} else if written {"ready"} else {"failed"}, cancellation_reason=job.cancellation_reason(), error_stage, error=?error,
                source_dimensions=?metrics.source_dimensions, output_dimensions=?metrics.output_dimensions,
                decoder=metrics.decoder, fallback_error=?metrics.fallback_error,
                queue_ms, cache_check_ms, read_ms=metrics.read_ms, header_ms=metrics.header_ms,
                decode_ms=metrics.decode_ms, orientation_ms=metrics.orientation_ms,
                resize_ms=metrics.resize_ms, write_ms, duration_ms, total_ms,
                duration_scope="worker_excludes_queue", "preview decode finished");
        };
    }
    if written || job.cancelled() {
        record!(tracing::Level::DEBUG);
    } else {
        record!(tracing::Level::ERROR);
    }
    (written && !job.cancelled()).then_some(output)
}

fn write_cache(job: &DecodeJob, image: image::DynamicImage) -> Result<(), String> {
    let temporary = job.target.with_extension(format!(
        "{}.tmp.jpg",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    let result = image
        .save(&temporary)
        .map_err(|e| e.to_string())
        .and_then(|_| {
            std::fs::rename(&temporary, &job.target)
                .or_else(|error| {
                    if job.target.is_file() {
                        Ok(())
                    } else {
                        Err(error)
                    }
                })
                .map_err(|e| e.to_string())
        });
    let _ = std::fs::remove_file(temporary);
    result
}
