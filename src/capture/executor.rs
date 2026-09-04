use std::io;
use std::process::Command;
use std::sync::Mutex;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

/// The outcome of running a child command to completion.
#[derive(Debug, Clone, Default)]
pub struct ExecutionResult {
    pub exit_code: i32,
    /// Set instead of `exit_code` when the child was killed by a signal rather than exiting normally.
    pub terminating_signal: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Abstracts child-process spawning so the CLI dispatch logic can be tested without
/// actually spawning real commands like `cargo` or `git`.
pub trait CommandExecutor {
    fn execute(&self, command: &str, args: &[String]) -> io::Result<ExecutionResult>;
}

/// Spawns a real child process. Used by `main.rs`.
pub struct RealExecutor;

impl CommandExecutor for RealExecutor {
    fn execute(&self, command: &str, args: &[String]) -> io::Result<ExecutionResult> {
        let output = Command::new(command).args(args).output()?;
        #[cfg(unix)]
        let terminating_signal = output.status.signal();
        #[cfg(not(unix))]
        let terminating_signal = None;
        Ok(ExecutionResult {
            exit_code: output.status.code().unwrap_or(0),
            terminating_signal,
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

/// Test double for [`CommandExecutor`]: records every invocation instead of spawning a
/// real child process, and returns a canned [`ExecutionResult`] for each one.
pub struct FakeExecutor {
    invocations: Mutex<Vec<(String, Vec<String>)>>,
    response: ExecutionResult,
}

impl FakeExecutor {
    pub fn new() -> Self {
        Self {
            invocations: Mutex::new(Vec::new()),
            response: ExecutionResult::default(),
        }
    }

    pub fn with_response(mut self, response: ExecutionResult) -> Self {
        self.response = response;
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
}
