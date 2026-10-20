//! Keep immutable file snapshots linear when saving after a historical checkout.
use super::*;
use lore_vm::ops::{
    branch::list::{self, BranchListArgs},
    file::unstage::{self, FileUnstageArgs},
    repository::status::RepositoryStatusResult,
};

impl VersionsService {
    pub(super) async fn prepare_snapshot_commit(
        &self,
        api: &LoreApi,
        content: &Path,
        before: &RepositoryStatusResult,
    ) -> Result<(), VersionsError> {
        if before.files.iter().any(|file| file.conflict)
            || before
                .revision
                .as_ref()
                .is_some_and(|revision| !revision.revision_merged.is_empty())
        {
            return Err(VersionsError::PendingMerge);
        }
        let branches = list::list(api, BranchListArgs { archived: false }).await?;
        let target = before.revision.as_ref().and_then(|revision| {
            branches
                .entries
                .iter()
                .find(|branch| {
                    branch.is_current
                        && branch.latest.bytes().any(|byte| byte != b'0')
                        && branch.latest != revision.revision
                })
                .map(|branch| branch.latest.clone())
        });
        let staged = before.files.iter().any(|file| file.staged);
        let Some(target) = target else {
            return if staged {
                Err(VersionsError::UnsavedChanges)
            } else {
                Ok(())
            };
        };
        // The only staged payload owned by this service is its private content
        // snapshot. Recover a rejected historical commit without touching the
        // library source, unrelated staged paths, or a pending merge.
        if before
            .files
            .iter()
            .any(|file| file.path.replace('\\', "/") != "content")
        {
            return Err(VersionsError::UnsavedChanges);
        }
        if staged {
            unstage::unstage(
                api,
                FileUnstageArgs {
                    paths: vec![content.to_string_lossy().into_owned()],
                },
            )
            .await?;
        }
        // Lore's shared branch tip remains at the latest commit after an older
        // revision is checked out. Refresh that base before staging the new
        // snapshot; otherwise branch-tip compare-and-swap rejects the commit.
        // reset affects only this private worktree. create_version copies the
        // untouched source file into it after this synchronization completes.
        sync::sync(
            api,
            RevisionSyncArgs {
                revision: target,
                reset: true,
                ..Default::default()
            },
        )
        .await?;
        flush::flush(api).await?;
        Ok(())
    }
}
