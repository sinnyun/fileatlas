//! Everything IPC adapter. Search can prepare an installed background client.

#[cfg(windows)]
mod client;

use cordis::{Context, Plugin, Service};
#[cfg(windows)]
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct EverythingInstallation {
    pub state: &'static str,
    pub version: Option<String>,
    pub path: Option<std::path::PathBuf>,
}

#[derive(Default)]
struct Readiness {
    active: AtomicBool,
    stopped: AtomicBool,
    revision: AtomicU64,
}

#[derive(Default)]
pub struct EverythingSearchService {
    query_lock: Mutex<Option<std::time::Instant>>,
    readiness: Arc<Readiness>,
}

impl Drop for EverythingSearchService {
    fn drop(&mut self) {
        self.readiness.stopped.store(true, Ordering::Release);
    }
}

impl Service for EverythingSearchService {
    const NAME: &'static str = "fileatlas/everything-search";
}

impl EverythingSearchService {
    /// Passive IO: request through a ReadModel; never call from GPUI render.
    pub fn inspect_installation(&self) -> EverythingInstallation {
        #[cfg(windows)]
        {
            client::inspect()
        }
        #[cfg(not(windows))]
        {
            EverythingInstallation {
                state: "unsupported",
                version: None,
                path: None,
            }
        }
    }

    pub fn revision(&self) -> u64 {
        self.readiness.revision.load(Ordering::Acquire)
    }

    pub fn index_loading(&self) -> bool {
        self.readiness.active.load(Ordering::Acquire)
    }

    #[cfg(windows)]
    fn watch_readiness(&self) {
        if self.readiness.active.swap(true, Ordering::AcqRel) {
            return;
        }
        let state = Arc::clone(&self.readiness);
        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            while !state.stopped.load(Ordering::Acquire) && started.elapsed().as_secs() < 300 {
                if windows::index_ready() {
                    state.active.store(false, Ordering::Release);
                    state.revision.fetch_add(1, Ordering::Release);
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            state.active.store(false, Ordering::Release);
        });
    }
    /// Run the Everything query off the GPUI thread. `roots` restrict the IPC
    /// search to FileAtlas's registered library folders before returning paths.
    pub fn search_paths(
        &self,
        query: &str,
        roots: &[String],
    ) -> Result<Vec<String>, EverythingSearchError> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        let mut guard = self
            .query_lock
            .lock()
            .map_err(|_| EverythingSearchError::Poisoned)?;
        #[cfg(windows)]
        {
            let mut paths = Vec::new();
            let mut seen = HashSet::new();
            let global = [String::new()];
            let roots = if roots.is_empty() { &global[..] } else { roots };
            for root in roots {
                let scoped = scoped_query(query, root);
                let result = windows::search(&scoped, &mut guard);
                if matches!(result, Err(EverythingSearchError::Loading)) {
                    self.watch_readiness();
                }
                for path in result? {
                    if seen.insert(path.to_lowercase()) {
                        paths.push(path);
                    }
                }
                if paths.len() > 250_000 {
                    return Err(EverythingSearchError::TooManyResults);
                }
            }
            Ok(paths)
        }
        #[cfg(not(windows))]
        {
            let _ = (query, roots);
            Err(EverythingSearchError::UnsupportedPlatform)
        }
    }
}

fn scoped_query(query: &str, root: &str) -> String {
    let root = root
        .strip_prefix(r"\\?\UNC\")
        .map(|tail| format!(r"\\{tail}"))
        .unwrap_or_else(|| root.strip_prefix(r"\\?\").unwrap_or(root).to_owned());
    let root = root.replace('/', "\\");
    let root = root.trim_end_matches('\\');
    // Match the same literal substring/LIKE wildcards as FileAtlas's SQL. Passing
    // raw text to Everything would interpret valid names such as !draft as
    // operators. Regex mode bypasses Everything's expression parser entirely.
    if root.is_empty() {
        return regex_text(query.trim(), true);
    }
    format!(
        r"^(?={}\\).*{}",
        regex_text(root, false),
        regex_text(query.trim(), true)
    )
}

fn regex_text(text: &str, like: bool) -> String {
    let mut pattern = String::new();
    for character in text.chars() {
        match character {
            '%' if like => pattern.push_str(".*"),
            '_' if like => pattern.push('.'),
            '\\' | '.' | '^' | '$' | '|' | '?' | '*' | '+' | '(' | ')' | '[' | ']' | '{' | '}' => {
                pattern.push('\\');
                pattern.push(character);
            }
            _ => pattern.push(character),
        }
    }
    pattern
}

#[derive(Debug, thiserror::Error)]
pub enum EverythingSearchError {
    #[error("Everything search is unavailable. Open Everything and wait for its index to load.")]
    Unavailable,
    #[error("Everything returned more than the supported search result limit.")]
    TooManyResults,
    #[error("Everything search is supported only on Windows.")]
    UnsupportedPlatform,
    #[error("Everything SDK query lock is poisoned.")]
    Poisoned,
    #[error("Everything SDK IPC query failed (error code 0x{0:08X}).")]
    Ipc(u32),
    #[error("Everything is not installed. Install it from voidtools.com to enable indexed search.")]
    NotInstalled,
    #[error("Could not start the installed Everything background client: {0}")]
    Launch(std::io::Error),
    #[error("Everything is still loading its index. Retry the search shortly.")]
    Loading,
    #[error("Everything {0} is incompatible. Install Everything 1.5 or later.")]
    IncompatibleVersion(String),
}

pub struct EverythingSearchPlugin;

impl Plugin for EverythingSearchPlugin {
    type Config = ();
    type Input = ();
    type PrepareError = std::convert::Infallible;
    type ApplyError = std::io::Error;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        "fileatlas.everything-search".into()
    }

    fn prepare(&self, _: ()) -> Result<(), Self::PrepareError> {
        Ok(())
    }

    async fn apply(&self, context: Context, _: &()) -> Result<(), Self::ApplyError> {
        let _publication = context
            .provide(Arc::new(EverythingSearchService::default()))
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        Ok(())
    }
}

#[cfg(windows)]
mod windows {
    mod transport;
    use super::EverythingSearchError;
    use std::ffi::c_char;
    const FALSE: i32 = 0;
    const TRUE: i32 = 1;
    const MAX_RESULTS: usize = 250_000;

    #[repr(C)]
    struct Client {
        _opaque: [u8; 0],
    }
    #[repr(C)]
    struct SearchState {
        _opaque: [u8; 0],
    }
    #[repr(C)]
    struct ResultList {
        _opaque: [u8; 0],
    }

    #[link(name = "Everything3_x64", kind = "static")]
    unsafe extern "system" {
        fn Everything3_ConnectUTF8(instance_name: *const c_char) -> *mut Client;
        fn Everything3_DestroyClient(client: *mut Client) -> i32;
        fn Everything3_ShutdownClient(client: *mut Client) -> i32;
        fn Everything3_GetLastError() -> u32;
        fn Everything3_IsDBLoaded(client: *mut Client) -> i32;
        fn Everything3_CreateSearchState() -> *mut SearchState;
        fn Everything3_DestroySearchState(search_state: *mut SearchState) -> i32;
        fn Everything3_SetSearchMatchPath(search_state: *mut SearchState, enable: i32) -> i32;
        fn Everything3_SetSearchRegex(search_state: *mut SearchState, enable: i32) -> i32;
        fn Everything3_SetSearchTextUTF8(
            search_state: *mut SearchState,
            search: *const c_char,
        ) -> i32;
        fn Everything3_SetSearchViewportOffset(
            search_state: *mut SearchState,
            offset: usize,
        ) -> i32;
        fn Everything3_SetSearchViewportCount(search_state: *mut SearchState, count: usize) -> i32;
        fn Everything3_GetSearchViewportCount(search_state: *mut SearchState) -> usize;
        fn Everything3_GetSearchViewportOffset(search_state: *mut SearchState) -> usize;
        fn Everything3_Search(
            client: *mut Client,
            search_state: *mut SearchState,
        ) -> *mut ResultList;
        fn Everything3_DestroyResultList(result_list: *mut ResultList) -> i32;
        fn Everything3_GetResultListViewportCount(result_list: *const ResultList) -> usize;
        fn Everything3_IsFolderResult(result_list: *const ResultList, index: usize) -> i32;
        fn Everything3_GetResultFullPathNameUTF8(
            result_list: *const ResultList,
            index: usize,
            buffer: *mut c_char,
            buffer_size: usize,
        ) -> usize;
    }

    pub(super) fn index_ready() -> bool {
        unsafe {
            let mut client = transport::connect(false);
            if client.is_null() {
                client = transport::connect(true);
            }
            if client.is_null() {
                return false;
            }
            let _lease = transport::ClientLease::new(client, 2);
            Everything3_IsDBLoaded(client) != FALSE
        }
    }

    pub(super) fn search(
        query: &str,
        last_start: &mut Option<std::time::Instant>,
    ) -> Result<Vec<String>, EverythingSearchError> {
        if query.as_bytes().contains(&0) {
            return Err(EverythingSearchError::Unavailable);
        }
        let mut query_utf8 = query.as_bytes().to_vec();
        query_utf8.push(0);
        let mut path_buffer = vec![0u8; 32_768];
        unsafe {
            // Everything 1.5 commonly runs as the named "1.5a" instance so
            // it can coexist with 1.4. Try that official default first, then
            // the unnamed instance for installations configured without it.
            let mut client = transport::connect(true);
            if client.is_null() {
                client = transport::connect(false);
            }
            if client.is_null()
                && Everything3_GetLastError() == 0xE000_0002
                && last_start.is_none_or(|time| time.elapsed().as_secs() >= 30)
            {
                *last_start = Some(std::time::Instant::now());
                super::client::start_background()?;
                let started = std::time::Instant::now();
                while client.is_null() && started.elapsed().as_secs() < 10 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    client = transport::connect(false);
                    if client.is_null() {
                        client = transport::connect(true);
                    }
                }
            }
            if client.is_null() {
                let error = Everything3_GetLastError();
                return Err(if matches!(error, 121 | 231) {
                    EverythingSearchError::Loading
                } else {
                    EverythingSearchError::Ipc(error)
                });
            }
            let lease = transport::ClientLease::new(client, 15);
            if Everything3_IsDBLoaded(client) == FALSE {
                let error = Everything3_GetLastError();
                return Err(if error == 0 || lease.timed_out() {
                    EverythingSearchError::Loading
                } else {
                    EverythingSearchError::Ipc(error)
                });
            }
            let search_state = Everything3_CreateSearchState();
            if search_state.is_null() {
                let error = Everything3_GetLastError();
                return Err(EverythingSearchError::Ipc(error));
            }
            // Root terms in the Everything expression must match paths. FileAtlas
            // subsequently applies its enabled filename/path scopes in SQL.
            // SDK 3.0.1.12's viewport setters update the state but always
            // return FALSE (see upstream Everything3.c). Verify the stored
            // values through getters instead of rejecting successful setup.
            Everything3_SetSearchViewportOffset(search_state, 0);
            Everything3_SetSearchViewportCount(search_state, MAX_RESULTS + 1);
            let configured = Everything3_SetSearchMatchPath(search_state, TRUE) != FALSE
                && Everything3_SetSearchRegex(search_state, TRUE) != FALSE
                && Everything3_SetSearchTextUTF8(search_state, query_utf8.as_ptr().cast()) != FALSE
                && Everything3_GetSearchViewportOffset(search_state) == 0
                && Everything3_GetSearchViewportCount(search_state) == MAX_RESULTS + 1;
            if !configured {
                let error = Everything3_GetLastError();
                Everything3_DestroySearchState(search_state);
                return Err(EverythingSearchError::Ipc(error));
            }
            let result_list = Everything3_Search(client, search_state);
            if result_list.is_null() {
                let error = Everything3_GetLastError();
                Everything3_DestroySearchState(search_state);
                return Err(EverythingSearchError::Ipc(error));
            }
            let count = Everything3_GetResultListViewportCount(result_list);
            if count > MAX_RESULTS {
                Everything3_DestroyResultList(result_list);
                Everything3_DestroySearchState(search_state);
                return Err(EverythingSearchError::TooManyResults);
            }
            let mut paths = Vec::with_capacity(count);
            let mut invalid_path = false;
            for index in 0..count {
                if Everything3_IsFolderResult(result_list, index) != FALSE {
                    continue;
                }
                // Zero-sized SDK reads return the required UTF-8 byte count,
                // including NUL. Long Unicode paths can exceed 32 KiB.
                let required = Everything3_GetResultFullPathNameUTF8(
                    result_list,
                    index,
                    std::ptr::null_mut(),
                    0,
                );
                if !(2..=131_073).contains(&required) {
                    invalid_path = true;
                    break;
                }
                if required > path_buffer.len() {
                    path_buffer.resize(required, 0);
                }
                let length = Everything3_GetResultFullPathNameUTF8(
                    result_list,
                    index,
                    path_buffer.as_mut_ptr().cast(),
                    path_buffer.len(),
                );
                if length + 1 != required {
                    invalid_path = true;
                    break;
                }
                paths.push(String::from_utf8_lossy(&path_buffer[..length]).into_owned());
            }
            Everything3_DestroyResultList(result_list);
            Everything3_DestroySearchState(search_state);
            if invalid_path {
                Err(EverythingSearchError::Unavailable)
            } else {
                Ok(paths)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::scoped_query;

    #[test]
    fn scopes_everything_query_to_one_registered_root() {
        assert_eq!(
            scoped_query("!holiday [1].png", r"\\?\C:\Photo Library"),
            r"^(?=C:\\Photo Library\\).*!holiday \[1\]\.png"
        );
    }

    #[test]
    fn source_like_wildcards_do_not_escape_the_root_scope() {
        assert_eq!(
            scoped_query("trip_%.jpg", r"C:\root_name"),
            r"^(?=C:\\root_name\\).*trip..*\.jpg"
        );
    }
}
