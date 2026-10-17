//! Bound SDK calls and retain each client until its cancellation thread exits.
use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Duration;

pub(super) struct ClientLease {
    client: *mut Client,
    cancel: Option<mpsc::Sender<()>>,
    worker: Option<std::thread::JoinHandle<()>>,
    timeout: Arc<AtomicBool>,
}

impl ClientLease {
    pub(super) fn new(client: *mut Client, seconds: u64) -> Self {
        let (send, receive) = mpsc::channel();
        let timeout = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&timeout);
        let address = client as usize;
        let worker = std::thread::spawn(move || {
            if matches!(
                receive.recv_timeout(Duration::from_secs(seconds)),
                Err(mpsc::RecvTimeoutError::Timeout)
            ) {
                flag.store(true, Ordering::Release);
                // Official API explicitly permits shutdown from any thread.
                unsafe {
                    Everything3_ShutdownClient(address as *mut Client);
                }
            }
        });
        Self {
            client,
            cancel: Some(send),
            worker: Some(worker),
            timeout,
        }
    }

    pub(super) fn timed_out(&self) -> bool {
        self.timeout.load(Ordering::Acquire)
    }
}

impl Drop for ClientLease {
    fn drop(&mut self) {
        if let Some(send) = self.cancel.take() {
            let _ = send.send(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        unsafe {
            Everything3_DestroyClient(self.client);
        }
    }
}

pub(super) fn connect(named: bool) -> *mut Client {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn WaitNamedPipeW(name: *const u16, timeout: u32) -> i32;
        fn ProcessIdToSessionId(process: u32, session: *mut u32) -> i32;
        fn SetLastError(error: u32);
    }
    let suffix = if named { " (1.5a)" } else { "" };
    let mut names = Vec::new();
    let mut session = 0;
    unsafe {
        if ProcessIdToSessionId(std::process::id(), &mut session) != FALSE {
            names.push(format!(
                r"\\.\PIPE\Everything IPC Session {session}{suffix}"
            ));
        }
        names.push(format!(r"\\.\PIPE\Everything IPC{suffix}"));
        let mut failure = 0xE000_0002;
        for name in names {
            let name = name.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
            // SDK 3 waits up to 30s on a busy pipe. Probe first so initial
            // indexing cannot monopolize the worker's query queue.
            if WaitNamedPipeW(name.as_ptr(), 1) != FALSE {
                return Everything3_ConnectUTF8(if named {
                    c"1.5a".as_ptr()
                } else {
                    std::ptr::null()
                });
            }
            let error = Everything3_GetLastError();
            if error != 2 {
                failure = error;
            }
        }
        SetLastError(failure);
    }
    std::ptr::null_mut()
}
