//! `functor watch`: stream repository events.
//!
//! Reads the event log through [`Follower`]; it never takes the write lock and never writes.

use std::collections::BTreeMap;
use std::future::Future;
use std::io::{self, Write};
use std::pin::pin;
use std::str::FromStr;
use std::task::Poll;
use std::time::Duration;

use functor_core::{ArtifactRepository, EventPosition, FunctorError, Result};
use functor_events::{FollowOptions, Follower, Update, encode_event};
use functor_model::{ArtifactId, Event, EventKind, EventSubject, MetadataValue};

use crate::cli::{OutputMode, WatchArgs};
use crate::context::{Context, Globals};
use crate::render::sanitize;
use crate::resolve;

/// The result of racing two futures.
#[derive(Debug, PartialEq, Eq)]
pub enum Either<A, B> {
    /// The first future finished first.
    Left(A),
    /// The second future finished first.
    Right(B),
}

/// Polls both futures and returns whichever finishes first (the first one wins a tie); the other
/// is dropped. Used instead of `tokio::select!` so that `tokio` needs no `macros` feature.
pub async fn race<A: Future, B: Future>(a: A, b: B) -> Either<A::Output, B::Output> {
    let mut a = pin!(a);
    let mut b = pin!(b);
    std::future::poll_fn(|cx| {
        if let Poll::Ready(value) = a.as_mut().poll(cx) {
            return Poll::Ready(Either::Left(value));
        }
        if let Poll::Ready(value) = b.as_mut().poll(cx) {
            return Poll::Ready(Either::Right(value));
        }
        Poll::Pending
    })
    .await
}

/// Resolves when the user presses Ctrl-C (on Windows, the console control event). If the handler
/// cannot be installed, never resolves: the command then runs until another limit stops it.
async fn interrupted() {
    if tokio::signal::ctrl_c().await.is_err() {
        std::future::pending::<()>().await;
    }
}

/// Which events to show.
#[derive(Debug, Default)]
struct Filter {
    types: Vec<String>,
    subject: Option<ArtifactId>,
}

impl Filter {
    fn accepts(&self, event: &Event) -> bool {
        let type_ok = self.types.is_empty()
            || self
                .types
                .iter()
                .any(|prefix| event.kind.matches_prefix(prefix));
        let subject_ok = self
            .subject
            .is_none_or(|id| event.subject == EventSubject::Artifact(id));
        type_ok && subject_ok
    }
}

/// Artifact slugs for the text output, resolved when a line is printed.
///
/// The cache is refreshed when an id is missing and cleared by `artifact.updated`, so renames show
/// up in later lines; an artifact that no longer resolves is printed by id.
#[derive(Debug, Default)]
struct Slugs {
    names: BTreeMap<ArtifactId, String>,
}

impl Slugs {
    async fn label(&mut self, repository: &dyn ArtifactRepository, id: &ArtifactId) -> String {
        if let Some(name) = self.names.get(id) {
            return name.clone();
        }
        if let Ok(artifacts) = repository.list().await {
            self.names = artifacts
                .iter()
                .map(|a| (*a.id(), a.slug().to_string()))
                .collect();
        }
        self.names
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.to_string())
    }

    fn forget(&mut self) {
        self.names.clear();
    }
}

fn text_field<'a>(event: &'a Event, key: &str) -> Option<&'a str> {
    match event.data.get(key) {
        Some(MetadataValue::String(text)) => Some(text.as_str()),
        _ => None,
    }
}

/// `<seq>  <at>  <type>  <summary>`
async fn text_line(
    event: &Event,
    slugs: &mut Slugs,
    repository: &dyn ArtifactRepository,
) -> String {
    if event.kind == EventKind::ArtifactUpdated {
        slugs.forget();
    }
    let summary = match (event.kind, &event.subject) {
        (EventKind::RelationshipCreated, _) => {
            let mut ends = Vec::new();
            for key in ["source", "target"] {
                let end = match text_field(event, key).and_then(|t| ArtifactId::from_str(t).ok()) {
                    Some(id) => slugs.label(repository, &id).await,
                    None => "?".to_owned(),
                };
                ends.push(end);
            }
            let kind = text_field(event, "relationship_type").unwrap_or("?");
            format!("{} {} {}", ends[0], sanitize(kind), ends[1])
        }
        (EventKind::ValidationCompleted, _) => {
            sanitize(text_field(event, "outcome").unwrap_or("?"))
        }
        (EventKind::RunStarted, EventSubject::Artifact(id)) => {
            format!(
                "{} provider={}",
                slugs.label(repository, id).await,
                sanitize(text_field(event, "provider").unwrap_or("?"))
            )
        }
        (EventKind::RunCompleted | EventKind::ValidationRecorded, EventSubject::Artifact(id)) => {
            format!(
                "{} {}",
                slugs.label(repository, id).await,
                sanitize(text_field(event, "outcome").unwrap_or("?"))
            )
        }
        (_, EventSubject::Artifact(id)) => slugs.label(repository, id).await,
        (_, other) => other.canonical(),
    };
    format!("{}  {}  {}  {}", event.seq, event.at, event.kind, summary)
}

/// Writes one line and flushes, so pipes see it at once. A closed pipe ends the command quietly.
fn write_line(line: &str) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(line.as_bytes())
        .and_then(|()| stdout.write_all(b"\n"))
        .and_then(|()| stdout.flush())
}

async fn stream(
    follower: &mut Follower,
    args: &WatchArgs,
    filter: &Filter,
    repository: &dyn ArtifactRepository,
    output: OutputMode,
) -> Result<()> {
    let idle = args.timeout.map(Duration::from_secs);
    let mut slugs = Slugs::default();
    let mut shown = 0_u64;
    loop {
        let Some(update) = follower.next_within(idle).await? else {
            return Ok(());
        };
        let event = match update {
            Update::Warning(text) => {
                eprintln!("warning: {text}");
                continue;
            }
            Update::Event(event) => event,
        };
        if !filter.accepts(&event) {
            continue;
        }
        let line = match output {
            OutputMode::Json => encode_event(&event),
            OutputMode::Text => text_line(&event, &mut slugs, repository).await,
        };
        match write_line(&line) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => return Ok(()),
            Err(error) => return Err(FunctorError::storage("write output", "stdout", error)),
        }
        shown += 1;
        if args.count == Some(shown) {
            return Ok(());
        }
    }
}

/// Runs `watch` until a limit is reached, the output pipe closes or the user interrupts it.
///
/// # Errors
/// `NotFound` for an unknown `--subject`, or `Storage` when the log cannot be read.
pub async fn run(args: WatchArgs, context: &Context, globals: &Globals) -> Result<()> {
    // `watch` outlives any one snapshot, so it reads the store directly rather than the cache.
    let repository: &dyn ArtifactRepository = context.store.inner();
    let subject = match &args.subject {
        Some(text) => Some(*resolve::artifact(repository, text).await?.id()),
        None => None,
    };
    let filter = Filter {
        types: args.types.clone(),
        subject,
    };
    let position = if args.from_start {
        EventPosition::Start
    } else if let Some(seq) = args.after {
        EventPosition::After(seq)
    } else {
        EventPosition::End
    };
    let mut follower =
        Follower::open(&context.handle.root, position, FollowOptions::default()).await?;

    let watching = stream(&mut follower, &args, &filter, repository, globals.output);
    match race(interrupted(), watching).await {
        Either::Left(()) => Ok(()),
        Either::Right(result) => result,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use functor_model::{EventData, EventId, NewEvent, RelationshipId, RepositoryId, Timestamp};

    use super::*;

    fn event(kind: EventKind, subject: EventSubject) -> Event {
        Event::new(
            EventId::from_bits(1),
            1,
            Timestamp::parse("2026-10-12T18:00:00Z").unwrap(),
            RepositoryId::from_bits(1),
            NewEvent::new(kind, subject, EventData::new()),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn race_returns_the_first_future_to_finish() {
        let slow = async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            "slow"
        };
        let fast = async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            "fast"
        };
        assert_eq!(race(slow, fast).await, Either::Right("fast"));

        let never = std::future::pending::<()>();
        let done = async { 7 };
        assert_eq!(race(never, done).await, Either::Right(7));
    }

    #[tokio::test]
    async fn race_prefers_the_first_future_on_a_tie() {
        assert_eq!(race(async { 1 }, async { 2 }).await, Either::Left(1));
    }

    #[test]
    fn filters_match_type_prefixes_and_the_subject() {
        let artifact = ArtifactId::from_bits(5);
        let created = event(EventKind::ArtifactCreated, EventSubject::Artifact(artifact));
        let related = event(
            EventKind::RelationshipCreated,
            EventSubject::Relationship(RelationshipId::from_bits(2)),
        );

        assert!(Filter::default().accepts(&created));
        let by_prefix = |prefix: &str| Filter {
            types: vec![prefix.to_owned()],
            subject: None,
        };
        assert!(by_prefix("artifact.").accepts(&created));
        assert!(by_prefix("artifact.created").accepts(&created));
        assert!(!by_prefix("artifact.updated").accepts(&created));
        assert!(!by_prefix("artifact.").accepts(&related));
        assert!(by_prefix("relationship").accepts(&related));
        assert!(!by_prefix("nothing.like.it").accepts(&created));

        let several = Filter {
            types: vec!["validation.".to_owned(), "relationship.".to_owned()],
            subject: None,
        };
        assert!(several.accepts(&related) && !several.accepts(&created));

        let subject = Filter {
            types: Vec::new(),
            subject: Some(artifact),
        };
        assert!(subject.accepts(&created));
        assert!(!subject.accepts(&related));
        let other = event(
            EventKind::ArtifactCreated,
            EventSubject::Artifact(ArtifactId::from_bits(6)),
        );
        assert!(!subject.accepts(&other));
    }
}
