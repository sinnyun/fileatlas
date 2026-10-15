use super::*;
use crate::GalleryService;

pub(crate) type MatchResults = (Vec<i64>, Vec<MediaItem>);
pub(super) enum MatchRequest {
    Selection(Vec<i64>),
    Query(MediaQuery, Vec<i64>),
}
impl MatchRequest {
    pub(super) fn load(self, service: &GalleryService) -> Result<MatchResults, String> {
        match self {
            Self::Selection(ids) => {
                let media = ids
                    .iter()
                    .map(|id| {
                        service
                            .media_by_id(*id)
                            .map_err(|error| error.to_string())?
                            .ok_or_else(|| format!("Selected media {id} is no longer indexed."))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok((ids, media))
            }
            Self::Query(query, tags) => {
                let snapshot = service.load(query).map_err(|error| error.to_string())?;
                let ids = snapshot
                    .skeletons
                    .iter()
                    .filter(|item| {
                        tags.is_empty()
                            || snapshot.media_tags.get(&item.id).is_some_and(|assigned| {
                                assigned.iter().any(|tag| tags.contains(&tag.id))
                            })
                    })
                    .map(|item| item.id)
                    .collect::<Vec<_>>();
                let set = ids
                    .iter()
                    .copied()
                    .collect::<std::collections::HashSet<_>>();
                let media = snapshot
                    .media
                    .into_iter()
                    .filter(|media| set.contains(&media.id))
                    .collect();
                Ok((ids, media))
            }
        }
    }
}

pub(crate) fn poll_matches(view: &mut GalleryView, cx: &mut Context<GalleryView>) -> bool {
    // Raw inputs form a stable identity; relative-date cutoffs are computed
    // once per edit, not once per timer tick. Execute still revalidates targets.
    let fields = (0..8).map(|i| read(view, i, cx)).collect::<Vec<_>>();
    let browser = view
        .snapshot
        .as_ref()
        .ok()
        .map(|s| (s.browser.active, s.browser.directory.clone()));
    let key = format!(
        "{:?}",
        (
            (
                view.bulk_source,
                view.bulk_media_type,
                fields,
                &view.bulk_tag_ids,
                view.bulk_extension_exclude,
                view.bulk_date_from_value,
                view.bulk_date_to_value
            ),
            (
                view.filter.folder_id,
                &view.filter.path_prefix,
                &view.filter.selected_tag_ids,
                &view.selection.selected_media_ids,
                browser,
                view.observed_index_revision,
                view.service.data_revision()
            ),
        )
    );
    let request = matching_request(view, cx);
    let service = Arc::clone(&view.service);
    view.bulk_read.update(key, move || request?.load(&service))
}
