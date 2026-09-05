//! Forwards SIGINT/SIGTERM received by this process to a spawned child, for verbs
//! (`run`/`proxy`) whose child does not automatically receive them via the terminal's
//! process-group delivery (e.g. when the signal is sent directly via `kill <stk-pid>`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use signal_hook::consts::signal::{SIGINT, SIGTERM};
use signal_hook::iterator::{Handle, Signals};

pub struct SignalForwarder {
    handle: Handle,
    thread: Option<std::thread::JoinHandle<()>>,
    child_reaped: Arc<AtomicBool>,
}

impl SignalForwarder {
    pub fn spawn(child_pid: u32) -> std::io::Result<Self> {
        let mut signals = Signals::new([SIGINT, SIGTERM])?;
        let handle = signals.handle();
        let child_reaped = Arc::new(AtomicBool::new(false));
        let thread_child_reaped = Arc::clone(&child_reaped);
        let thread = std::thread::spawn(move || {
            for sig in &mut signals {
                // Best-effort: narrows, but can't fully close, the race against the OS
                // reusing `child_pid` for an unrelated process after it's been reaped.
                if !thread_child_reaped.load(Ordering::SeqCst) {
                    unsafe {
                        libc::kill(child_pid as libc::pid_t, sig);
                    }
                }
            }
        });
        Ok(Self {
            handle,
            thread: Some(thread),
            child_reaped,
        })
    }

    pub fn mark_child_reaped(&self) {
        self.child_reaped.store(true, Ordering::SeqCst);
    }
}

impl Drop for SignalForwarder {
    fn drop(&mut self) {
        self.handle.close();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
