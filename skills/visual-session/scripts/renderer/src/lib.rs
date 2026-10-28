use serde_json::{Map, Value};
use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

mod layout;

const TEMPLATE: &str = include_str!("../../../assets/dashboard.html");
const LIMIT: u64 = 1024 * 1024;
const HTML_LIMIT: u64 = 8 * LIMIT;
const STATUSES: [&str; 6] = [
    "planned", "running", "passed", "failed", "skipped", "unknown",
];
const BEGIN: &str = "<!-- visual-session:begin ";
const END: &str = "<!-- visual-session:end -->";
type Result<T> = std::result::Result<T, String>;

#[derive(Debug)]
struct Goal {
    id: String,
    title: String,
    status: String,
    evidence_ids: Vec<String>,
}

#[derive(Debug)]
struct Evidence {
    id: String,
    label: String,
    status: String,
    source: String,
    as_of: String,
    url: Option<String>,
}

#[derive(Debug)]
struct Blocker {
    title: String,
    status: String,
    detail: String,
}

#[derive(Debug)]
struct Suggestion {
    kind: String,
    title: String,
    detail: String,
}

#[derive(Debug)]
pub struct Session {
    id: String,
    revision: u64,
    title: String,
    summary: String,
    as_of: String,
    goals: Vec<Goal>,
    evidence: Vec<Evidence>,
    blockers: Vec<Blocker>,
    suggestions: Vec<Suggestion>,
    notes: Vec<String>,
}

fn object<'a>(
    v: &'a Value,
    required: &[&str],
    optional: &[&str],
    at: &str,
) -> Result<&'a Map<String, Value>> {
    let map = v
        .as_object()
        .ok_or_else(|| format!("{at}: expected object"))?;
    for name in required {
        if !map.contains_key(*name) {
            return Err(format!("{at}: missing field {name}"));
        }
    }
    if map
        .keys()
        .any(|key| !required.contains(&key.as_str()) && !optional.contains(&key.as_str()))
    {
        return Err(format!("{at}: unknown field"));
    }
    Ok(map)
}

fn string(v: &Value, at: &str) -> Result<String> {
    let value = v.as_str().ok_or_else(|| format!("{at}: expected string"))?;
    if value.trim().is_empty()
        || value.len() > 8000
        || value
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(format!(
            "{at}: string must be nonempty, at most 8000 bytes, and free of control characters"
        ));
    }
    Ok(value.to_owned())
}

fn array<'a>(v: &'a Value, at: &str) -> Result<&'a Vec<Value>> {
    let values = v
        .as_array()
        .ok_or_else(|| format!("{at}: expected array"))?;
    if values.len() > 100 {
        return Err(format!("{at}: at most 100 entries permitted"));
    }
    Ok(values)
}

fn slug(v: &Value, at: &str) -> Result<String> {
    let s = string(v, at)?;
    if s.len() > 80
        || !s
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        || s.starts_with('-')
        || s.ends_with('-')
    {
        return Err(format!(
            "{at}: expected lowercase alphanumeric slug, at most 80 bytes"
        ));
    }
    Ok(s)
}

fn unique(set: &mut HashSet<String>, id: &str, at: &str) -> Result<()> {
    if !set.insert(id.to_owned()) {
        return Err(format!("{at}: duplicate id"));
    }
    Ok(())
}

fn status(v: &Value, at: &str) -> Result<String> {
    let s = string(v, at)?;
    if !STATUSES.contains(&s.as_str()) {
        return Err(format!(
            "{at}: expected planned, running, passed, failed, skipped, or unknown"
        ));
    }
    Ok(s)
}

fn timestamp(v: &Value, at: &str) -> Result<String> {
    let s = string(v, at)?;
    let b = s.as_bytes();
    if b.len() != 20
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[19] != b'Z'
        || b.iter()
            .enumerate()
            .any(|(i, c)| ![4, 7, 10, 13, 16, 19].contains(&i) && !c.is_ascii_digit())
    {
        return Err(format!("{at}: expected UTC YYYY-MM-DDTHH:MM:SSZ"));
    }
    let n = |a: usize, z: usize| -> u32 { s[a..z].parse().expect("validated ASCII digits") };
    let (year, month, day, hour, minute, second) =
        (n(0, 4), n(5, 7), n(8, 10), n(11, 13), n(14, 16), n(17, 19));
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    if year == 0 || day == 0 || day > max_day || hour > 23 || minute > 59 || second > 59 {
        return Err(format!("{at}: invalid calendar date or time"));
    }
    Ok(s)
}

fn https_url(v: &Value) -> Result<String> {
    let s = string(v, "evidence.url")?;
    if !s.starts_with("https://")
        || s.chars()
            .any(|c| c.is_control() || c.is_whitespace() || "\\\"'<>`".contains(c))
    {
        return Err("evidence.url: only explicit safe https links permitted".into());
    }
    let authority = s[8..].split(['/', '?', '#']).next().unwrap_or_default();
    let mut parts = authority.split(':');
    let host = parts.next().unwrap_or_default();
    if let Some(port) = parts.next() {
        if port.is_empty()
            || port.parse::<u16>().ok().is_none_or(|p| p == 0)
            || parts.next().is_some()
        {
            return Err("evidence.url: invalid port".into());
        }
    }
    if host.is_empty()
        || host.len() > 253
        || host.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        })
    {
        return Err("evidence.url: expected ASCII DNS hostname without credentials".into());
    }
    Ok(s)
}

pub fn parse(input: &str) -> Result<Session> {
    if input.len() as u64 > LIMIT {
        return Err("state exceeds 1 MiB".into());
    }
    let v: Value = serde_json::from_str(input)
        .map_err(|e| format!("invalid JSON at line {}, column {}", e.line(), e.column()))?;
    let m = object(
        &v,
        &[
            "schema_version",
            "id",
            "revision",
            "title",
            "summary",
            "as_of",
            "goals",
            "evidence",
            "blockers",
            "suggestions",
            "notes",
        ],
        &[],
        "session",
    )?;
    if m["schema_version"].as_u64() != Some(1) {
        return Err("unsupported schema_version (expected 1)".into());
    }
    let revision = m["revision"]
        .as_u64()
        .filter(|n| *n >= 1)
        .ok_or("revision must be an integer >= 1")?;
    let as_of = timestamp(&m["as_of"], "as_of")?;
    let mut evidence = Vec::new();
    let mut evidence_ids = HashSet::new();
    for value in array(&m["evidence"], "evidence")? {
        let e = object(
            value,
            &["id", "label", "status", "source", "as_of"],
            &["url"],
            "evidence",
        )?;
        let id = slug(&e["id"], "evidence.id")?;
        unique(&mut evidence_ids, &id, "evidence")?;
        let time = timestamp(&e["as_of"], "evidence.as_of")?;
        if time > as_of {
            return Err("evidence.as_of must not be later than session.as_of".into());
        }
        evidence.push(Evidence {
            id,
            label: string(&e["label"], "evidence.label")?,
            status: status(&e["status"], "evidence.status")?,
            source: string(&e["source"], "evidence.source")?,
            as_of: time,
            url: e.get("url").map(https_url).transpose()?,
        });
    }
    let mut goals = Vec::new();
    let mut goal_ids = HashSet::new();
    for value in array(&m["goals"], "goals")? {
        let g = object(
            value,
            &["id", "title", "status", "evidence_ids"],
            &[],
            "goal",
        )?;
        let id = slug(&g["id"], "goal.id")?;
        unique(&mut goal_ids, &id, "goals")?;
        let mut refs = Vec::new();
        let mut seen = HashSet::new();
        for item in array(&g["evidence_ids"], "goal.evidence_ids")? {
            let id = slug(item, "goal.evidence_ids")?;
            if !evidence_ids.contains(&id) {
                return Err("goal references missing evidence id".into());
            }
            unique(&mut seen, &id, "goal.evidence_ids")?;
            refs.push(id);
        }
        let state = status(&g["status"], "goal.status")?;
        if state == "passed"
            && !evidence
                .iter()
                .any(|e| refs.contains(&e.id) && e.status == "passed")
        {
            return Err("passed goals require at least one referenced passed evidence item".into());
        }
        goals.push(Goal {
            id,
            title: string(&g["title"], "goal.title")?,
            status: state,
            evidence_ids: refs,
        });
    }
    let mut blockers = Vec::new();
    let mut blocker_ids = HashSet::new();
    for value in array(&m["blockers"], "blockers")? {
        let b = object(value, &["id", "title", "status", "detail"], &[], "blocker")?;
        unique(&mut blocker_ids, &slug(&b["id"], "blocker.id")?, "blockers")?;
        blockers.push(Blocker {
            title: string(&b["title"], "blocker.title")?,
            status: status(&b["status"], "blocker.status")?,
            detail: string(&b["detail"], "blocker.detail")?,
        });
    }
    let mut suggestions = Vec::new();
    for value in array(&m["suggestions"], "suggestions")? {
        let s = object(value, &["kind", "title", "detail"], &[], "suggestion")?;
        let kind = string(&s["kind"], "suggestion.kind")?;
        if !["prompt", "skill", "context", "loop"].contains(&kind.as_str()) {
            return Err("suggestion.kind must be prompt, skill, context, or loop".into());
        }
        suggestions.push(Suggestion {
            kind,
            title: string(&s["title"], "suggestion.title")?,
            detail: string(&s["detail"], "suggestion.detail")?,
        });
    }
    let notes = array(&m["notes"], "notes")?
        .iter()
        .map(|n| string(n, "note"))
        .collect::<Result<Vec<_>>>()?;
    Ok(Session {
        id: slug(&m["id"], "id")?,
        revision,
        title: string(&m["title"], "title")?,
        summary: string(&m["summary"], "summary")?,
        as_of,
        goals,
        evidence,
        blockers,
        suggestions,
        notes,
    })
}

pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn badge(status: &str) -> String {
    format!("<span class=\"badge s-{status}\">{status}</span>")
}

fn legacy_content(s: &Session) -> String {
    let mut html = String::new();
    let passed = s.goals.iter().filter(|g| g.status == "passed").count();
    let _ = write!(html, "<div class=\"session-content\"><header class=\"hero\"><p class=\"eyebrow\">VISUAL SESSION <span>LOCAL SNAPSHOT · REVISION {}</span></p><h1>{}</h1><p class=\"summary\">{}</p><p class=\"as-of\">As of <time datetime=\"{}\">{}</time> · {}</p></header><section class=\"overview\" aria-label=\"Goal status overview\"><div class=\"completion\"><span class=\"eyebrow\">GOALS WITH PASSED EVIDENCE</span><p><strong>{}</strong><span> / {}</span></p><small>{}</small></div><div class=\"status-chart\" role=\"group\" aria-label=\"Goal counts by status\">", s.revision, escape(&s.title), escape(&s.summary), s.as_of, s.as_of, escape(&s.id), passed, s.goals.len(), if s.goals.is_empty() { "No goals recorded yet" } else { "Counts reflect recorded states, not effort or time" });
    for state in STATUSES {
        let count = s.goals.iter().filter(|g| g.status == state).count();
        let width = if s.goals.is_empty() {
            0
        } else {
            count * 100 / s.goals.len()
        };
        let _ = write!(html, "<div class=\"status-row\"><span>{state}</span><strong>{count}</strong><span class=\"bar s-{state}\"><i style=\"width:{width}%\"></i></span></div>");
    }
    html.push_str("</div></section><div class=\"columns\"><div class=\"primary\"><section class=\"panel\" id=\"goals\"><div class=\"section-heading\"><h2>Goals &amp; progress</h2><span>Evidence before confidence</span></div><ol class=\"goal-list\">");
    for goal in &s.goals {
        let _ = write!(html, "<li class=\"goal\" id=\"goal-{}\"><div class=\"goal-top\"><h3>{}</h3>{}</div><p class=\"evidence-refs\">", goal.id, escape(&goal.title), badge(&goal.status));
        if goal.evidence_ids.is_empty() {
            html.push_str("No evidence linked");
        }
        for (index, id) in goal.evidence_ids.iter().enumerate() {
            if index > 0 {
                html.push_str(" · ");
            }
            let label = &s
                .evidence
                .iter()
                .find(|e| e.id == *id)
                .expect("validated evidence reference")
                .label;
            let _ = write!(html, "<a href=\"#evidence-{id}\">{}</a>", escape(label));
        }
        html.push_str("</p></li>");
    }
    if s.goals.is_empty() {
        html.push_str("<li class=\"empty\">Add a concrete goal when scope is known.</li>");
    }
    html.push_str("</ol></section><section class=\"panel\" id=\"evidence\"><div class=\"section-heading\"><h2>Evidence ledger</h2><span>Source + observation time</span></div><div class=\"evidence-list\">");
    for e in &s.evidence {
        let _ = write!(html, "<article class=\"evidence\" id=\"evidence-{}\"><div class=\"goal-top\"><h3>{}</h3>{}</div><p>{}</p><p class=\"meta\"><time datetime=\"{}\">{}</time>", e.id, escape(&e.label), badge(&e.status), escape(&e.source), e.as_of, e.as_of);
        if let Some(url) = &e.url {
            let _ = write!(
                html,
                " · <a href=\"{}\" rel=\"noreferrer noopener\">Open evidence ↗</a>",
                escape(url)
            );
        }
        html.push_str("</p></article>");
    }
    if s.evidence.is_empty() {
        html.push_str(
            "<p class=\"empty\">No observations recorded. Unknown is an honest starting point.</p>",
        );
    }
    html.push_str("</div></section></div><aside class=\"secondary\"><section class=\"panel\"><div class=\"section-heading\"><h2>Blockers</h2><span>What needs attention</span></div>");
    for b in &s.blockers {
        let _ = write!(html, "<article class=\"blocker\"><div class=\"goal-top\"><h3>{}</h3>{}</div><p>{}</p></article>", escape(&b.title), badge(&b.status), escape(&b.detail));
    }
    if s.blockers.is_empty() {
        html.push_str(
            "<p class=\"empty\">No blockers recorded. This does not prove there are none.</p>",
        );
    }
    html.push_str("</section><section class=\"panel\"><div class=\"section-heading\"><h2>Suggested next moves</h2></div><p class=\"meta\">Suggestions only. Review against the current goal and authorization.</p>");
    for suggestion in &s.suggestions {
        let _ = write!(html, "<article class=\"suggestion\"><span class=\"eyebrow\">{} SUGGESTION</span><h3>{}</h3><p>{}</p></article>", suggestion.kind.to_uppercase(), escape(&suggestion.title), escape(&suggestion.detail));
    }
    if s.suggestions.is_empty() {
        html.push_str("<p class=\"empty\">No next moves suggested yet.</p>");
    }
    html.push_str("</section></aside></div><section class=\"panel notes\"><div class=\"section-heading\"><h2>Session notes</h2></div><ul>");
    for note in &s.notes {
        let _ = write!(html, "<li>{}</li>", escape(note));
    }
    if s.notes.is_empty() {
        html.push_str("<li class=\"empty\">No session notes recorded.</li>");
    }
    html.push_str("</ul></section><footer class=\"snapshot-footer\">A local snapshot, not a live monitor. Refresh from verified evidence when the session changes.</footer></div>\n");
    html
}

fn checksum(s: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in s.bytes() {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn managed(s: &Session, layout2: bool) -> String {
    let body = format!(
        "\n{}",
        if layout2 {
            layout::content(s)
        } else {
            legacy_content(s)
        }
    );
    format!(
        "{BEGIN}revision={} checksum={} -->{body}{END}",
        s.revision,
        checksum(&body)
    )
}

pub fn render(s: &Session, previous: Option<&str>) -> Result<String> {
    let layout2 = previous
        .is_none_or(|html| html.contains("<meta name=\"visual-session-layout\" content=\"2\">"));
    let replacement = managed(s, layout2);
    let Some(previous) = previous else {
        // Expand only tokens from the trusted template, never tokens introduced by data.
        let mut parts = TEMPLATE.split("{{");
        let mut result = parts.next().unwrap_or_default().to_owned();
        for part in parts {
            if let Some(tail) = part.strip_prefix("TITLE}}") {
                result.push_str(&escape(&s.title));
                result.push_str(tail);
            } else if let Some(tail) = part.strip_prefix("SESSION_ID}}") {
                result.push_str(&s.id);
                result.push_str(tail);
            } else if let Some(tail) = part.strip_prefix("CONTENT}}") {
                result.push_str(&replacement);
                result.push_str(tail);
            } else {
                result.push_str("{{");
                result.push_str(part);
            }
        }
        return Ok(result);
    };
    let identity = format!("<meta name=\"visual-session-id\" content=\"{}\">", s.id);
    if previous.matches("name=\"visual-session-id\"").count() != 1 || !previous.contains(&identity)
    {
        return Err("existing output has a different or missing artifact identity; choose a new output path".into());
    }
    if previous.matches(BEGIN).count() != 1 || previous.matches(END).count() != 1 {
        return Err("existing output must contain exactly one managed region".into());
    }
    let start = previous.find(BEGIN).expect("one begin marker");
    let end = previous.find(END).expect("one end marker");
    let header_end = previous[start..]
        .find(" -->")
        .map(|n| start + n + 4)
        .ok_or("invalid managed marker")?;
    if header_end > end {
        return Err("invalid managed region order".into());
    }
    let header = &previous[start + BEGIN.len()..header_end - 4];
    let (revision, hash) = header
        .strip_prefix("revision=")
        .and_then(|h| h.split_once(" checksum="))
        .ok_or("invalid managed metadata")?;
    let old_revision = revision
        .parse::<u64>()
        .map_err(|_| "invalid managed revision")?;
    if hash != checksum(&previous[header_end..end]) {
        return Err("managed region was edited; preserve those edits in state or outside the region before updating".into());
    }
    let old = &previous[start..end + END.len()];
    if s.revision < old_revision || (s.revision == old_revision && old != replacement) {
        return Err("revision is stale or unchanged with different content; increment revision before updating".into());
    }
    let mut result = previous[..start].to_owned();
    result.push_str(&replacement);
    result.push_str(&previous[end + END.len()..]);
    Ok(result)
}

fn read_state(path: &Path) -> Result<Session> {
    let mut data = String::new();
    fs::File::open(path)
        .map_err(|_| "could not open state file")?
        .take(LIMIT + 1)
        .read_to_string(&mut data)
        .map_err(|_| "could not read UTF-8 state file")?;
    parse(&data)
}

struct Lock(PathBuf);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn private_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}

fn write_output(state: &Session, input: &Path, output: &Path) -> Result<()> {
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let canonical_parent =
        fs::canonicalize(parent).map_err(|_| "output parent must already exist")?;
    let filename = output.file_name().ok_or("output must have a filename")?;
    let output = canonical_parent.join(filename);
    if fs::canonicalize(input).map_err(|_| "could not resolve state file")? == output {
        return Err("state and output must be different files".into());
    }
    let lock_path = output.with_file_name(format!(
        "{}.visual-session.lock",
        filename.to_string_lossy()
    ));
    private_options().open(&lock_path).map_err(|_| {
        "output is locked; inspect the existing .visual-session.lock before retrying"
    })?;
    let _lock = Lock(lock_path);
    let previous = match fs::symlink_metadata(&output) {
        Ok(meta) if meta.file_type().is_symlink() || !meta.is_file() => {
            return Err("output must be a regular file, never a symlink".into())
        }
        Ok(meta) if meta.len() > HTML_LIMIT => return Err("existing output exceeds 8 MiB".into()),
        Ok(_) => Some(fs::read_to_string(&output).map_err(|_| "could not read existing output")?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err("could not inspect output".into()),
    };
    let html = render(state, previous.as_deref())?;
    if html.len() as u64 > HTML_LIMIT {
        return Err(
            "rendered output exceeds 8 MiB; shorten labels or reduce repeated evidence references"
                .into(),
        );
    }
    if previous.as_deref() == Some(&html) {
        return Ok(());
    }
    let temp = output.with_file_name(format!("{}.visual-session.tmp", filename.to_string_lossy()));
    let mut file = private_options()
        .open(&temp)
        .map_err(|_| "temporary output already exists or cannot be created")?;
    let saved = file
        .write_all(html.as_bytes())
        .and_then(|()| file.sync_all());
    drop(file);
    let saved = saved.and_then(|()| fs::rename(&temp, &output));
    if saved.is_err() {
        let _ = fs::remove_file(&temp);
        return Err("could not atomically save dashboard".into());
    }
    Ok(())
}

pub fn run(args: Vec<String>) -> Result<()> {
    match args.as_slice() {
        [command, state] if command == "validate" => {
            read_state(Path::new(state))?;
            println!("valid session state (schema 1)");
            Ok(())
        }
        [command, state, output] if command == "render" => {
            let session = read_state(Path::new(state))?;
            write_output(&session, Path::new(state), Path::new(output))?;
            println!("dashboard rendered (revision {})", session.revision);
            Ok(())
        }
        _ => Err("usage: visual-session validate STATE.json | visual-session render STATE.json OUTPUT.html".into()),
    }
}
