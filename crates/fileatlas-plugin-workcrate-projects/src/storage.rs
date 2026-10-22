use super::*;

pub(super) fn validate_index(index: &ProjectIndex) -> Result<(), WorkCrateProjectsError> {
    let mut ids = std::collections::HashSet::new();
    for project in &index.projects {
        if Uuid::parse_str(&project.id).is_err()
            || !ids.insert(project.id.as_str())
            || project.name.trim().is_empty()
        {
            return Err(WorkCrateProjectsError::Invalid(
                "project index contains an invalid or duplicate entry",
            ));
        }
    }
    if index
        .locations
        .iter()
        .any(|(id, path)| !ids.contains(id.as_str()) || !path.is_absolute())
    {
        return Err(WorkCrateProjectsError::Invalid(
            "project locator is invalid",
        ));
    }
    Ok(())
}
pub(super) fn validate_name(name: &str) -> Result<String, WorkCrateProjectsError> {
    let name = name.trim();
    if name.is_empty()
        || name.chars().count() > 120
        || name.chars().any(|c| matches!(c, '\\' | '/' | '\0'))
    {
        return Err(WorkCrateProjectsError::Invalid(
            "project name must be 1–120 characters and cannot contain a path separator",
        ));
    }
    Ok(name.to_owned())
}
pub(super) fn ensure_unique_name(
    index: &ProjectIndex,
    name: &str,
    except: Option<&str>,
) -> Result<(), WorkCrateProjectsError> {
    if index.projects.iter().any(|project| {
        Some(project.id.as_str()) != except && project.name.eq_ignore_ascii_case(name)
    }) {
        return Err(WorkCrateProjectsError::DuplicateName(name.to_owned()));
    }
    Ok(())
}
pub(super) fn read_json<T: for<'de> Deserialize<'de>>(
    path: &Path,
) -> Result<T, WorkCrateProjectsError> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

pub(super) fn read_json_with_backup<T: for<'de> Deserialize<'de>>(
    path: &Path,
) -> Result<(T, bool), WorkCrateProjectsError> {
    let backup = path.with_extension("json.bak");
    match read_json(path) {
        Ok(value) => Ok((value, false)),
        Err(_primary) if backup.is_file() => Ok((read_json(&backup)?, true)),
        Err(error) => Err(error),
    }
}

pub(super) fn copy_hash_atomic(
    source: &Path,
    target: &Path,
) -> Result<String, WorkCrateProjectsError> {
    let parent = target.parent().ok_or(WorkCrateProjectsError::Invalid(
        "preview file has no parent directory",
    ))?;
    let mut input = std::fs::File::open(source)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        temp.write_all(&buffer[..count])?;
        hash.update(&buffer[..count]);
    }
    temp.as_file().sync_all()?;
    temp.persist(target)
        .map_err(|error| WorkCrateProjectsError::Io(error.error))?;
    Ok(format!("{:x}", hash.finalize()))
}
pub(super) fn atomic_write_json(
    path: &Path,
    value: &impl Serialize,
) -> Result<(), WorkCrateProjectsError> {
    let parent = path.parent().ok_or(WorkCrateProjectsError::Invalid(
        "data file has no parent directory",
    ))?;
    std::fs::create_dir_all(parent)?;
    let bytes = serde_json::to_vec_pretty(value)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(&bytes)?;
    temp.as_file().sync_all()?;
    if !path.exists() {
        temp.persist(path)
            .map_err(|error| WorkCrateProjectsError::Io(error.error))?;
        return Ok(());
    }
    let backup = path.with_extension("json.bak");
    if backup.exists() {
        std::fs::remove_file(&backup)?;
    }
    std::fs::rename(path, &backup)?;
    match temp.persist(path) {
        Ok(_) => {
            let _ = std::fs::remove_file(backup);
            Ok(())
        }
        Err(error) => {
            let _ = std::fs::rename(&backup, path);
            Err(error.error.into())
        }
    }
}
pub(super) fn same_path(a: &str, b: &str) -> bool {
    a.replace('/', "\\")
        .trim_end_matches('\\')
        .eq_ignore_ascii_case(b.replace('/', "\\").trim_end_matches('\\'))
}
pub(super) fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_secs())
        .unwrap_or_default()
}
pub(super) fn modified_at(metadata: &std::fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|v| v.as_secs())
}
