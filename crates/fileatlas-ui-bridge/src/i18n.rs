//! Public translation boundary. The i18n plugin owns resources and persistence.
use std::sync::{
    OnceLock, RwLock, Weak,
    atomic::{AtomicU64, Ordering},
};

pub trait Translator: Send + Sync {
    fn translate(&self, source: &str, args: &[(&str, String)]) -> String;
}

static TRANSLATOR: OnceLock<RwLock<Option<Weak<dyn Translator>>>> = OnceLock::new();
static REVISION: AtomicU64 = AtomicU64::new(0);

pub fn install(translator: Option<Weak<dyn Translator>>) {
    *TRANSLATOR
        .get_or_init(Default::default)
        .write()
        .expect("translation lock") = translator;
    changed();
}

pub fn changed() {
    REVISION.fetch_add(1, Ordering::Release);
}
pub fn revision() -> u64 {
    REVISION.load(Ordering::Acquire)
}

pub fn tr(source: &str) -> String {
    tr_args(source, &[])
}

pub fn tr_args(source: &str, args: &[(&str, String)]) -> String {
    TRANSLATOR
        .get_or_init(Default::default)
        .read()
        .ok()
        .and_then(|translator| translator.as_ref().and_then(Weak::upgrade))
        .map(|translator| translator.translate(source, args))
        .unwrap_or_else(|| source.to_owned())
}
