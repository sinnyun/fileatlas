# Source pins

These exact upstream commits define the migration baseline for this workspace.
The full selected PhoTag source is mirrored under `vendor/photag-source/`.

| Project | URL | Commit | Purpose |
|---|---|---|---|
| PhoTag | https://github.com/OleksandrShabaldas/PhoTag | `36cee242a78c0a8cd9ac6b0d63b64265aa0add90` | Feature and behavior baseline |
| cordis-rs | https://github.com/dshbox/cordis-rs | `d77cf919964f657b122b5eac9a7f723c0b002ca8` | Lifecycle, services, events, dependencies |
| GPUI Kit | https://github.com/longbridge/gpui-kit | `1e7ffaa6d1c0c230cb84099584e6c057531d5a5d` | Native GPUI components and application UI |

PhoTag baseline commit date: 2026-08-04. The runtime and UI toolkit revisions
were cloned from their default branches on 2026-10-12 and frozen here for API
review.
