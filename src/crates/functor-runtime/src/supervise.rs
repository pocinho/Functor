//! Running one child process under supervision: bounded capture, timeout, cancellation and
//! process-tree termination (ADR-0012).
//!
//! The child runs in a Windows Job Object (a process group elsewhere), so killing it kills every
//! process it started. This is cleanup, not a sandbox: it does not restrict what the child can
//! read, write or connect to.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use functor_core::{CancelToken, StreamCapture};
#[cfg(windows)]
use process_wrap::std::JobObject;
#[cfg(unix)]
use process_wrap::std::ProcessGroup;
use process_wrap::std::{ChildWrapper, CommandWrap};

/// How often the supervisor looks at the child, the clock and the cancel token.
const POLL: Duration = Duration::from_millis(10);

/// What to run.
pub(crate) struct Spawn {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: BTreeMap<OsString, OsString>,
}

/// Why supervision ended early, if it did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Interrupt {
    TimedOut,
    Cancelled,
}

/// What happened to the child.
#[derive(Debug)]
pub(crate) struct Supervised {
    /// `None` when the child was killed by the supervisor or by a signal.
    pub exit_code: Option<i32>,
    /// Whether the exit status was reported at all (a normal exit or a signal).
    pub exited: bool,
    pub interrupt: Option<Interrupt>,
    pub stdout: StreamCapture,
    pub stderr: StreamCapture,
}

/// A failure of the supervisor itself, as opposed to anything the child did.
#[derive(Debug)]
pub(crate) enum Failure {
    /// The child could not be started.
    Spawn(io::Error),
    /// Waiting for the child failed.
    Wait(io::Error),
}

/// After the process tree is gone, how long to wait for the output pipes to reach end-of-file.
///
/// On Windows another process started at the same moment (even for an unrelated run) can inherit
/// a pipe's write end, which keeps the pipe open after our child is gone. We take what was
/// captured and mark the stream truncated instead of waiting for that process.
const DRAIN_GRACE: Duration = Duration::from_secs(2);

/// A reader thread filling a shared, bounded buffer.
struct Reader {
    shared: Arc<Mutex<StreamCapture>>,
    handle: JoinHandle<()>,
}

fn capture(mut stream: impl Read + Send + 'static, limit: usize) -> Reader {
    let shared = Arc::new(Mutex::new(StreamCapture::default()));
    let sink = Arc::clone(&shared);
    let handle = thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let mut kept = sink.lock().unwrap_or_else(PoisonError::into_inner);
                    let room = limit.saturating_sub(kept.bytes.len());
                    kept.truncated |= n > room;
                    kept.bytes.extend_from_slice(&buffer[..n.min(room)]);
                    // Keep reading past the limit: the child must never block on a full pipe.
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
    });
    Reader { shared, handle }
}

impl Reader {
    /// What was captured, waiting for end-of-file only until `until`.
    fn finish(self, until: Instant) -> StreamCapture {
        while !self.handle.is_finished() && Instant::now() < until {
            thread::sleep(POLL);
        }
        let mut captured = self
            .shared
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        if !self.handle.is_finished() {
            captured.truncated = true;
        }
        captured
    }
}

fn finished(reader: Option<Reader>, until: Instant) -> StreamCapture {
    reader
        .map(|reader| reader.finish(until))
        .unwrap_or_default()
}

/// Runs the child to completion, to the timeout, or until `cancel` is set; then terminates
/// everything it started.
pub(crate) fn run(
    spawn: &Spawn,
    timeout: Duration,
    stream_limit: u64,
    cancel: &CancelToken,
) -> Result<Supervised, Failure> {
    let mut wrap = CommandWrap::with_new(&spawn.program, |command| {
        command
            .args(&spawn.args)
            .current_dir(&spawn.cwd)
            .env_clear()
            .envs(&spawn.env)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    });
    #[cfg(windows)]
    wrap.wrap(JobObject);
    #[cfg(unix)]
    wrap.wrap(ProcessGroup::leader());
    let mut child = wrap.spawn().map_err(Failure::Spawn)?;

    let limit = usize::try_from(stream_limit).unwrap_or(usize::MAX);
    let stdout = child.stdout().take().map(|pipe| capture(pipe, limit));
    let stderr = child.stderr().take().map(|pipe| capture(pipe, limit));

    let deadline = Instant::now() + timeout;
    let mut interrupt = None;
    let mut status = None;
    loop {
        // Poll the leader process itself. The Job Object wrapper's own `try_wait` and `wait` both
        // read the job's completion port, so mixing them can consume the "job is empty" message
        // and leave a later `wait` blocked forever; the job is used only to terminate.
        match child.inner_mut().try_wait() {
            Ok(Some(done)) => {
                status = Some(done);
                break;
            }
            Ok(None) => {}
            Err(error) => {
                terminate(child.as_mut());
                return Err(Failure::Wait(error));
            }
        }
        if cancel.is_cancelled() {
            interrupt = Some(Interrupt::Cancelled);
            break;
        }
        if Instant::now() >= deadline {
            interrupt = Some(Interrupt::TimedOut);
            break;
        }
        thread::sleep(POLL);
    }

    // Whatever the reason we stopped, nothing the child started may outlive the run.
    terminate(child.as_mut());
    let until = Instant::now() + DRAIN_GRACE;
    let stdout = finished(stdout, until);
    let stderr = finished(stderr, until);

    Ok(Supervised {
        exit_code: status.and_then(|done| done.code()),
        exited: status.is_some(),
        interrupt,
        stdout,
        stderr,
    })
}

fn terminate(child: &mut dyn ChildWrapper) {
    // Terminate the whole job (or process group), then reap only the leader process: it is already
    // dead or about to be, so this returns promptly (see the note in `run`).
    let _ = child.start_kill();
    let _ = child.inner_mut().wait();
}
