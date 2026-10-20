//! Compile-time Cordis grouping provider. FileManager only renders its ordered sections.
//! Month behavior follows PhoTag commit 36cee242a78c0a8cd9ac6b0d63b64265aa0add90.
//! Upstream license: `vendor/photag-source/LICENSE`.
use cordis::{Context, Plugin, Service};
use fileatlas_plugin_database::{MediaGroupingFields, MediaSkeleton, SortOrder};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, ops::Range, sync::Arc};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupingMode {
    None,
    #[default]
    Month,
    FileType,
    Modified,
    Created,
    Folder,
    Subfolder,
}
impl GroupingMode {
    pub const ALL: [Self; 7] = [
        Self::None,
        Self::Month,
        Self::FileType,
        Self::Modified,
        Self::Created,
        Self::Folder,
        Self::Subfolder,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Month => "month",
            Self::FileType => "file_type",
            Self::Modified => "modified",
            Self::Created => "created",
            Self::Folder => "folder",
            Self::Subfolder => "subfolder",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.key() == value)
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "No grouping",
            Self::Month => "Group by month",
            Self::FileType => "Group by file type",
            Self::Modified => "Group by modified month",
            Self::Created => "Group by created month",
            Self::Folder => "Group by root folder",
            Self::Subfolder => "Group by subfolder",
        }
    }
    pub fn is_date(self) -> bool {
        matches!(self, Self::Month | Self::Modified | Self::Created)
    }
}

#[derive(Clone, Debug)]
pub struct GroupRow {
    pub media_range: Range<usize>,
    pub month_label: Option<String>,
    pub collapsed: bool,
    pub month_count: usize,
}

pub struct GroupingService;
impl Service for GroupingService {
    const NAME: &'static str = "fileatlas/grouping";
}
impl GroupingService {
    /// Stable group ordering preserves the selected query sort inside each section.
    /// Date sections are newest first except for the explicit oldest-first sort.
    pub fn arrange(
        &self,
        skeletons: &mut Vec<MediaSkeleton>,
        fields: &[MediaGroupingFields],
        mode: GroupingMode,
        sort: SortOrder,
    ) -> Vec<Option<String>> {
        self.arrange_in_scope(skeletons, fields, mode, sort, None)
    }

    /// In a selected directory, its direct files precede descendant groups.
    /// Across the library, registered roots precede all descendant groups.
    pub fn arrange_in_scope(
        &self,
        skeletons: &mut Vec<MediaSkeleton>,
        fields: &[MediaGroupingFields],
        mode: GroupingMode,
        sort: SortOrder,
        directory: Option<&str>,
    ) -> Vec<Option<String>> {
        let roots: HashSet<String> = directory
            .map(|path| HashSet::from([path_key(path)]))
            .unwrap_or_else(|| {
                fields
                    .iter()
                    .map(|field| path_key(&field.root_path))
                    .collect()
            });
        let fields: std::collections::HashMap<_, _> = fields.iter().map(|f| (f.id, f)).collect();
        let mut entries: Vec<_> = skeletons
            .iter()
            .copied()
            .map(|s| {
                let field = fields.get(&s.id);
                let label = match mode {
                    GroupingMode::None => None,
                    GroupingMode::Month => Some(month(s.effective_date)),
                    GroupingMode::Modified => {
                        Some(month(field.map(|f| f.modified_at).unwrap_or(0)))
                    }
                    GroupingMode::Created => Some(month(
                        field
                            .and_then(|f| {
                                std::fs::metadata(&f.full_path)
                                    .ok()?
                                    .created()
                                    .ok()?
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .ok()
                                    .map(|d| d.as_secs() as i64)
                            })
                            .unwrap_or(0),
                    )),
                    GroupingMode::FileType => Some(
                        field
                            .and_then(|f| std::path::Path::new(&f.full_path).extension())
                            .map(|e| e.to_string_lossy().to_uppercase())
                            .filter(|e| !e.is_empty())
                            .unwrap_or_else(|| "No extension".into()),
                    ),
                    GroupingMode::Folder => Some(
                        field
                            .map(|f| f.root_path.clone())
                            .unwrap_or_else(|| "Unknown folder".into()),
                    ),
                    GroupingMode::Subfolder => Some(
                        field
                            .and_then(|f| std::path::Path::new(&f.full_path).parent())
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "Unknown folder".into()),
                    ),
                };
                let rank = if mode == GroupingMode::Subfolder {
                    match label.as_deref() {
                        Some("Unknown folder") | None => 2,
                        Some(path) => u8::from(!roots.contains(&path_key(path))),
                    }
                } else {
                    0
                };
                (s, label, rank)
            })
            .collect();
        entries.sort_by(|a, b| {
            let unknown_a = a.1.as_deref() == Some("Unknown date");
            let unknown_b = b.1.as_deref() == Some("Unknown date");
            a.2.cmp(&b.2)
                .then_with(|| unknown_a.cmp(&unknown_b))
                .then_with(|| {
                    if mode.is_date() && sort != SortOrder::OldestFirst {
                        b.1.cmp(&a.1)
                    } else {
                        a.1.cmp(&b.1)
                    }
                })
        });
        *skeletons = entries.iter().map(|e| e.0).collect();
        entries.into_iter().map(|e| e.1).collect()
    }

    pub fn rows(
        labels: &[Option<String>],
        columns: usize,
        collapsed: &HashSet<String>,
    ) -> Vec<GroupRow> {
        let mut rows = Vec::new();
        let mut start = 0;
        while start < labels.len() {
            let label = &labels[start];
            let mut end = start + 1;
            while end < labels.len() && labels[end] == *label {
                end += 1;
            }
            let is_collapsed = label.as_ref().is_some_and(|l| collapsed.contains(l));
            let count = end - start;
            let mut row_start = start;
            while row_start < end {
                let row_end = if is_collapsed {
                    end
                } else {
                    (row_start + columns.max(1)).min(end)
                };
                rows.push(GroupRow {
                    media_range: row_start..row_end,
                    month_label: (row_start == start).then(|| label.clone()).flatten(),
                    collapsed: is_collapsed,
                    month_count: if row_start == start { count } else { 0 },
                });
                row_start = row_end;
            }
            start = end;
        }
        rows
    }
}
// Windows indexed paths may mix extended prefixes, separator styles and casing.
// Compare lexically without filesystem IO, so unavailable directories still group.
fn path_key(path: &str) -> String {
    let slashes = path.replace('\\', "/");
    let normal = if let Some(unc) = slashes.strip_prefix("//?/UNC/") {
        format!("//{unc}")
    } else {
        slashes.strip_prefix("//?/").unwrap_or(&slashes).to_string()
    };
    let key = normal.trim_end_matches('/').to_string();
    if cfg!(windows) {
        key.to_lowercase()
    } else {
        key
    }
}

fn month(timestamp: i64) -> String {
    if timestamp <= 0 {
        return "Unknown date".into();
    }
    chrono::DateTime::from_timestamp(timestamp, 0)
        .map(|d| d.with_timezone(&chrono::Local).format("%Y-%m").to_string())
        .unwrap_or_else(|| "Unknown date".into())
}

/// Translate source English month collapse keys when reading an older session.
pub fn restore_collapsed(keys: &HashSet<String>, mode: GroupingMode) -> HashSet<String> {
    keys.iter()
        .map(|key| {
            if mode == GroupingMode::Month {
                chrono::NaiveDate::parse_from_str(&format!("1 {key}"), "%d %B %Y")
                    .map(|d| d.format("%Y-%m").to_string())
                    .unwrap_or_else(|_| key.clone())
            } else {
                key.clone()
            }
        })
        .collect()
}

pub struct GroupingPlugin;
impl Plugin for GroupingPlugin {
    type Config = ();
    type Input = ();
    type PrepareError = std::convert::Infallible;
    type ApplyError = std::io::Error;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        "fileatlas.grouping".into()
    }
    fn prepare(&self, _: ()) -> Result<(), Self::PrepareError> {
        Ok(())
    }
    #[tracing::instrument(skip_all, err)]
    async fn apply(&self, context: Context, _: &()) -> Result<(), std::io::Error> {
        let _publication = context
            .provide(Arc::new(GroupingService))
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        fileatlas_ui_bridge::register_for_plugin(
            &context,
            "fileatlas.grouping",
            [fileatlas_ui_bridge::UiContribution::new(
                "grouping/modes",
                fileatlas_ui_bridge::ContributionKind::Widget {
                    label: "Grouping".into(),
                    slot: "file_manager.grouping".into(),
                    order: 0,
                },
            )],
        )
        .map_err(|e| std::io::Error::other(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
