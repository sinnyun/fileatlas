//! OS-level drag-and-drop: drag files out of PhoTag to external
//! destinations (desktop, folders, Premiere Pro, etc.).
//!
//! Uses raw Win32 COM APIs to build a CF_HDROP data object and call
//! DoDragDrop on the current (UI) thread.

use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::{AtomicU32, Ordering};

// ── Win32 constants ──────────────────────────────────────────────────
const CF_HDROP: u16 = 15;
const GMEM_MOVEABLE: u32 = 0x0002;
const GMEM_ZEROINIT: u32 = 0x0040;
const TYMED_HGLOBAL: u32 = 1;
const DVASPECT_CONTENT: u32 = 1;
const DROPEFFECT_NONE: u32 = 0;
const DROPEFFECT_COPY: u32 = 1;
const MK_LBUTTON: u32 = 0x0001;
const S_OK: i32 = 0;
const S_FALSE: i32 = 1;
const E_NOTIMPL: i32 = 0x80004001_u32 as i32;
const E_NOINTERFACE: i32 = 0x80004002_u32 as i32;
const E_OUTOFMEMORY: i32 = 0x8007000E_u32 as i32;
const DV_E_FORMATETC: i32 = 0x80040064_u32 as i32;
const DRAGDROP_S_DROP: i32 = 0x00040100;
const DRAGDROP_S_CANCEL: i32 = 0x00040101;
const DRAGDROP_S_USEDEFAULTCURSORS: i32 = 0x00040102;
const DATA_S_SAMEFORMATETC: i32 = 0x00040130;

// ── Win32 types ──────────────────────────────────────────────────────

#[repr(C)]
struct GUID { d1: u32, d2: u16, d3: u16, d4: [u8; 8] }
impl GUID {
    const fn eq(&self, o: &GUID) -> bool {
        self.d1 == o.d1 && self.d2 == o.d2 && self.d3 == o.d3
            && self.d4[0] == o.d4[0] && self.d4[1] == o.d4[1]
            && self.d4[2] == o.d4[2] && self.d4[3] == o.d4[3]
            && self.d4[4] == o.d4[4] && self.d4[5] == o.d4[5]
            && self.d4[6] == o.d4[6] && self.d4[7] == o.d4[7]
    }
}

const IID_IUNKNOWN: GUID        = GUID { d1: 0x00000000, d2: 0, d3: 0, d4: [0xC0,0,0,0,0,0,0,0x46] };
const IID_IDROPSOURCE: GUID     = GUID { d1: 0x00000121, d2: 0, d3: 0, d4: [0xC0,0,0,0,0,0,0,0x46] };
const IID_IDATAOBJECT: GUID     = GUID { d1: 0x0000010E, d2: 0, d3: 0, d4: [0xC0,0,0,0,0,0,0,0x46] };
const IID_IENUMFORMATETC: GUID  = GUID { d1: 0x00000103, d2: 0, d3: 0, d4: [0xC0,0,0,0,0,0,0,0x46] };

type PVOID = *mut std::ffi::c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct FORMATETC { cf: u16, ptd: PVOID, aspect: u32, lindex: i32, tymed: u32 }
unsafe impl Sync for FORMATETC {}
#[repr(C)]
struct STGMEDIUM { tymed: u32, handle: PVOID, punk: PVOID }
#[repr(C)]
struct DROPFILES { p_files: u32, pt_x: i32, pt_y: i32, f_nc: i32, f_wide: i32 }

// Vtable arrays of function pointers stored as PVOID need Sync for statics.
// They are immutable function pointers — safe to share.
struct VtblSync<const N: usize>([PVOID; N]);
unsafe impl<const N: usize> Sync for VtblSync<N> {}

extern "system" {
    fn GlobalAlloc(f: u32, n: usize) -> PVOID;
    fn GlobalLock(h: PVOID) -> PVOID;
    fn GlobalUnlock(h: PVOID) -> i32;
    fn GlobalFree(h: PVOID) -> PVOID;
    fn GlobalSize(h: PVOID) -> usize;
    fn DoDragDrop(obj: PVOID, src: PVOID, ok: u32, eff: *mut u32) -> i32;
    fn OleInitialize(r: PVOID) -> i32;
}

// SAFETY: FORMATETC contains a null pointer (ptd), never dereferenced.
unsafe impl Send for FORMATETC {}

fn hdrop_fmt() -> FORMATETC {
    FORMATETC { cf: CF_HDROP, ptd: std::ptr::null_mut(), aspect: DVASPECT_CONTENT, lindex: -1, tymed: TYMED_HGLOBAL }
}

// ── IDropSource ──────────────────────────────────────────────────────

#[repr(C)]
struct DropSource {
    vtbl: &'static [PVOID; 5],
    rc: AtomicU32,
}

unsafe extern "system" fn ds_qi(this: PVOID, riid: *const GUID, ppv: *mut PVOID) -> i32 {
    let r = &*riid;
    if r.eq(&IID_IUNKNOWN) || r.eq(&IID_IDROPSOURCE) {
        ds_addref(this); *ppv = this; S_OK
    } else { *ppv = std::ptr::null_mut(); E_NOINTERFACE }
}
unsafe extern "system" fn ds_addref(this: PVOID) -> u32 {
    (*(this as *mut DropSource)).rc.fetch_add(1, Ordering::Relaxed) + 1
}
unsafe extern "system" fn ds_release(this: PVOID) -> u32 {
    let obj = this as *mut DropSource;
    let prev = (*obj).rc.fetch_sub(1, Ordering::Relaxed);
    if prev == 1 { let _ = Box::from_raw(obj); 0 } else { prev - 1 }
}
unsafe extern "system" fn ds_query_continue(
    _: PVOID, esc: i32, keys: u32,
) -> i32 {
    if esc != 0 { DRAGDROP_S_CANCEL }
    else if (keys & MK_LBUTTON) == 0 { DRAGDROP_S_DROP }
    else { S_OK }
}
unsafe extern "system" fn ds_give_feedback(_: PVOID, _: u32) -> i32 {
    DRAGDROP_S_USEDEFAULTCURSORS
}

static DS_VTBL: VtblSync<5> = VtblSync([
    ds_qi as PVOID,
    ds_addref as PVOID,
    ds_release as PVOID,
    ds_query_continue as PVOID,
    ds_give_feedback as PVOID,
]);

fn new_drop_source() -> PVOID {
    Box::into_raw(Box::new(DropSource {
        vtbl: &DS_VTBL.0,
        rc: AtomicU32::new(1),
    })) as PVOID
}

// ── IEnumFORMATETC ───────────────────────────────────────────────────

#[repr(C)]
struct EnumFmt {
    vtbl: &'static [PVOID; 7],
    rc: AtomicU32,
    idx: AtomicU32,
}

unsafe extern "system" fn ef_qi(this: PVOID, riid: *const GUID, ppv: *mut PVOID) -> i32 {
    let r = &*riid;
    if r.eq(&IID_IUNKNOWN) || r.eq(&IID_IENUMFORMATETC) {
        ef_addref(this); *ppv = this; S_OK
    } else { *ppv = std::ptr::null_mut(); E_NOINTERFACE }
}
unsafe extern "system" fn ef_addref(this: PVOID) -> u32 {
    (*(this as *mut EnumFmt)).rc.fetch_add(1, Ordering::Relaxed) + 1
}
unsafe extern "system" fn ef_release(this: PVOID) -> u32 {
    let obj = this as *mut EnumFmt;
    let prev = (*obj).rc.fetch_sub(1, Ordering::Relaxed);
    if prev == 1 { let _ = Box::from_raw(obj); 0 } else { prev - 1 }
}
unsafe extern "system" fn ef_next(
    this: PVOID, celt: u32, rgelt: *mut FORMATETC, fetched: *mut u32,
) -> i32 {
    let obj = this as *mut EnumFmt;
    let i = (*obj).idx.load(Ordering::Relaxed);
    let mut n = 0u32;
    if i == 0 && celt >= 1 {
        *rgelt = hdrop_fmt();
        n = 1;
        (*obj).idx.store(1, Ordering::Relaxed);
    }
    if !fetched.is_null() { *fetched = n; }
    if n == celt { S_OK } else { S_FALSE }
}
unsafe extern "system" fn ef_skip(this: PVOID, celt: u32) -> i32 {
    let obj = this as *mut EnumFmt;
    let i = (*obj).idx.fetch_add(celt, Ordering::Relaxed);
    if i + celt <= 1 { S_OK } else { S_FALSE }
}
unsafe extern "system" fn ef_reset(this: PVOID) -> i32 {
    (*(this as *mut EnumFmt)).idx.store(0, Ordering::Relaxed); S_OK
}
unsafe extern "system" fn ef_clone(_: PVOID, out: *mut PVOID) -> i32 {
    *out = new_enum_fmt(); S_OK
}

static EF_VTBL: VtblSync<7> = VtblSync([
    ef_qi as PVOID, ef_addref as PVOID, ef_release as PVOID,
    ef_next as PVOID, ef_skip as PVOID, ef_reset as PVOID, ef_clone as PVOID,
]);

fn new_enum_fmt() -> PVOID {
    Box::into_raw(Box::new(EnumFmt {
        vtbl: &EF_VTBL.0,
        rc: AtomicU32::new(1),
        idx: AtomicU32::new(0),
    })) as PVOID
}

// ── IDataObject ──────────────────────────────────────────────────────

#[repr(C)]
struct DataObj {
    vtbl: &'static [PVOID; 12],
    rc: AtomicU32,
    hglobal: PVOID,
}

unsafe extern "system" fn do_qi(this: PVOID, riid: *const GUID, ppv: *mut PVOID) -> i32 {
    let r = &*riid;
    if r.eq(&IID_IUNKNOWN) || r.eq(&IID_IDATAOBJECT) {
        do_addref(this); *ppv = this; S_OK
    } else { *ppv = std::ptr::null_mut(); E_NOINTERFACE }
}
unsafe extern "system" fn do_addref(this: PVOID) -> u32 {
    (*(this as *mut DataObj)).rc.fetch_add(1, Ordering::Relaxed) + 1
}
unsafe extern "system" fn do_release(this: PVOID) -> u32 {
    let obj = this as *mut DataObj;
    let prev = (*obj).rc.fetch_sub(1, Ordering::Relaxed);
    if prev == 1 {
        let o = Box::from_raw(obj);
        if !o.hglobal.is_null() { GlobalFree(o.hglobal); }
        0
    } else { prev - 1 }
}

unsafe extern "system" fn do_get_data(
    this: PVOID, pf: *const FORMATETC, pm: *mut STGMEDIUM,
) -> i32 {
    let f = &*pf;
    if f.cf != CF_HDROP || (f.tymed & TYMED_HGLOBAL) == 0 { return DV_E_FORMATETC; }
    let src = (*(this as *mut DataObj)).hglobal;
    if src.is_null() { return E_OUTOFMEMORY; }
    let sz = GlobalSize(src);
    let copy = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, sz);
    if copy.is_null() { return E_OUTOFMEMORY; }
    let sp = GlobalLock(src);
    let dp = GlobalLock(copy);
    if !sp.is_null() && !dp.is_null() {
        std::ptr::copy_nonoverlapping(sp as *const u8, dp as *mut u8, sz);
    }
    GlobalUnlock(src);
    GlobalUnlock(copy);
    (*pm).tymed = TYMED_HGLOBAL;
    (*pm).handle = copy;
    (*pm).punk = std::ptr::null_mut();
    S_OK
}

unsafe extern "system" fn do_query(
    _: PVOID, pf: *const FORMATETC,
) -> i32 {
    let f = &*pf;
    if f.cf == CF_HDROP && (f.tymed & TYMED_HGLOBAL) != 0 { S_OK } else { DV_E_FORMATETC }
}

unsafe extern "system" fn do_enum(
    _: PVOID, dir: u32, out: *mut PVOID,
) -> i32 {
    if dir == 1 { *out = new_enum_fmt(); S_OK } else { E_NOTIMPL }
}

unsafe extern "system" fn stub2(_: PVOID, _: *const FORMATETC, _: *mut STGMEDIUM) -> i32 { E_NOTIMPL }
unsafe extern "system" fn stub_canonical(_: PVOID, _: *const FORMATETC, _: *mut FORMATETC) -> i32 { DATA_S_SAMEFORMATETC }
unsafe extern "system" fn stub_set(_: PVOID, _: *const FORMATETC, _: *mut STGMEDIUM, _: i32) -> i32 { E_NOTIMPL }
unsafe extern "system" fn stub_advise(_: PVOID, _: *const FORMATETC, _: u32, _: PVOID, _: *mut u32) -> i32 { E_NOTIMPL }
unsafe extern "system" fn stub_unadvise(_: PVOID, _: u32) -> i32 { E_NOTIMPL }
unsafe extern "system" fn stub_enumadvise(_: PVOID, _: *mut PVOID) -> i32 { E_NOTIMPL }

static DO_VTBL: VtblSync<12> = VtblSync([
    do_qi as PVOID,
    do_addref as PVOID,
    do_release as PVOID,
    do_get_data as PVOID,
    stub2 as PVOID,          // GetDataHere
    do_query as PVOID,       // QueryGetData
    stub_canonical as PVOID, // GetCanonicalFormatEtc
    stub_set as PVOID,       // SetData
    do_enum as PVOID,        // EnumFormatEtc
    stub_advise as PVOID,    // DAdvise
    stub_unadvise as PVOID,  // DUnadvise
    stub_enumadvise as PVOID,// EnumDAdvise
]);

fn build_hdrop(paths: &[String]) -> PVOID {
    let mut wide: Vec<u16> = Vec::new();
    for p in paths {
        wide.extend(std::ffi::OsStr::new(p).encode_wide());
        wide.push(0);
    }
    wide.push(0);

    let hdr = std::mem::size_of::<DROPFILES>();
    let total = hdr + wide.len() * 2;

    unsafe {
        let hg = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, total);
        if hg.is_null() { return hg; }
        let p = GlobalLock(hg);
        if p.is_null() { GlobalFree(hg); return std::ptr::null_mut(); }

        let df = p as *mut DROPFILES;
        (*df).p_files = hdr as u32;
        (*df).f_wide = 1;

        let dst = (p as *mut u8).add(hdr) as *mut u16;
        std::ptr::copy_nonoverlapping(wide.as_ptr(), dst, wide.len());

        GlobalUnlock(hg);
        hg
    }
}

fn new_data_object(paths: &[String]) -> PVOID {
    Box::into_raw(Box::new(DataObj {
        vtbl: &DO_VTBL.0,
        rc: AtomicU32::new(1),
        hglobal: build_hdrop(paths),
    })) as PVOID
}

// ── Public API ───────────────────────────────────────────────────────

/// Start an OS-level file drag. Blocks until the user drops or cancels.
/// Must be called from the UI thread. Files are always **copied**, never
/// moved from the original location.
pub fn start_file_drag(paths: &[String]) -> bool {
    if paths.is_empty() { return false; }
    unsafe {
        OleInitialize(std::ptr::null_mut());
        let obj = new_data_object(paths);
        let src = new_drop_source();
        let mut effect: u32 = DROPEFFECT_NONE;
        let hr = DoDragDrop(obj, src, DROPEFFECT_COPY, &mut effect);
        do_release(obj);
        ds_release(src);
        hr == DRAGDROP_S_DROP
    }
}
