//! Encoding of evidence records: JSON documents and artifact metadata (ADR-0013, ADR-0015).
//!
//! Serialization lives where the representation lives (ADR-0005). Documents use sorted keys, so
//! the same value always produces the same bytes. Every `decode_*` reports a problem as text; the
//! recorder turns it into `Inconsistent` naming the artifact.

use std::collections::BTreeMap;
use std::str::FromStr;

use functor_core::{FunctorError, Result};
use functor_model::{
    Artifact, ArtifactId, Capture, CaptureRole, ContentDigest, EVIDENCE_KEY, EvidenceKind,
    ExecutionLimits, Finding, Metadata, MetadataKey, MetadataValue, ModelError, Outcome,
    ProgramProvenance, RelationshipId, RepoPath, RequestId, Revision, RuleCode, RunIntent,
    RunOutcome, RunResult, RunResultSpec, Severity, Subject, Timestamp, Truncation,
    ValidationReport,
};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

pub(crate) type Decode<T> = std::result::Result<T, String>;

/// The SHA-256 digest of `bytes`.
pub(crate) fn sha256(bytes: &[u8]) -> ContentDigest {
    ContentDigest::from_bytes(Sha256::digest(bytes).into())
}

// ---------------------------------------------------------------------------------------------
// metadata
// ---------------------------------------------------------------------------------------------

/// Builds artifact metadata, starting with the evidence marker of `kind`.
pub(crate) struct MetaBuilder(Metadata);

impl MetaBuilder {
    pub(crate) fn evidence(kind: EvidenceKind) -> Result<Self> {
        let mut builder = Self(Metadata::new());
        builder.put(EVIDENCE_KEY, kind.schema())?;
        Ok(builder)
    }

    pub(crate) fn plain() -> Self {
        Self(Metadata::new())
    }

    pub(crate) fn put(&mut self, key: &str, value: impl Into<MetadataValue>) -> Result<&mut Self> {
        self.0.insert(MetadataKey::new(key)?, value.into());
        Ok(self)
    }

    pub(crate) fn finish(self) -> Metadata {
        self.0
    }
}

pub(crate) fn int(value: u64) -> Result<i64> {
    i64::try_from(value)
        .map_err(|_| FunctorError::invalid_input("metadata", "integer out of range"))
}

pub(crate) fn meta_str<'a>(artifact: &'a Artifact, key: &str) -> Option<&'a str> {
    match artifact.metadata().get(&MetadataKey::new(key).ok()?)? {
        MetadataValue::String(text) => Some(text),
        _ => None,
    }
}

pub(crate) fn meta_int(artifact: &Artifact, key: &str) -> Option<i64> {
    match artifact.metadata().get(&MetadataKey::new(key).ok()?)? {
        MetadataValue::Integer(number) => Some(*number),
        _ => None,
    }
}

pub(crate) fn meta_id(artifact: &Artifact, key: &str) -> Option<ArtifactId> {
    meta_str(artifact, key)?.parse().ok()
}

pub(crate) fn meta_ids(artifact: &Artifact, key: &str) -> Vec<ArtifactId> {
    let Some(Ok(key)) = Some(MetadataKey::new(key)) else {
        return Vec::new();
    };
    match artifact.metadata().get(&key) {
        Some(MetadataValue::StringList(items)) => {
            items.iter().filter_map(|item| item.parse().ok()).collect()
        }
        _ => Vec::new(),
    }
}

pub(crate) fn capture_metadata(capture: &Capture) -> Result<Metadata> {
    let mut m = MetaBuilder::evidence(EvidenceKind::Capture)?;
    m.put("role", capture.role.as_str())?
        .put("source", capture.source.to_string())?
        .put("source_revision", int(capture.source_revision.get())?)?
        .put("sha256", capture.digest.as_str())?
        .put("size", int(capture.size)?)?;
    Ok(m.finish())
}

/// Reads a capture's record back from its artifact.
pub(crate) fn decode_capture(artifact: &Artifact) -> Decode<Capture> {
    let source = meta_id(artifact, "source").ok_or("missing or malformed `source`")?;
    let revision = meta_int(artifact, "source_revision")
        .and_then(|n| u64::try_from(n).ok())
        .ok_or("missing `source_revision`")?;
    let digest = parse::<ContentDigest>(meta_str(artifact, "sha256").ok_or("missing `sha256`")?)?;
    let size = meta_int(artifact, "size")
        .and_then(|n| u64::try_from(n).ok())
        .ok_or("missing `size`")?;
    let role = CaptureRole::parse(meta_str(artifact, "role").ok_or("missing `role`")?)
        .map_err(|e| e.to_string())?;
    Ok(Capture {
        artifact: *artifact.id(),
        source,
        source_revision: Revision::new(revision).map_err(|e| e.to_string())?,
        role,
        digest,
        size,
    })
}

pub(crate) fn run_metadata(intent: &RunIntent) -> Result<Metadata> {
    let mut m = MetaBuilder::evidence(EvidenceKind::Run)?;
    m.put("provider", intent.provider.as_str())?
        .put("workflow_capture", intent.workflow.to_string())?
        .put("request_id", intent.request_id.as_str())?
        .put("started", intent.started.to_string())?
        .put("network", functor_model::NETWORK_NOT_ENFORCED)?;
    Ok(m.finish())
}

pub(crate) fn result_metadata(result: &RunResult) -> Result<Metadata> {
    let mut m = MetaBuilder::evidence(EvidenceKind::RunResult)?;
    m.put("run", result.run().to_string())?
        .put("outcome", result.outcome().as_str())?
        .put("finished", result.finished().to_string())?
        .put("truncated", result.truncated().any())?
        .put(
            "outputs",
            result
                .outputs()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
        )?;
    if let Some(code) = result.exit_code() {
        m.put("exit_code", i64::from(code))?;
    }
    Ok(m.finish())
}

// ---------------------------------------------------------------------------------------------
// JSON helpers
// ---------------------------------------------------------------------------------------------

fn parse<T: FromStr<Err = ModelError>>(text: &str) -> Decode<T> {
    text.parse::<T>().map_err(|error| error.to_string())
}

fn document(text: &str, schema: &str) -> Decode<Value> {
    let value: Value =
        serde_json::from_str(text).map_err(|error| format!("invalid JSON: {error}"))?;
    match value.get("schema").and_then(Value::as_str) {
        Some(found) if found == schema => Ok(value),
        Some(other) => Err(format!("unsupported schema `{other}`; expected `{schema}`")),
        None => Err("missing `schema`".to_owned()),
    }
}

fn field<'a>(value: &'a Value, key: &str) -> Decode<&'a Value> {
    value.get(key).ok_or_else(|| format!("missing `{key}`"))
}

fn str_field<'a>(value: &'a Value, key: &str) -> Decode<&'a str> {
    field(value, key)?
        .as_str()
        .ok_or_else(|| format!("`{key}` must be a string"))
}

fn u64_field(value: &Value, key: &str) -> Decode<u64> {
    field(value, key)?
        .as_u64()
        .ok_or_else(|| format!("`{key}` must be a non-negative integer"))
}

fn bool_field(value: &Value, key: &str) -> Decode<bool> {
    field(value, key)?
        .as_bool()
        .ok_or_else(|| format!("`{key}` must be a boolean"))
}

fn array_field<'a>(value: &'a Value, key: &str) -> Decode<&'a Vec<Value>> {
    field(value, key)?
        .as_array()
        .ok_or_else(|| format!("`{key}` must be an array"))
}

fn strings(value: &Value, key: &str) -> Decode<Vec<String>> {
    array_field(value, key)?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("`{key}` must contain only strings"))
        })
        .collect()
}

fn render(value: &Value) -> String {
    // Maps are sorted by key (no `preserve_order`), so output is deterministic.
    let mut text = serde_json::to_string_pretty(value).unwrap_or_default();
    text.push('\n');
    text
}

// ---------------------------------------------------------------------------------------------
// run intent
// ---------------------------------------------------------------------------------------------

pub(crate) fn encode_intent(intent: &RunIntent) -> String {
    let inputs: Map<String, Value> = intent
        .inputs
        .iter()
        .map(|(name, id)| (name.clone(), json!(id.to_string())))
        .collect();
    render(&json!({
        "schema": EvidenceKind::Run.schema(),
        "request_id": intent.request_id.as_str(),
        "provider": intent.provider,
        "provider_version": intent.provider_version,
        "workflow": intent.workflow.to_string(),
        "inputs": inputs,
        "args": intent.args,
        "limits": {
            "timeout_seconds": intent.limits.timeout_seconds(),
            "stream_bytes": intent.limits.stream_bytes(),
        },
        "env_names": intent.env_names,
        "program": {
            "path": intent.program.path,
            "sha256": intent.program.digest.as_ref().map(ContentDigest::as_str),
        },
        "platform": intent.platform,
        "started": intent.started.to_string(),
    }))
}

pub(crate) fn decode_intent(text: &str) -> Decode<RunIntent> {
    let value = document(text, EvidenceKind::Run.schema())?;
    let inputs = field(&value, "inputs")?
        .as_object()
        .ok_or("`inputs` must be an object")?
        .iter()
        .map(|(name, id)| {
            let id = id.as_str().ok_or("input ids must be strings")?;
            Ok((name.clone(), parse::<ArtifactId>(id)?))
        })
        .collect::<Decode<BTreeMap<_, _>>>()?;
    let limits = field(&value, "limits")?;
    let timeout = u32::try_from(u64_field(limits, "timeout_seconds")?)
        .map_err(|_| "`timeout_seconds` out of range".to_owned())?;
    let program = field(&value, "program")?;
    let digest = match field(program, "sha256")? {
        Value::Null => None,
        Value::String(text) => Some(parse::<ContentDigest>(text)?),
        _ => return Err("`program.sha256` must be a string or null".to_owned()),
    };
    Ok(RunIntent {
        request_id: parse::<RequestId>(str_field(&value, "request_id")?)?,
        provider: str_field(&value, "provider")?.to_owned(),
        provider_version: str_field(&value, "provider_version")?.to_owned(),
        workflow: parse(str_field(&value, "workflow")?)?,
        inputs,
        args: strings(&value, "args")?,
        limits: ExecutionLimits::new(timeout, u64_field(limits, "stream_bytes")?)
            .map_err(|e| e.to_string())?,
        env_names: strings(&value, "env_names")?,
        program: ProgramProvenance {
            path: str_field(program, "path")?.to_owned(),
            digest,
        },
        platform: str_field(&value, "platform")?.to_owned(),
        started: parse::<Timestamp>(str_field(&value, "started")?)?,
    })
}

// ---------------------------------------------------------------------------------------------
// run result
// ---------------------------------------------------------------------------------------------

/// A stored stream: what it must hash to and how long it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BlobRef {
    pub digest: ContentDigest,
    pub size: u64,
}

/// One published output as recorded in the result document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputRef {
    pub artifact: ArtifactId,
    pub name: String,
    pub digest: ContentDigest,
    pub size: u64,
}

/// A decoded result document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredResult {
    pub result: RunResult,
    pub stdout: BlobRef,
    pub stderr: BlobRef,
}

fn blob_json(blob: &BlobRef) -> Value {
    json!({ "sha256": blob.digest.as_str(), "size": blob.size })
}

fn decode_blob(value: &Value, key: &str) -> Decode<BlobRef> {
    let blob = field(value, key)?;
    Ok(BlobRef {
        digest: parse(str_field(blob, "sha256")?)?,
        size: u64_field(blob, "size")?,
    })
}

pub(crate) fn encode_result(
    result: &RunResult,
    stdout: &BlobRef,
    stderr: &BlobRef,
    outputs: &[OutputRef],
) -> String {
    render(&json!({
        "schema": EvidenceKind::RunResult.schema(),
        "run": result.run().to_string(),
        "outcome": result.outcome().as_str(),
        "finished": result.finished().to_string(),
        "exit_code": result.exit_code(),
        "truncated": {
            "stdout": result.truncated().stdout,
            "stderr": result.truncated().stderr,
        },
        "outputs": outputs.iter().map(|o| json!({
            "artifact": o.artifact.to_string(),
            "name": o.name,
            "sha256": o.digest.as_str(),
            "size": o.size,
        })).collect::<Vec<_>>(),
        "diagnostics": result.diagnostics(),
        "stdout": blob_json(stdout),
        "stderr": blob_json(stderr),
    }))
}

pub(crate) fn decode_result(text: &str, started: Timestamp) -> Decode<StoredResult> {
    let value = document(text, EvidenceKind::RunResult.schema())?;
    let exit_code = match field(&value, "exit_code")? {
        Value::Null => None,
        Value::Number(number) => Some(
            number
                .as_i64()
                .and_then(|n| i32::try_from(n).ok())
                .ok_or("`exit_code` out of range")?,
        ),
        _ => return Err("`exit_code` must be an integer or null".to_owned()),
    };
    let truncated = field(&value, "truncated")?;
    let outputs = array_field(&value, "outputs")?
        .iter()
        .map(|output| parse::<ArtifactId>(str_field(output, "artifact")?))
        .collect::<Decode<Vec<_>>>()?;
    let spec = RunResultSpec {
        outcome: parse::<RunOutcome>(str_field(&value, "outcome")?)?,
        finished: parse::<Timestamp>(str_field(&value, "finished")?)?,
        exit_code,
        truncated: Truncation {
            stdout: bool_field(truncated, "stdout")?,
            stderr: bool_field(truncated, "stderr")?,
        },
        outputs,
        diagnostics: strings(&value, "diagnostics")?,
    };
    let run = parse::<ArtifactId>(str_field(&value, "run")?)?;
    Ok(StoredResult {
        result: RunResult::new(run, started, spec).map_err(|e| e.to_string())?,
        stdout: decode_blob(&value, "stdout")?,
        stderr: decode_blob(&value, "stderr")?,
    })
}

// ---------------------------------------------------------------------------------------------
// validation report (the shape of `functor validate --output json`, schema_version 1)
// ---------------------------------------------------------------------------------------------

fn subject_json(subject: &Subject) -> Decode<Value> {
    Ok(match subject {
        Subject::Repository => json!({ "kind": "repository" }),
        Subject::Artifact(id) => json!({ "kind": "artifact", "id": id.to_string() }),
        Subject::Relationship(id) => json!({ "kind": "relationship", "id": id.to_string() }),
        Subject::File(path) => json!({ "kind": "file", "path": path.as_str() }),
        _ => return Err("a finding subject this version cannot store".to_owned()),
    })
}

fn decode_subject(value: &Value) -> Decode<Subject> {
    match str_field(value, "kind")? {
        "repository" => Ok(Subject::Repository),
        "artifact" => Ok(Subject::Artifact(parse(str_field(value, "id")?)?)),
        "relationship" => Ok(Subject::Relationship(parse::<RelationshipId>(str_field(
            value, "id",
        )?)?)),
        "file" => Ok(Subject::File(
            RepoPath::new(str_field(value, "path")?).map_err(|e| e.to_string())?,
        )),
        other => Err(format!("unknown subject kind `{other}`")),
    }
}

/// Encodes a report as the `validate` JSON document.
///
/// # Errors
/// Fails only for a subject kind this version cannot represent.
pub(crate) fn encode_report(report: &ValidationReport, strict: bool) -> Decode<String> {
    let findings = report
        .findings()
        .iter()
        .map(|finding| {
            let related = finding
                .related
                .iter()
                .map(subject_json)
                .collect::<Decode<Vec<_>>>()?;
            Ok(json!({
                "code": finding.code.as_str(),
                "severity": finding.severity.as_str(),
                "subject": subject_json(&finding.subject)?,
                "message": finding.message,
                "related": related,
            }))
        })
        .collect::<Decode<Vec<_>>>()?;
    let counts = report.counts();
    Ok(render(&json!({
        "schema_version": 1,
        "outcome": report.outcome().as_str(),
        "strict": strict,
        "counts": {
            "artifacts": counts.artifacts,
            "relationships": counts.relationships,
            "errors": counts.errors,
            "warnings": counts.warnings,
            "infos": counts.infos,
        },
        "findings": findings,
    })))
}

/// Rebuilds the report; `completed` is not in the document (it keeps `validate` output
/// reproducible), so the caller passes it from the artifact metadata.
pub(crate) fn decode_report(text: &str, completed: Timestamp) -> Decode<(ValidationReport, bool)> {
    let value: Value =
        serde_json::from_str(text).map_err(|error| format!("invalid JSON: {error}"))?;
    if field(&value, "schema_version")?.as_u64() != Some(1) {
        return Err("unsupported `schema_version`".to_owned());
    }
    let counts = field(&value, "counts")?;
    let artifacts = usize::try_from(u64_field(counts, "artifacts")?).map_err(|e| e.to_string())?;
    let relationships =
        usize::try_from(u64_field(counts, "relationships")?).map_err(|e| e.to_string())?;
    let findings = array_field(&value, "findings")?
        .iter()
        .map(|item| {
            let related = array_field(item, "related")?
                .iter()
                .map(decode_subject)
                .collect::<Decode<Vec<_>>>()?;
            Ok(Finding::new(
                RuleCode::parse(str_field(item, "code")?).map_err(|e| e.to_string())?,
                Severity::parse(str_field(item, "severity")?).map_err(|e| e.to_string())?,
                decode_subject(field(item, "subject")?)?,
                str_field(item, "message")?,
            )
            .with_related(related))
        })
        .collect::<Decode<Vec<_>>>()?;
    let stored = Outcome::parse(str_field(&value, "outcome")?).map_err(|e| e.to_string())?;
    // `incomplete` is not stored; it is whatever reproduces the stored outcome.
    let report = [false, true]
        .into_iter()
        .map(|incomplete| {
            ValidationReport::new(
                findings.clone(),
                artifacts,
                relationships,
                incomplete,
                completed,
            )
        })
        .find(|report| report.outcome() == stored)
        .ok_or("the stored outcome does not follow from the stored findings")?;
    Ok((report, bool_field(&value, "strict")?))
}

pub(crate) fn report_metadata(
    report: &ValidationReport,
    strict: bool,
    subject: &str,
    revision: Option<Revision>,
    digest: Option<&ContentDigest>,
    rules: &str,
) -> Result<Metadata> {
    let counts = report.counts();
    let mut m = MetaBuilder::evidence(EvidenceKind::ValidationReport)?;
    m.put("outcome", report.outcome().as_str())?
        .put("errors", int(counts.errors as u64)?)?
        .put("warnings", int(counts.warnings as u64)?)?
        .put("infos", int(counts.infos as u64)?)?
        .put("strict", strict)?
        .put("subject", subject)?
        .put("rules", rules)?
        .put("completed", report.completed().to_string())?;
    if let Some(revision) = revision {
        m.put("subject_revision", int(revision.get())?)?;
    }
    if let Some(digest) = digest {
        m.put("subject_sha256", digest.as_str())?;
    }
    Ok(m.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u128) -> ArtifactId {
        ArtifactId::from_bits(n)
    }

    fn ts(s: i64) -> Timestamp {
        Timestamp::from_unix_seconds(1_800_000_000 + s).unwrap()
    }

    fn intent() -> RunIntent {
        RunIntent {
            request_id: RequestId::new("req-1").unwrap(),
            provider: "local-process".to_owned(),
            provider_version: "1".to_owned(),
            workflow: id(1),
            inputs: BTreeMap::from([("a".to_owned(), id(2)), ("b".to_owned(), id(3))]),
            args: vec![
                "--x".to_owned(),
                "quote \" and \\ and \n newline".to_owned(),
            ],
            limits: ExecutionLimits::new(7, 99).unwrap(),
            env_names: vec!["LANG".to_owned()],
            program: ProgramProvenance {
                path: "C:\\tools\\tool.exe".to_owned(),
                digest: Some(sha256(b"tool")),
            },
            platform: "windows-x86_64".to_owned(),
            started: ts(0),
        }
    }

    #[test]
    fn sha256_matches_the_known_vectors() {
        assert_eq!(
            sha256(b"").as_str(),
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256(b"abc").as_str(),
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn an_intent_round_trips_exactly_and_deterministically() {
        let text = encode_intent(&intent());
        assert_eq!(decode_intent(&text).unwrap(), intent());
        assert_eq!(text, encode_intent(&intent()));
        let mut no_digest = intent();
        no_digest.program.digest = None;
        assert_eq!(
            decode_intent(&encode_intent(&no_digest)).unwrap(),
            no_digest
        );
    }

    #[test]
    fn damaged_intents_are_described() {
        let good = encode_intent(&intent());
        for (name, text) in [
            ("not json", "{".to_owned()),
            (
                "wrong schema",
                good.replace("functor.run/1", "functor.run/9"),
            ),
            (
                "missing field",
                good.replace("\"platform\"", "\"platformx\""),
            ),
            ("bad id", good.replace("art_", "xxx_")),
            (
                "bad limits",
                good.replace("\"timeout_seconds\": 7", "\"timeout_seconds\": 0"),
            ),
        ] {
            assert!(decode_intent(&text).is_err(), "{name}");
        }
    }

    fn result(outcome: RunOutcome) -> RunResult {
        let mut spec = RunResultSpec {
            outcome,
            finished: ts(5),
            exit_code: None,
            truncated: Truncation {
                stdout: true,
                stderr: false,
            },
            outputs: vec![],
            diagnostics: vec![],
        };
        if outcome == RunOutcome::Succeeded {
            spec.exit_code = Some(0);
            spec.outputs = vec![id(7)];
        }
        RunResult::new(id(10), ts(0), spec).unwrap()
    }

    #[test]
    fn a_result_round_trips() {
        let blob = |n: u64| BlobRef {
            digest: sha256(&n.to_le_bytes()),
            size: n,
        };
        let outputs = [OutputRef {
            artifact: id(7),
            name: "result".to_owned(),
            digest: sha256(b"x"),
            size: 1,
        }];
        let original = result(RunOutcome::Succeeded);
        let text = encode_result(&original, &blob(1), &blob(2), &outputs);
        let decoded = decode_result(&text, ts(0)).unwrap();
        assert_eq!(decoded.result, original);
        assert_eq!((decoded.stdout, decoded.stderr), (blob(1), blob(2)));

        let failed = result(RunOutcome::TimedOut);
        let text = encode_result(&failed, &blob(0), &blob(0), &[]);
        assert_eq!(decode_result(&text, ts(0)).unwrap().result, failed);
    }

    #[test]
    fn a_result_that_cannot_be_honest_is_refused_on_decode() {
        let blob = BlobRef {
            digest: sha256(b""),
            size: 0,
        };
        let text = encode_result(&result(RunOutcome::Succeeded), &blob, &blob, &[]);
        let forged = text.replace("\"exit_code\": 0", "\"exit_code\": 1");
        assert!(
            decode_result(&forged, ts(0)).is_err(),
            "a success with exit 1"
        );
        assert!(
            decode_result(&text, ts(100)).is_err(),
            "finished before the run started"
        );
    }

    #[test]
    fn a_report_round_trips_with_every_subject_kind() {
        let findings = vec![
            Finding::new(
                RuleCode::new("V0012"),
                Severity::Warning,
                Subject::Repository,
                "custom relationship type `related_to`",
            ),
            Finding::new(
                RuleCode::new("V0007"),
                Severity::Error,
                Subject::Relationship(RelationshipId::from_bits(4)),
                "target does not exist",
            )
            .with_related([
                Subject::Artifact(id(1)),
                Subject::File(RepoPath::new("a/b.toml").unwrap()),
            ]),
        ];
        for incomplete in [false, true] {
            let report = ValidationReport::new(findings.clone(), 3, 2, incomplete, ts(9));
            let text = encode_report(&report, true).unwrap();
            let (decoded, strict) = decode_report(&text, ts(9)).unwrap();
            assert_eq!(decoded, report);
            assert!(strict);
        }
        let unknown = ValidationReport::new(vec![], 0, 0, true, ts(1));
        assert_eq!(unknown.outcome(), Outcome::Unknown);
        let (back, _) = decode_report(&encode_report(&unknown, false).unwrap(), ts(1)).unwrap();
        assert_eq!(
            back, unknown,
            "an incomplete run without findings stays unknown"
        );
    }

    #[test]
    fn a_report_that_contradicts_its_findings_is_refused() {
        let report = ValidationReport::new(vec![], 0, 0, false, ts(1));
        let text = encode_report(&report, false)
            .unwrap()
            .replace("passed", "failed");
        assert!(decode_report(&text, ts(1)).is_err());
    }

    #[test]
    fn metadata_round_trips_a_capture() {
        let capture = Capture {
            artifact: id(5),
            source: id(6),
            source_revision: Revision::new(3).unwrap(),
            role: CaptureRole::Input,
            digest: sha256(b"x"),
            size: 1,
        };
        let metadata = capture_metadata(&capture).unwrap();
        let artifact = Artifact::create(
            id(5),
            functor_model::NewArtifact::new(
                functor_model::Slug::parse("c").unwrap(),
                EvidenceKind::Capture.to_artifact_type(),
                functor_model::Title::new("c").unwrap(),
            )
            .with_metadata(metadata),
            ts(0),
        );
        assert_eq!(EvidenceKind::of(&artifact), Some(EvidenceKind::Capture));
        assert_eq!(decode_capture(&artifact).unwrap(), capture);
    }
}
