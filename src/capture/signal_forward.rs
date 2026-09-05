//! Forwards SIGINT/SIGTERM received by this process to a spawned child, for verbs
//! (`run`/`proxy`) whose child does not automatically receive them via the terminal's
//! process-group delivery (e.g. when the signal is sent directly via `kill <stk-pid>`).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use signal_hook::consts::signal::{SIGINT, SIGTERM};
use signal_hook::iterator::{Handle, Signals};

enum Target {
    /// The child hasn't been spawned yet: a signal arriving now waits (briefly) rather
    /// than being dropped, so registering the handler before spawning closes the race
    /// window that existed when the child's pid was supplied at construction time.
    AwaitingChild,
    Active(libc::pid_t),
    /// The child has been reaped: never signal its pid again, since the OS may have
    /// already reused it for an unrelated process.
    Done,
}

pub struct SignalForwarder {
    handle: Handle,
    thread: Option<std::thread::JoinHandle<()>>,
    target: Arc<Mutex<Target>>,
}

impl SignalForwarder {
    /// Registers the signal handler immediately, before any child exists. Call
    /// [`set_child_pid`](Self::set_child_pid) once the child is actually spawned.
    pub fn spawn() -> std::io::Result<Self> {
        let mut signals = Signals::new([SIGINT, SIGTERM])?;
        let handle = signals.handle();
        let target = Arc::new(Mutex::new(Target::AwaitingChild));
        let thread_target = Arc::clone(&target);
        let thread = std::thread::spawn(move || {
            for sig in &mut signals {
                loop {
                    match *thread_target.lock().unwrap() {
                        Target::Active(pid) => {
                            unsafe {
                                libc::kill(pid, sig);
                            }
                            break;
                        }
                        Target::Done => break,
                        Target::AwaitingChild => {}
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
        });
        Ok(Self {
            handle,
            thread: Some(thread),
            target,
        })
    }

    pub fn set_child_pid(&self, pid: u32) {
        *self.target.lock().unwrap() = Target::Active(pid as libc::pid_t);
    }

    pub fn mark_child_reaped(&self) {
        *self.target.lock().unwrap() = Target::Done;
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
