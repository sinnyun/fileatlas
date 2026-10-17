use super::*;
use lore_vm::ops::branch::{
    archive::{self as branch_archive, BranchArchiveArgs},
    create::{self as branch_create, BranchCreateArgs},
    list::{self as branch_list, BranchListArgs, BranchListEntry},
    merge_abort::{self as merge_abort, BranchMergeAbortArgs},
    merge_resolve::{self as merge_resolve, BranchMergeResolveArgs},
    merge_resolve_mine::{self as merge_resolve_mine, BranchMergeResolveMineArgs},
    merge_resolve_theirs::{self as merge_resolve_theirs, BranchMergeResolveTheirsArgs},
    merge_start::{self as merge_start, BranchMergeStartArgs},
    switch::{self as branch_switch, BranchSwitchArgs},
};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchMergeResult {
    pub source_branch: String,
    pub target_branch: String,
    pub merge_revision_id: Option<String>,
    pub has_conflicts: bool,
    pub conflict_files: Vec<String>,
    pub completed: bool,
}

impl VersionsService {
    pub async fn branch_graph(
        &self,
        path: &str,
        include_archived: bool,
    ) -> Result<VersionGraph, VersionsError> {
        let (key, _) = Self::key(Path::new(path))?;
        let _guard = self.lock(&key).await?;
        let (_, work, store) = self.paths(&key);
        if !work.join(".lore").exists() {
            return Ok(VersionGraph {
                branches: Vec::new(),
                graph_state: "empty".into(),
                pending_merge: None,
            });
        }
        let api = self.open(&key, &work, &store).await?;
        let listed = branch_list::list(
            &api,
            BranchListArgs {
                archived: include_archived,
            },
        )
        .await?;
        let known = self
            .info
            .list(&key)?
            .into_iter()
            .map(|record| (record.version_id.clone(), record))
            .collect::<HashMap<_, _>>();
        let mut branches = Vec::with_capacity(listed.entries.len());
        for branch in listed.entries {
            branches.push(self.project_branch(&api, &key, branch, &known).await?);
        }
        let mut graph = self.info.graph(&key)?;
        graph.branches = branches;
        graph.graph_state = "ready".into();
        self.info.put_graph(&key, &graph)?;
        Ok(graph)
    }

    async fn project_branch(
        &self,
        api: &LoreApi,
        key: &str,
        branch: BranchListEntry,
        known: &HashMap<String, VersionRecord>,
    ) -> Result<VersionBranch, VersionsError> {
        let history_entries = history::history(
            api,
            RevisionHistoryArgs {
                branch: branch.name.clone(),
                only_branch: true,
                ..Default::default()
            },
        )
        .await;
        let history_entries = match history_entries {
            Ok(history) => Some(history.entries),
            Err(error) if branch.archived && error.to_string().contains("branch not found") => None,
            Err(error) => return Err(error.into()),
        };
        let mut revisions = Vec::new();
        if let Some(history_entries) = history_entries {
            for revision in history_entries {
                let record = if let Some(record) = known.get(&revision.revision) {
                    record.clone()
                } else {
                    let details = info::info(
                        api,
                        RevisionInfoArgs {
                            revision: revision.revision.clone(),
                            delta: true,
                            metadata: true,
                        },
                    )
                    .await?;
                    VersionRecord {
                        entity_id: key.into(),
                        version_id: revision.revision.clone(),
                        revision_number: revision.revision_number,
                        parent_revision_ids: revision.parents.clone(),
                        title: details.message().unwrap_or("Lore revision").into(),
                        created_at: details.timestamp().and_then(parse_time).unwrap_or_else(now),
                        size_bytes: details
                            .deltas
                            .iter()
                            .filter(|delta| delta.path.replace('\\', "/") == "content")
                            .map(|delta| delta.size)
                            .max()
                            .unwrap_or(0),
                        content_type: None,
                        thumbnail_path: None,
                        is_current: false,
                    }
                };
                revisions.push(record);
            }
        } else {
            let mut revision_id = branch.latest.clone();
            let mut seen = HashSet::new();
            while !revision_id.is_empty() && seen.insert(revision_id.clone()) {
                let Some(record) = known.get(&revision_id) else {
                    break;
                };
                revisions.push(record.clone());
                revision_id = record
                    .parent_revision_ids
                    .first()
                    .cloned()
                    .unwrap_or_default();
            }
        }
        Ok(VersionBranch {
            branch_id: branch.id,
            name: branch.name,
            category: branch.category,
            head_revision_id: (!branch.latest.is_empty()).then_some(branch.latest),
            is_current: branch.is_current,
            is_archived: branch.archived,
            created_at_unix: branch.created,
            stack: branch
                .stack
                .into_iter()
                .map(|point| VersionBranchPoint {
                    branch_id: point.branch,
                    revision_id: point.revision,
                })
                .collect(),
            revisions,
        })
    }

    pub async fn create_branch(
        &self,
        path: &str,
        name: &str,
    ) -> Result<VersionBranch, VersionsError> {
        let name = validate_branch_name(name)?;
        let (key, path) = Self::key(Path::new(path))?;
        let _guard = self.lock(&key).await?;
        let (_, work, store) = self.paths(&key);
        if !work.join(".lore").exists() {
            return Err(VersionsError::NotFound);
        }
        let api = self.open(&key, &work, &store).await?;
        self.require_clean_snapshot(&api, &path, &work.join("content"))
            .await?;
        self.ensure_no_pending_merge(&key)?;
        let result = branch_create::create(
            &api,
            BranchCreateArgs {
                branch: name.to_owned(),
                category: "feature".into(),
                id: String::new(),
            },
        )
        .await?;
        flush::flush(&api).await?;
        let branch = branch_list::list(&api, BranchListArgs { archived: false })
            .await?
            .entries
            .into_iter()
            .find(|branch| branch.name == result.name)
            .ok_or(VersionsError::NotFound)?;
        self.project_branch(&api, &key, branch, &self.version_map(&key)?)
            .await
    }

    pub async fn switch_branch(
        &self,
        path: &str,
        name: &str,
        overwrite: bool,
    ) -> Result<String, VersionsError> {
        let name = validate_branch_name(name)?;
        let (key, path) = Self::key(Path::new(path))?;
        let _guard = self.lock(&key).await?;
        let (content, work, store) = self.paths(&key);
        if !work.join(".lore").exists() {
            return Err(VersionsError::NotFound);
        }
        let api = self.open(&key, &work, &store).await?;
        self.ensure_no_pending_merge(&key)?;
        if !overwrite {
            self.require_clean_snapshot(&api, &path, &content).await?;
        }
        let exists = branch_list::list(&api, BranchListArgs { archived: false })
            .await?
            .entries
            .iter()
            .any(|branch| branch.name == name);
        if !exists {
            return Err(VersionsError::NotFound);
        }
        let switched = branch_switch::switch(
            &api,
            BranchSwitchArgs {
                branch: name.into(),
                revision: String::new(),
                reset: overwrite,
                bare: false,
            },
        )
        .await?;
        flush::flush(&api).await?;
        replace_atomic(&content, &path)?;
        let head = Self::status(&api, false)
            .await?
            .revision
            .map(|revision| revision.revision);
        if let Some(revision) = head.as_deref() {
            self.record_revision(&api, &key, &path, revision).await?;
        }
        Ok(switched.branch)
    }

    pub async fn archive_branch(&self, path: &str, name: &str) -> Result<(), VersionsError> {
        let name = validate_branch_name(name)?;
        let (key, path) = Self::key(Path::new(path))?;
        let _guard = self.lock(&key).await?;
        let (_, work, store) = self.paths(&key);
        if !work.join(".lore").exists() {
            return Err(VersionsError::NotFound);
        }
        let api = self.open(&key, &work, &store).await?;
        self.require_clean_snapshot(&api, &path, &work.join("content"))
            .await?;
        self.ensure_no_pending_merge(&key)?;
        let branches = branch_list::list(&api, BranchListArgs { archived: false }).await?;
        let selected = branches.entries.iter().find(|branch| branch.name == name);
        if selected.is_some_and(|branch| branch.is_current) {
            return Err(VersionsError::CurrentBranch);
        }
        if selected.is_none() {
            return Err(VersionsError::NotFound);
        }
        branch_archive::archive(
            &api,
            BranchArchiveArgs {
                branch: name.into(),
            },
        )
        .await?;
        flush::flush(&api).await?;
        Ok(())
    }

    pub async fn merge_branch(
        &self,
        path: &str,
        source_branch: &str,
    ) -> Result<BranchMergeResult, VersionsError> {
        let source_branch = validate_branch_name(source_branch)?;
        let (key, path) = Self::key(Path::new(path))?;
        let _guard = self.lock(&key).await?;
        let (content, work, store) = self.paths(&key);
        if !work.join(".lore").exists() {
            return Err(VersionsError::NotFound);
        }
        let api = self.open(&key, &work, &store).await?;
        self.require_clean_snapshot(&api, &path, &content).await?;
        self.ensure_no_pending_merge(&key)?;
        let branches = branch_list::list(&api, BranchListArgs { archived: false }).await?;
        let current = branches
            .entries
            .iter()
            .find(|branch| branch.is_current)
            .ok_or(VersionsError::NotFound)?;
        if current.name == source_branch {
            return Err(VersionsError::MergeIntoSelf);
        }
        if !branches
            .entries
            .iter()
            .any(|branch| branch.name == source_branch)
        {
            return Err(VersionsError::NotFound);
        }
        let target_branch = current.name.clone();
        let base_revision = current.latest.clone();
        let merged = merge_start::merge_start(
            &api,
            BranchMergeStartArgs {
                branch: source_branch.to_owned(),
                message: format!("Merge {source_branch} into {target_branch}"),
                no_commit: false,
                link: String::new(),
                ignore_links: false,
            },
        )
        .await?;
        if merged.has_conflicts {
            let mut graph = self.info.graph(&key)?;
            graph.pending_merge = Some(PendingVersionMerge {
                source_branch: source_branch.to_owned(),
                target_branch: target_branch.clone(),
                conflict_files: merged.conflict_files.clone(),
                target_base_revision_id: base_revision,
            });
            self.info.put_graph(&key, &graph)?;
            flush::flush(&api).await?;
            return Ok(BranchMergeResult {
                source_branch: source_branch.to_owned(),
                target_branch,
                merge_revision_id: None,
                has_conflicts: true,
                conflict_files: merged.conflict_files,
                completed: false,
            });
        }
        flush::flush(&api).await?;
        replace_atomic(&content, &path)?;
        let revision = self.record_head(&api, &key, &path).await?;
        Ok(BranchMergeResult {
            source_branch: source_branch.to_owned(),
            target_branch,
            merge_revision_id: revision,
            has_conflicts: false,
            conflict_files: Vec::new(),
            completed: true,
        })
    }

    pub async fn resolve_merge(
        &self,
        path: &str,
        strategy: &str,
    ) -> Result<BranchMergeResult, VersionsError> {
        if !matches!(strategy, "ours" | "theirs") {
            return Err(VersionsError::InvalidMergeStrategy);
        }
        let (key, path) = Self::key(Path::new(path))?;
        let _guard = self.lock(&key).await?;
        let (content, work, store) = self.paths(&key);
        let mut graph = self.info.graph(&key)?;
        let mut pending = graph
            .pending_merge
            .clone()
            .ok_or(VersionsError::PendingMerge)?;
        let api = self.open(&key, &work, &store).await?;
        let selected = pending.conflict_files.clone();
        if selected.is_empty() {
            return Err(VersionsError::PendingMerge);
        }
        if strategy == "ours" {
            merge_resolve_mine::merge_resolve_mine(
                &api,
                BranchMergeResolveMineArgs {
                    paths: selected.clone(),
                },
            )
            .await?;
        } else {
            merge_resolve_theirs::merge_resolve_theirs(
                &api,
                BranchMergeResolveTheirsArgs {
                    paths: selected.clone(),
                },
            )
            .await?;
        }
        let resolved =
            merge_resolve::merge_resolve(&api, BranchMergeResolveArgs { paths: selected }).await?;
        pending
            .conflict_files
            .retain(|file| !resolved.resolved_paths.contains(file));
        if !pending.conflict_files.is_empty() {
            graph.pending_merge = Some(pending.clone());
            self.info.put_graph(&key, &graph)?;
            flush::flush(&api).await?;
            return Ok(BranchMergeResult {
                source_branch: pending.source_branch,
                target_branch: pending.target_branch,
                merge_revision_id: Some(resolved.revision),
                has_conflicts: true,
                conflict_files: pending.conflict_files,
                completed: false,
            });
        }
        let commit = commit::commit(
            &api,
            CommitArgs {
                message: format!(
                    "Merge {} into {}",
                    pending.source_branch, pending.target_branch
                ),
            },
        )
        .await?;
        flush::flush(&api).await?;
        replace_atomic(&content, &path)?;
        self.record_revision(&api, &key, &path, &commit.revision)
            .await?;
        graph.pending_merge = None;
        self.info.put_graph(&key, &graph)?;
        Ok(BranchMergeResult {
            source_branch: pending.source_branch,
            target_branch: pending.target_branch,
            merge_revision_id: Some(commit.revision),
            has_conflicts: false,
            conflict_files: Vec::new(),
            completed: true,
        })
    }

    pub async fn abort_merge(&self, path: &str) -> Result<(), VersionsError> {
        let (key, _) = Self::key(Path::new(path))?;
        let _guard = self.lock(&key).await?;
        let (_, work, store) = self.paths(&key);
        let mut graph = self.info.graph(&key)?;
        if graph.pending_merge.is_none() {
            return Err(VersionsError::PendingMerge);
        }
        let api = self.open(&key, &work, &store).await?;
        merge_abort::merge_abort(
            &api,
            BranchMergeAbortArgs {
                link: String::new(),
                ignore_links: false,
            },
        )
        .await?;
        flush::flush(&api).await?;
        graph.pending_merge = None;
        self.info.put_graph(&key, &graph)?;
        Ok(())
    }

    fn version_map(&self, key: &str) -> Result<HashMap<String, VersionRecord>, VersionsError> {
        Ok(self
            .info
            .list(key)?
            .into_iter()
            .map(|record| (record.version_id.clone(), record))
            .collect())
    }

    fn ensure_no_pending_merge(&self, key: &str) -> Result<(), VersionsError> {
        if self.info.graph(key)?.pending_merge.is_some() {
            Err(VersionsError::PendingMerge)
        } else {
            Ok(())
        }
    }

    async fn require_clean_snapshot(
        &self,
        api: &LoreApi,
        path: &Path,
        content: &Path,
    ) -> Result<(), VersionsError> {
        let status = Self::status(api, true).await?;
        if !status.files.is_empty() || !same_file(path, content)? {
            return Err(VersionsError::UnsavedChanges);
        }
        Ok(())
    }

    async fn record_head(
        &self,
        api: &LoreApi,
        key: &str,
        path: &Path,
    ) -> Result<Option<String>, VersionsError> {
        let head = Self::status(api, false).await?.revision.map(|r| r.revision);
        if let Some(revision) = head.as_deref() {
            self.record_revision(api, key, path, revision).await?;
        }
        Ok(head)
    }

    async fn record_revision(
        &self,
        api: &LoreApi,
        key: &str,
        path: &Path,
        revision_id: &str,
    ) -> Result<(), VersionsError> {
        let details = info::info(
            api,
            RevisionInfoArgs {
                revision: revision_id.into(),
                delta: true,
                metadata: true,
            },
        )
        .await?;
        let history = history::history(api, RevisionHistoryArgs::default()).await?;
        let revision = history
            .entries
            .into_iter()
            .find(|entry| entry.revision == revision_id);
        let thumbs = self.thumbs.clone();
        let source_path = path.to_path_buf();
        let preview_root = self.root.join("previews").join(key);
        let revision_name = format!("{revision_id}.jpg");
        let thumbnail_path = tokio::task::spawn_blocking(move || {
            thumbs.generate_image(&source_path).and_then(|generated| {
                let source = generated.0;
                let target = preview_root.join(revision_name);
                target
                    .parent()
                    .and_then(|parent| fs::create_dir_all(parent).ok())?;
                fs::copy(source, &target)
                    .ok()
                    .map(|_| target.to_string_lossy().into_owned())
            })
        })
        .await
        .map_err(|_| VersionsError::Worker)?;
        self.info.save(&VersionRecord {
            entity_id: key.into(),
            version_id: revision_id.into(),
            revision_number: revision
                .as_ref()
                .map(|entry| entry.revision_number)
                .unwrap_or(0),
            parent_revision_ids: revision.map(|entry| entry.parents).unwrap_or_default(),
            title: details.message().unwrap_or("Lore revision").into(),
            created_at: details.timestamp().and_then(parse_time).unwrap_or_else(now),
            size_bytes: fs::metadata(path)?.len(),
            content_type: mime_guess::from_path(path).first_raw().map(str::to_owned),
            thumbnail_path,
            is_current: true,
        })?;
        Ok(())
    }
}

fn validate_branch_name(name: &str) -> Result<&str, VersionsError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 240 || name.chars().any(char::is_control) {
        return Err(VersionsError::InvalidBranchName);
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "photag-lore-branches-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        ));
        if let Err(error) = fs::create_dir_all(&path) {
            panic!("could not create test directory: {error}");
        }
        path
    }

    #[tokio::test]
    async fn activating_older_version_keeps_newer_version_switchable()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = test_dir();
        let library_file = root.join("photo.jpg");
        fs::write(&library_file, b"base bytes\n")?;
        let info = Arc::new(VersionInfoService::open_at(
            root.join("version-info.sqlite3"),
        )?);
        let thumbs = Arc::new(ThumbnailService::open_at(root.join("thumbs"))?);
        let service = VersionsService {
            root: root.join("versions"),
            info,
            thumbs,
            locks: Arc::new(Mutex::new(HashMap::new())),
        };
        let path = library_file.to_string_lossy().into_owned();
        let base = service.create_version(&path, "Base").await?;
        fs::write(&library_file, b"newer bytes\n")?;
        let newer = service.create_version(&path, "Newer").await?;

        assert_eq!(service.list(&path).await?.len(), 2);
        service.activate(&path, &base.version_id).await?;
        assert_eq!(fs::read(&library_file)?, b"base bytes\n");

        let after_rewind = service.list(&path).await?;
        assert_eq!(after_rewind.len(), 2, "rewinding must retain newer version");
        assert_eq!(after_rewind[0].version_id, newer.version_id);
        assert_eq!(after_rewind[1].version_id, base.version_id);
        assert!(
            after_rewind
                .iter()
                .any(|entry| { entry.version_id == newer.version_id && !entry.is_current })
        );
        assert!(
            after_rewind
                .iter()
                .any(|entry| { entry.version_id == base.version_id && entry.is_current })
        );

        service.activate(&path, &newer.version_id).await?;
        assert_eq!(fs::read(&library_file)?, b"newer bytes\n");
        let after_forward = service.list(&path).await?;
        assert_eq!(after_forward.len(), 2);
        assert!(
            after_forward
                .iter()
                .any(|entry| { entry.version_id == newer.version_id && entry.is_current })
        );
        service.activate(&path, &base.version_id).await?;
        fs::write(&library_file, b"third edit based on base\n")?;
        let third = service.create_version(&path, "Version 03").await?;
        assert_eq!(service.list(&path).await?.len(), 3);
        assert_eq!(third.parent_revision_ids, vec![newer.version_id.clone()]);
        for (version, bytes) in [
            (&base, b"base bytes\n".as_slice()),
            (&newer, b"newer bytes\n".as_slice()),
            (&third, b"third edit based on base\n".as_slice()),
        ] {
            service.activate(&path, &version.version_id).await?;
            assert_eq!(fs::read(&library_file)?, bytes);
        }
        service.activate(&path, &base.version_id).await?;
        assert!(matches!(
            service.create_version(&path, "No edit").await,
            Err(VersionsError::UnsavedChanges)
        ));
        fs::write(&library_file, b"recover rejected fourth edit\n")?;
        // Reproduce a pre-fix failed commit, including its persisted staged
        // content. The next public create must repair that exact state.
        let (key, _) = VersionsService::key(&library_file)?;
        let (content, work, store) = service.paths(&key);
        {
            let api = service.open(&key, &work, &store).await?;
            fs::copy(&library_file, &content)?;
            stage::stage(
                &api,
                FileStageArgs {
                    paths: vec![content.to_string_lossy().into_owned()],
                    case_change: CaseChange::Error,
                    scan: true,
                },
            )
            .await?;
            let rejected = commit::commit(
                &api,
                CommitArgs {
                    message: "Old failed attempt".into(),
                },
            )
            .await;
            assert!(
                matches!(rejected, Err(ref error) if error.to_string().contains("Branch has been advanced"))
            );
            flush::flush(&api).await?;
        }
        let recovered = service.create_version(&path, "Recovered 04").await?;
        assert_eq!(
            recovered.parent_revision_ids,
            vec![third.version_id.clone()]
        );
        assert_eq!(fs::read(&library_file)?, b"recover rejected fourth edit\n");
        assert_eq!(service.list(&path).await?.len(), 4);
        let _ = fs::remove_dir_all(root);
        Ok(())
    }

    #[tokio::test]
    async fn branch_lifecycle_persists_and_resolves_conflict() {
        let root = test_dir();
        let library_file = root.join("photo.jpg");
        if let Err(error) = fs::write(&library_file, b"base\n") {
            panic!("could not create source file: {error}");
        }
        let info = match VersionInfoService::open_at(root.join("version-info.sqlite3")) {
            Ok(service) => Arc::new(service),
            Err(error) => panic!("could not open version metadata: {error}"),
        };
        let thumbs = match ThumbnailService::open_at(root.join("thumbs")) {
            Ok(service) => Arc::new(service),
            Err(error) => panic!("could not open thumbnail cache: {error}"),
        };
        let service = VersionsService {
            root: root.join("versions"),
            info,
            thumbs,
            locks: Arc::new(Mutex::new(HashMap::new())),
        };
        let path = library_file.to_string_lossy().into_owned();

        service
            .create_version(&path, "Base")
            .await
            .expect("base version");
        let base_graph = service
            .branch_graph(&path, false)
            .await
            .expect("read initial branch");
        let base_branch = match base_graph.branches.iter().find(|branch| branch.is_current) {
            Some(branch) => branch.name.clone(),
            None => panic!("Lore did not report a current branch"),
        };
        service
            .create_branch(&path, "feature/conflict")
            .await
            .expect("create feature branch");
        service
            .switch_branch(&path, "feature/conflict", false)
            .await
            .expect("switch to feature branch");
        fs::write(&library_file, b"base\nfeature\n").expect("feature edit");
        service
            .create_version(&path, "Feature edit")
            .await
            .expect("feature version");
        service
            .switch_branch(&path, &base_branch, false)
            .await
            .expect("switch to main branch");
        fs::write(&library_file, b"base\nmain\n").expect("main edit");
        service
            .create_version(&path, "Main edit")
            .await
            .expect("main version");

        let merge = service
            .merge_branch(&path, "feature/conflict")
            .await
            .expect("start branch merge");
        assert!(
            merge.has_conflicts,
            "divergent single-file edits should conflict"
        );
        let pending = service
            .branch_graph(&path, false)
            .await
            .expect("read persisted merge state");
        assert!(pending.pending_merge.is_some());

        let resolved = service
            .resolve_merge(&path, "theirs")
            .await
            .expect("resolve using incoming branch");
        assert!(resolved.completed);
        assert_eq!(
            fs::read(&library_file).expect("restored merged file"),
            b"base\nfeature\n"
        );
        service
            .archive_branch(&path, "feature/conflict")
            .await
            .expect("archive non-current branch");
        let graph = service
            .branch_graph(&path, true)
            .await
            .expect("include archived branch in graph");
        assert!(
            graph
                .branches
                .iter()
                .any(|branch| branch.name == "feature/conflict" && branch.is_archived)
        );
        let _ = fs::remove_dir_all(root);
    }
}
