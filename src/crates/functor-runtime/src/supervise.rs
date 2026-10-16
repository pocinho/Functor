//! Running one child process under supervision: bounded capture, timeout, cancellation and
//! process-tree termination (ADR-0012).
//!
//! The child runs in a Windows Job Object (a process group elsewhere), so killing it kills every
//! process it started. This is cleanup, not a sandbox: it does not restrict what the child can
//! read, write or connect to.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use functor_core::{CancelToken, StreamCapture};
#[cfg(windows)]
use process_wrap::tokio::JobObject;
#[cfg(unix)]
use process_wrap::tokio::ProcessGroup;
use process_wrap::tokio::{ChildWrapper, CommandWrap, KillOnDrop};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::task::JoinHandle;
use tokio::time::sleep;

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
    /// A stream could not be captured.
    Read(io::Error),
}

/// After the process tree is gone, how long to wait for the output pipes to reach end-of-file.
///
/// On Windows another process started at the same moment (even for an unrelated run) can inherit
/// a pipe's write end, which keeps the pipe open after our child is gone. We take what was
/// captured and mark the stream truncated instead of waiting for that process.
const DRAIN_GRACE: Duration = Duration::from_secs(2);

/// A reader task filling a shared, bounded buffer.
struct Reader {
    shared: Arc<Mutex<StreamCapture>>,
    handle: JoinHandle<io::Result<()>>,
}

fn capture(mut stream: impl AsyncRead + Unpin + Send + 'static, limit: usize) -> Reader {
    let shared = Arc::new(Mutex::new(StreamCapture::default()));
    let sink = Arc::clone(&shared);
    let handle = tokio::spawn(async move {
        let mut buffer = [0_u8; 8192];
        loop {
            match stream.read(&mut buffer).await {
                Ok(0) => return Ok(()),
                Ok(n) => {
                    let mut kept = sink.lock().unwrap_or_else(PoisonError::into_inner);
                    let room = limit.saturating_sub(kept.bytes.len());
                    kept.truncated |= n > room;
                    kept.bytes.extend_from_slice(&buffer[..n.min(room)]);
                    // Keep reading past the limit: the child must never block on a full pipe.
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
    });
    Reader { shared, handle }
}

impl Reader {
    /// What was captured, waiting for end-of-file only until `until`.
    async fn finish(self, until: Instant) -> io::Result<StreamCapture> {
        while !self.handle.is_finished() && Instant::now() < until {
            sleep(POLL).await;
        }
        let mut captured = self
            .shared
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        if self.handle.is_finished() {
            self.handle.await.map_err(io::Error::other)??;
        } else {
            captured.truncated = true;
            self.handle.abort();
        }
        Ok(captured)
    }
}

async fn finished(reader: Option<Reader>, until: Instant) -> io::Result<StreamCapture> {
    match reader {
        Some(reader) => reader.finish(until).await,
        None => Ok(StreamCapture::default()),
    }
}

/// Runs the child to completion, to the timeout, or until `cancel` is set; then terminates
/// everything it started.
pub(crate) async fn run(
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
    wrap.wrap(KillOnDrop);
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
                terminate(child.as_mut()).await;
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
        sleep(POLL).await;
    }

    // Whatever the reason we stopped, nothing the child started may outlive the run.
    terminate(child.as_mut()).await;
    let until = Instant::now() + DRAIN_GRACE;
    let stdout = finished(stdout, until).await;
    let stderr = finished(stderr, until).await;

    Ok(Supervised {
        exit_code: status.and_then(|done| done.code()),
        exited: status.is_some(),
        interrupt,
        stdout: stdout.map_err(Failure::Read)?,
        stderr: stderr.map_err(Failure::Read)?,
    })
}

async fn terminate(child: &mut dyn ChildWrapper) {
    // Terminate the whole job (or process group), then reap only the leader process: it is already
    // dead or about to be, so this returns promptly (see the note in `run`).
    let _ = child.start_kill();
    let _ = child.inner_mut().wait().await;
}

#[cfg(test)]
mod tests {
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use tokio::io::ReadBuf;

    use super::*;

    struct Broken;

    impl AsyncRead for Broken {
        fn poll_read(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
            _: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            Poll::Ready(Err(io::Error::other("stream fault")))
        }
    }

    #[tokio::test]
    async fn stream_read_errors_are_not_success_shaped_captures() {
        let reader = capture(Broken, 16);
        let result = reader.finish(Instant::now() + Duration::from_secs(1)).await;
        assert!(matches!(result, Err(error) if error.to_string() == "stream fault"));
    }

    #[tokio::test]
    async fn a_stream_past_the_shared_drain_deadline_is_cut_and_marked_truncated() {
        let (stream, _writer) = tokio::io::duplex(16);
        let result = capture(stream, 16).finish(Instant::now()).await;
        assert!(matches!(result, Ok(captured) if captured.truncated && captured.bytes.is_empty()));
    }
}
