# Media resource links

`fileatlas-plugin-media-resources` publishes the framework-level Cordis service
`fileatlas/media-resources`. It has no UI or database and owns no source files.
Shell starts it after QuickLook when the file workbench is enabled. FileAtlas
feature plugins depend on this service to exchange stable links.

## Public contract

`fileatlas-contracts::resources` defines `ResourceKind`, `ResourceLink`,
`ResolvedResource`, and the `ResourceProvider` trait. `MediaResourceService`
starts with a local-file provider and offers:

- `link_file(path) -> ResourceLink`: canonicalizes an existing file and returns
  a stable `file:///...` URI with kind, title, provider ID, and known MIME type.
- `create_link(provider_id, source) -> ResourceLink`: asks a registered provider
  to publish its own source as a durable link.
- `resolve(uri) -> ResolvedResource`: routes URI to its provider. Existing
  absolute local paths remain accepted as compatibility inputs.
- `local_path(uri) -> PathBuf`: exposes a provider's local source/cache file
  when available. Providers may serve remote or virtual resources without one.
- `open_with_provider(uri) -> bool`: offers opening to the owning provider;
  `false` lets the consumer use a fallback viewer.
- `register_provider(Arc<dyn ResourceProvider>) -> ResourceProviderRegistration`:
  registers one provider by ID. Hold the returned lease for the owner's Cordis
  lifetime; dropping it removes the provider. Each provider decides which URI
  schemes it accepts and how links resolve.

Provider IDs must be unique. Resolution rejects zero matches and ambiguous URI
matches. Register a provider during its owning Cordis plugin's startup and keep
the lease for that plugin's lifetime so stopping the plugin removes its route.

The built-in local provider classifies common image/video/audio extensions. It
does not decode video or image content, download URLs, rewrite media, or choose
the opening application. Consumers retain those responsibilities or use their
existing services. New providers can expose remote, generated, cloud, screenshot,
clipboard, or application-native assets through their own durable URI scheme.

## Existing plugin mapping

- Whiteboard consumes catalog URIs for new image Scene sources and host file-card
  mappings. Version-1 whiteboard `path` metadata migrates to version 2 `uri`;
  older absolute image Scene sources remain resolvable. Image pixels still use
  keyed ReadModel plus the existing ThumbnailService fallback. File-card open
  resolves to a path before calling QuickLook.
- Indexing and Database remain authoritative for indexed rows, ratings, tags,
  folder identity, and media classification. They should not be moved into this
  catalog; the link is a cross-feature address, not an index record.
- Thumbnails remains the shared derived-image cache. Video poster extraction is
  still an indexing/thumbnail capability and should be exposed as a provider
  preview rendition only when a consumer requests one.
- Clipboard and Screenshot keep ownership of capture history/settings and their
  generated files. They can register a provider later to publish their assets;
  moving capture/history into this catalog would mix unrelated lifecycles.
- WorkCrate keeps its official document/artboard automation and preview cache.
  A later adapter can publish read-only artboard preview links if another feature
  needs them; it must preserve the official editor as the owner of artwork.
- QuickLook remains the host's external open handoff. DragDrop remains the native
  gesture transport; it can send resource URIs once more targets consume them.
- No video playback plugin currently provides an inline-player resource. The
  built-in local provider can identify a video URI; actual playback/seek controls
  belong to a future video provider/consumer capability.

## Diagnostics and boundaries

Inspect provider IDs, accepted URIs, local source existence, and service startup
ordering. Duplicate provider IDs fail registration. Unknown URI schemes fail
resolution instead of being treated as paths. The catalog uses an in-memory
provider registry, no persistence of its own; owners persist ResourceLink values
with their feature data. Current first-party local-file URIs are durable only as
long as the source remains at that path.
