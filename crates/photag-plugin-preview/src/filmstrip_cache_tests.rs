use super::*;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

fn fixture(name: &str) -> (PathBuf, PreviewService) {
    let directory =
        std::env::temp_dir().join(format!("photag-filmstrip-{name}-{}", std::process::id()));
    let service = PreviewService::open_at(&directory).unwrap();
    (directory, service)
}
fn wait_thumbnail(service: &PreviewService, source: &std::path::Path) -> PathBuf {
    let start = Instant::now();
    loop {
        if let Some(path) = service
            .ready_decodes
            .lock()
            .unwrap()
            .get(&(source.to_owned(), 96))
            .cloned()
        {
            return path;
        }
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn existing_gallery_thumbnail_does_not_decode_original_or_make_another_cache() {
    let (directory, service) = fixture("reuse");
    let source = directory.join("source.png");
    // It is intentionally undecodable: a ready result proves the source wasn't decoded.
    std::fs::write(&source, b"not pixel data").unwrap();
    let thumb = directory.join("gallery.jpg");
    image::RgbImage::from_pixel(40, 30, image::Rgb([20, 60, 100]))
        .save(&thumb)
        .unwrap();
    service.set_indexed_filmstrip_items([FilmstripMedia {
        path: source.clone(),
        media_id: 7,
        thumbnail: Some(thumb.clone()),
    }]);
    service.open_media(&source, Some(7)).unwrap();
    assert_eq!(wait_thumbnail(&service, &source), thumb);
    assert_eq!(service.presentation_source().1, Some(thumb));
    assert!(!service.decode_cache_path(&source, 96).exists());
    assert!(!service.decode_cache_path(&source, 1600).exists());
    drop(service);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn missing_or_stale_gallery_cache_falls_back_to_one_generated_thumbnail() {
    for stale in [false, true] {
        let (directory, service) = fixture(if stale { "stale" } else { "missing" });
        let source = directory.join("source.png");
        image::RgbImage::from_pixel(80, 60, image::Rgb([20, 60, 100]))
            .save(&source)
            .unwrap();
        let thumb = directory.join("gallery.jpg");
        if stale {
            image::RgbImage::from_pixel(10, 10, image::Rgb([255, 0, 0]))
                .save(&thumb)
                .unwrap();
            std::fs::File::options()
                .write(true)
                .open(&thumb)
                .unwrap()
                .set_modified(std::time::SystemTime::now() - Duration::from_secs(10))
                .unwrap();
        }
        service.set_indexed_filmstrip_items([FilmstripMedia {
            path: source.clone(),
            media_id: 8,
            thumbnail: Some(thumb.clone()),
        }]);
        service.open_media(&source, Some(8)).unwrap();
        let generated = wait_thumbnail(&service, &source);
        assert_ne!(generated, thumb);
        assert_eq!(image::image_dimensions(&generated).unwrap(), (80, 60));
        assert_eq!(service.presentation_source().1, Some(generated));
        drop(service);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
