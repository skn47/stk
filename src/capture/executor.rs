use std::io;
use std::process::{Command, Stdio};
use std::sync::Mutex;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[cfg(unix)]
use crate::capture::signal_forward::SignalForwarder;

/// The outcome of running a child command to completion.
#[derive(Debug, Clone, Default)]
pub struct ExecutionResult {
    pub exit_code: i32,
    /// Set instead of `exit_code` when the child was killed by a signal rather than exiting normally.
    pub terminating_signal: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Like [`ExecutionResult`], but without captured output — used where output is inherited
/// directly rather than passing through our code.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExitInfo {
    pub exit_code: i32,
    pub terminating_signal: Option<i32>,
}

/// Abstracts child-process spawning so the CLI dispatch logic can be tested without
/// actually spawning real commands like `cargo` or `git`.
pub trait CommandExecutor {
    fn execute(&self, command: &str, args: &[String]) -> io::Result<ExecutionResult>;

    /// Like `execute`, but streams the child's stdio live instead of buffering it. Used by `run`/`proxy`.
    fn execute_inherited(&self, command: &str, args: &[String]) -> io::Result<ExitInfo>;
}

#[cfg(unix)]
fn terminating_signal(status: &std::process::ExitStatus) -> Option<i32> {
    status.signal()
}

#[cfg(not(unix))]
fn terminating_signal(_status: &std::process::ExitStatus) -> Option<i32> {
    None
}

/// Spawns a real child process. Used by `main.rs`.
pub struct RealExecutor;

impl CommandExecutor for RealExecutor {
    fn execute(&self, command: &str, args: &[String]) -> io::Result<ExecutionResult> {
        let output = Command::new(command).args(args).output()?;
        Ok(ExecutionResult {
            exit_code: output.status.code().unwrap_or(0),
            terminating_signal: terminating_signal(&output.status),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }

    fn execute_inherited(&self, command: &str, args: &[String]) -> io::Result<ExitInfo> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        // If this fails (e.g. fd exhaustion), degrade to no signal forwarding rather than
        // leaking the already-spawned child by returning early without waiting on it.
        #[cfg(unix)]
        let forwarder = SignalForwarder::spawn(child.id()).ok();

        let status = child.wait()?;

        // Tell the forwarder the pid has been reaped before it (or its Drop) can act on
        // it again, narrowing the window where a signal could hit a reused pid.
        #[cfg(unix)]
        if let Some(forwarder) = &forwarder {
            forwarder.mark_child_reaped();
        }

        Ok(ExitInfo {
            exit_code: status.code().unwrap_or(0),
            terminating_signal: terminating_signal(&status),
        })
    }
}

/// Test double for [`CommandExecutor`]: records every invocation instead of spawning a
/// real child process, and returns a canned [`ExecutionResult`] for each one.
pub struct FakeExecutor {
    invocations: Mutex<Vec<(String, Vec<String>)>>,
    response: ExecutionResult,
    inherited_response: ExitInfo,
}

impl FakeExecutor {
    pub fn new() -> Self {
        Self {
            invocations: Mutex::new(Vec::new()),
            response: ExecutionResult::default(),
            inherited_response: ExitInfo::default(),
        }
    }

    pub fn with_response(mut self, response: ExecutionResult) -> Self {
        self.response = response;
        self
    }

    pub fn with_inherited_response(mut self, response: ExitInfo) -> Self {
        self.inherited_response = response;
        self
    }

    pub fn invocations(&self) -> Vec<(String, Vec<String>)> {
        self.invocations.lock().unwrap().clone()
    }
}

impl Default for FakeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandExecutor for FakeExecutor {
    fn execute(&self, command: &str, args: &[String]) -> io::Result<ExecutionResult> {
        self.invocations
            .lock()
            .unwrap()
            .push((command.to_string(), args.to_vec()));
        Ok(self.response.clone())
    }

    fn execute_inherited(&self, command: &str, args: &[String]) -> io::Result<ExitInfo> {
        self.invocations
            .lock()
            .unwrap()
            .push((command.to_string(), args.to_vec()));
        Ok(self.inherited_response)
    }
}
