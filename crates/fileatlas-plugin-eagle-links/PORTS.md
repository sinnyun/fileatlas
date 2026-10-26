# Eagle link conversion

## Identity and ownership

- Cordis plugin: `fileatlas.eagle-links`; service: `fileatlas/eagle-links`.
- Converts locations supplied by Eagle's Plugin API into canonical canvas
  `ResourceLink` values. It owns no UI, database, settings, or file copies.
- Depends on `fileatlas-plugin-media-resources`; its service is the sole owner
  of local file URI canonicalization and resource classification.

## Public operation

`EagleLinkService::convert(&str) -> Result<ResourceLink, String>` accepts an
Eagle item `fileURL` (`file:///...`) or `filePath`. It rejects empty, non-local,
missing, and non-file locations. The returned URI is the same stable `file://`
form used by canvas resource imports, with resource kind and title populated by
the shared provider.

`EagleLinkService::link_item_by_id(&str) -> Result<ResourceLink, String>` uses
Eagle Web API V2 `item/get?id=...` for the ID, extension, byte size, and
`noThumbnail` flag, then the V1 `item/thumbnail` endpoint for the item directory.
When a thumbnail exists, it only accepts one sibling whose extension and byte
size match Eagle's item metadata. For `noThumbnail` items, Eagle's reported file
is itself linked directly. Ambiguous or unavailable files fail closed. This
requires Eagle 4.0 Build 21+ for V2.

`source_for_dropped_image(&Path) -> Result<Option<EagleImageSource>, String>`
recognizes an image dropped from an Eagle `.info` item directory, resolves its
item metadata plus assigned folder names, confirms the dropped path is the
original (not the thumbnail), and returns the original path/URI, thumbnail
path, item ID, and direct Web API item URL. Non-Eagle paths return `None`.

Eagle's `eagle://item/{id}` app deep link is not itself a file location. Pass
the extracted ID to `link_item_by_id`; it queries the running Eagle Web API and
locates the matching original beside the API-reported thumbnail. The direct
Plugin API route remains available to callers that already have `item.fileURL`
or `item.filePath`. This plugin never modifies Eagle metadata.

## Lifecycle and failures

The shell starts this plugin after the shared media-resource catalog whenever
file browsing is enabled. It publishes one Cordis service and has no events or
background workers. Failures are returned to the caller as descriptive strings;
inspect shell startup diagnostics if the service is unavailable.

## Validation limits

`convert` is a local-path operation. `link_item_by_id` requires Eagle running
and its local API enabled. The `.info` sibling layout and matching by extension
plus byte size have been checked against the active local library, but Eagle
does not document that original-file lookup as a stable API contract. The
whiteboard calls this service from the native image-drop path and persists the
returned metadata on that board. Broader Windows path and duplicate-size edge
cases remain unverified.

`EagleLinkService::open_item(&str) -> Result<(), String>` validates a non-empty
alphanumeric Eagle item ID and hands `eagle://item/{id}` to the operating system's
registered URL handler with `open::that_detached`. This uses Eagle's supported
item deep link to open/locate the item. It does not read or change Eagle content.
Failure to dispatch (for example, no URL handler registered) returns an error to
the canvas status message. Eagle deep-link behavior: https://www.eagle.cool/blog/post/eagle4-build7.
