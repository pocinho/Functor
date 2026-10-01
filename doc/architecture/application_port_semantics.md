# Application Port Semantics

This document defines the asynchronous behavior of Application ports. A caller owns the
`CancellationToken` passed to a port and must stop treating the operation as current after
cancellation.

## Cancellation

- File operations receive the workflow token and pass it to asynchronous reads and writes.
- File enumeration checks the token while collecting paths.
- Clipboard and dialog adapters check the token before and after framework calls.
- Tokenizers receive the workflow token and must observe it while producing output.
- `OperationCanceledException` is not converted into an application error command. The
  interpreter drops the result because the workflow is no longer current.

## Recoverable Failures

- File and tokenizer ports return `Result` data for expected failures. Coordinators preserve
  those failures as application commands or workspace result errors.
- Clipboard and dialog ports use `option` for normal absence: unavailable clipboard text,
  unavailable UI host, and user cancellation of a picker are not failures.
- Unexpected adapter exceptions are reserved for invariant or process-level faults and are
  converted by the outer interpreter into the established failure command.
- Port error messages are diagnostic text only; callers must not use message text for control
  flow.

## Timeout

Ports do not create implicit timers. The coordinator that owns a workflow chooses a timeout,
links it to that workflow's cancellation token, and treats timeout cancellation exactly like
other cancellation. This keeps timeout policy out of storage, UI, and tokenizer adapters.

## Ordering And Stale Results

- A coordinator owns one cancellation token per workflow and cancels superseded work.
- Tokenization results are accepted only when their document revision and request identity are
  still current.
- Workspace replacement rereads each source snapshot before writing and reports changed files
  as stale instead of overwriting them.
- Workspace search and replacement preserve deterministic path ordering and report partial
  successes without discarding completed work.
