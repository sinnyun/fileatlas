# PhoTag development test media

`photag-library/` contains small, redistributable generated fixtures for local
scanner, thumbnail and Gallery checks. It covers JPEG, PNG, WebP, GIF, TIFF,
BMP and WAV; file dimensions and sizes vary, and nested directories exercise
recursive enumeration. `04-no-thumbnail/broken-image.jpg` is intentionally
invalid, while `sample.wav` is a valid audio file for a media type that does
not get an image thumbnail. Scanning the valid image files generates actual
thumbnails in the configured PhoTag thumbnail cache. `.dev-support/preview.png`
is hidden from scanning and only provides a real thumbnail path for virtual
benchmark rows.

Regenerate the assets with:

```powershell
cargo +1.97.1 run -p photag-plugin-dev-test --example generate_test_media --locked
```

The `dev-test` development plugin scans this directory and can inject 1,000,
10,000 or 100,000 synthetic database records. Those virtual rows have
deterministic media types, dimensions, sizes and ratings; one quarter references
the hidden preview thumbnail and the rest have no thumbnail path. They do not
create corresponding files. The plugin only runs when selected with
`PHOTAG_DEV_PLUGINS=dev-test`, and uses `target/dev-test/photag.db` by default
instead of the normal PhoTag user database. Set `PHOTAG_DEV_TEST_DB` to override
that development database, or `PHOTAG_TEST_LIBRARY` to scan another fixture
directory.
