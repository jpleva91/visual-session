use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "visual-session-tests-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn state(&self, state: &Value) -> PathBuf {
        let path = self.0.join("session.json");
        fs::write(&path, serde_json::to_vec_pretty(state).unwrap()).unwrap();
        path
    }

    fn render(&self, state: &Value) -> Output {
        Command::new(env!("CARGO_BIN_EXE_visual-session"))
            .arg("render")
            .arg(self.state(state))
            .arg(self.0.join("dashboard.html"))
            .output()
            .unwrap()
    }

    fn validate(&self, state: &Value) -> Output {
        Command::new(env!("CARGO_BIN_EXE_visual-session"))
            .arg("validate")
            .arg(self.state(state))
            .output()
            .unwrap()
    }

    fn html(&self) -> String {
        fs::read_to_string(self.0.join("dashboard.html")).unwrap()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture() -> Value {
    json!({
        "schema_version": 1,
        "id": "sample-session",
        "revision": 1,
        "title": "Map a small garden",
        "summary": "A synthetic session with one checked observation.",
        "as_of": "2026-10-28T01:00:00Z",
        "goals": [{
            "id": "garden-plan",
            "title": "Check the layout",
            "status": "passed",
            "evidence_ids": ["layout-check"]
        }],
        "evidence": [{
            "id": "layout-check",
            "label": "Synthetic layout check",
            "status": "passed",
            "source": "Synthetic fixture, manually inspected",
            "as_of": "2026-10-28T00:59:00Z",
            "url": "https://example.org/evidence"
        }],
        "blockers": [{
            "id": "soil-choice",
            "title": "Choose a soil mix",
            "status": "unknown",
            "detail": "A synthetic unresolved choice."
        }],
        "suggestions": [
            {"kind":"prompt", "title":"Next prompt", "detail":"Compare two generic options."},
            {"kind":"skill", "title":"Optional skill", "detail":"Consider a planning skill if available."},
            {"kind":"context", "title":"Keep context small", "detail":"Carry goals and evidence into the next turn."},
            {"kind":"loop", "title":"Suggested loop", "detail":"Observe, update, review, then ask for the next goal."}
        ],
        "notes": ["This fixture is fully synthetic."]
    })
}

fn assert_ok(output: Output) {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_rejected(output: Output) {
    assert!(
        !output.status.success(),
        "unexpected success: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn validates_and_renders_deterministically() {
    let one = Sandbox::new();
    let two = Sandbox::new();
    assert_ok(one.validate(&fixture()));
    assert_ok(one.render(&fixture()));
    assert_ok(two.render(&fixture()));
    assert_eq!(one.html(), two.html());
    let html = one.html();
    assert!(html.contains("Map a small garden"));
    assert!(html.contains("2026-10-28T00:59:00Z"));
    assert!(html.contains("Synthetic fixture, manually inspected"));
    for status in [
        "planned", "running", "passed", "failed", "skipped", "unknown",
    ] {
        assert!(html.contains(status), "missing status {status}");
    }
    assert_ok(one.render(&fixture()));
    assert_eq!(html, one.html(), "identical rerender must be a no-op");
}

#[test]
fn updates_in_place_and_preserves_user_content_outside_managed_region() {
    let sandbox = Sandbox::new();
    assert_ok(sandbox.render(&fixture()));
    let html = sandbox.html().replace(
        "</body>",
        "<aside>My own persistent annotation</aside>\n</body>",
    );
    fs::write(sandbox.0.join("dashboard.html"), html).unwrap();
    let mut state = fixture();
    state["revision"] = json!(2);
    state["summary"] = json!("Updated after a second synthetic observation.");
    state["as_of"] = json!("2026-10-28T02:00:00Z");
    assert_ok(sandbox.render(&state));
    let updated = sandbox.html();
    assert!(updated.contains("My own persistent annotation"));
    assert!(updated.contains("Updated after a second synthetic observation."));
    assert!(!updated.contains("A synthetic session with one checked observation."));
}

#[test]
fn refuses_managed_edits_without_changing_the_file() {
    let sandbox = Sandbox::new();
    assert_ok(sandbox.render(&fixture()));
    let edited = sandbox
        .html()
        .replace("Check the layout", "A user-edited goal");
    fs::write(sandbox.0.join("dashboard.html"), &edited).unwrap();
    let mut state = fixture();
    state["revision"] = json!(2);
    assert_rejected(sandbox.render(&state));
    assert_eq!(edited, sandbox.html());
}

#[test]
fn refuses_foreign_identity_stale_revision_and_changed_same_revision() {
    let sandbox = Sandbox::new();
    let mut state = fixture();
    state["revision"] = json!(2);
    assert_ok(sandbox.render(&state));
    let original = sandbox.html();
    state["id"] = json!("another-session");
    assert_rejected(sandbox.render(&state));
    state["id"] = json!("sample-session");
    state["revision"] = json!(1);
    assert_rejected(sandbox.render(&state));
    state["revision"] = json!(2);
    state["summary"] = json!("Same revision, different contents.");
    assert_rejected(sandbox.render(&state));
    assert_eq!(original, sandbox.html());
}

#[test]
fn refuses_to_overwrite_an_unrelated_html_file() {
    let sandbox = Sandbox::new();
    let unrelated = "<!doctype html><title>My own document</title>";
    fs::write(sandbox.0.join("dashboard.html"), unrelated).unwrap();
    assert_rejected(sandbox.render(&fixture()));
    assert_eq!(unrelated, sandbox.html());
}

#[test]
fn escapes_untrusted_session_strings_as_text() {
    let sandbox = Sandbox::new();
    let mut state = fixture();
    let payload = "<script>alert('sample')</script> & \"quoted\"";
    state["title"] = json!(payload);
    state["summary"] = json!(payload);
    state["goals"][0]["title"] = json!(payload);
    state["evidence"][0]["label"] = json!(payload);
    state["evidence"][0]["source"] = json!(payload);
    state["blockers"][0]["detail"] = json!(payload);
    state["suggestions"][0]["detail"] = json!(payload);
    state["notes"][0] = json!(payload);
    assert_ok(sandbox.render(&state));
    let html = sandbox.html();
    assert!(!html.contains("<script>alert"));
    assert!(html.contains("&lt;script&gt;"));
    assert!(html.contains("&amp;"));
    assert!(!html.contains("<script"));
}

#[test]
fn rejects_unsafe_evidence_links() {
    let sandbox = Sandbox::new();
    for link in [
        "javascript:alert(1)",
        "data:text/html,<script>alert(1)</script>",
        "file:///some/local/path",
        "http://example.org/evidence",
        "//example.org/evidence",
        "https://person:password@example.org/evidence",
        "https://example.org/\"onclick=alert(1)",
        "https://example.org/<script>",
        "https://example.org/with space",
        "https://example.org\\other",
        "https://example.org/\nother",
        "https:///missing-host",
    ] {
        let mut state = fixture();
        state["evidence"][0]["url"] = json!(link);
        assert_rejected(sandbox.validate(&state));
    }
}

#[test]
fn rejects_invalid_contracts_and_dangling_evidence() {
    let sandbox = Sandbox::new();
    let mut cases = Vec::new();
    let mut state = fixture();
    state["unexpected"] = json!(true);
    cases.push(state);
    let mut state = fixture();
    state["goals"][0]["unexpected"] = json!(true);
    cases.push(state);
    let mut state = fixture();
    state["schema_version"] = json!(2);
    cases.push(state);
    let mut state = fixture();
    state["revision"] = json!(0);
    cases.push(state);
    let mut state = fixture();
    state["title"] = json!("");
    cases.push(state);
    let mut state = fixture();
    state["id"] = json!("Not a slug");
    cases.push(state);
    let mut state = fixture();
    state["goals"][0]["status"] = json!("done");
    cases.push(state);
    let mut state = fixture();
    state["goals"][0]["evidence_ids"] = json!(["missing-evidence"]);
    cases.push(state);
    let mut state = fixture();
    state["goals"][0]["evidence_ids"] = json!([]);
    cases.push(state);
    let mut state = fixture();
    state["evidence"][0]["status"] = json!("planned");
    cases.push(state);
    let mut state = fixture();
    state["evidence"]
        .as_array_mut()
        .unwrap()
        .push(fixture()["evidence"][0].clone());
    cases.push(state);
    let mut state = fixture();
    state["evidence"][0]["as_of"] = json!("2026-10-28T01:01:00Z");
    cases.push(state);
    let mut state = fixture();
    state.as_object_mut().unwrap().remove("notes");
    cases.push(state);
    let mut state = fixture();
    state["notes"] = json!(["x".repeat(8001)]);
    cases.push(state);
    for state in cases {
        assert_rejected(sandbox.validate(&state));
    }
}

#[test]
fn validates_calendar_dates_and_requires_exact_utc_timestamps() {
    let sandbox = Sandbox::new();
    for timestamp in [
        "2026-02-29T01:00:00Z",
        "2026-04-31T01:00:00Z",
        "2026-13-01T01:00:00Z",
        "2026-10-28T24:00:00Z",
        "2026-10-28T01:60:00Z",
        "2026-10-28T01:00:60Z",
        "2026-10-28T01:00:00+00:00",
        "2026-10-28T01:00:00.000Z",
        "2026-10-28",
    ] {
        let mut state = fixture();
        state["as_of"] = json!(timestamp);
        assert_rejected(sandbox.validate(&state));
    }
    let mut leap_year = fixture();
    leap_year["as_of"] = json!("2028-02-29T01:00:00Z");
    assert_ok(sandbox.validate(&leap_year));
}

#[test]
fn allows_explicit_empty_sections_without_implying_success() {
    let sandbox = Sandbox::new();
    let mut state = fixture();
    for field in ["goals", "evidence", "blockers", "suggestions", "notes"] {
        state[field] = json!([]);
    }
    assert_ok(sandbox.render(&state));
    let html = sandbox.html();
    assert!(html.contains("No goals recorded yet"));
    assert!(!html.contains("width:100%"));
}

#[test]
fn user_template_tokens_remain_literal_and_do_not_create_extra_regions() {
    let sandbox = Sandbox::new();
    let mut state = fixture();
    state["title"] = json!("Literal {{CONTENT}} and {{SESSION_ID}} and {{TITLE}}");
    state["summary"] = json!("Summary {{CONTENT}} and {{SESSION_ID}} and {{TITLE}}");
    assert_ok(sandbox.render(&state));
    let html = sandbox.html();
    assert!(html.contains("Literal {{CONTENT}} and {{SESSION_ID}} and {{TITLE}}"));
    assert!(html.contains("Summary {{CONTENT}} and {{SESSION_ID}} and {{TITLE}}"));
    assert_eq!(html.matches("<!-- visual-session:begin ").count(), 1);
    assert_eq!(html.matches("<!-- visual-session:end -->").count(), 1);
    state["revision"] = json!(2);
    assert_ok(sandbox.render(&state));
}

#[test]
fn existing_lock_blocks_update_and_preserves_all_files() {
    let sandbox = Sandbox::new();
    assert_ok(sandbox.render(&fixture()));
    let original = sandbox.html();
    let lock = sandbox.0.join("dashboard.html.visual-session.lock");
    fs::write(&lock, "synthetic competing renderer").unwrap();
    let mut state = fixture();
    state["revision"] = json!(2);
    assert_rejected(sandbox.render(&state));
    assert_eq!(original, sandbox.html());
    assert_eq!(
        fs::read_to_string(lock).unwrap(),
        "synthetic competing renderer"
    );
}

#[cfg(unix)]
#[test]
fn creates_private_output_and_refuses_symlink_output() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let sandbox = Sandbox::new();
    assert_ok(sandbox.render(&fixture()));
    let output = sandbox.0.join("dashboard.html");
    assert_eq!(
        fs::metadata(&output).unwrap().permissions().mode() & 0o777,
        0o600
    );
    fs::remove_file(&output).unwrap();
    let target = sandbox.0.join("user-file.html");
    fs::write(&target, "untouched").unwrap();
    symlink(&target, &output).unwrap();
    assert_rejected(sandbox.render(&fixture()));
    assert_eq!(fs::read_to_string(target).unwrap(), "untouched");
}

#[test]
fn rejects_oversized_input_before_rendering() {
    let sandbox = Sandbox::new();
    let path = sandbox.0.join("session.json");
    fs::write(&path, vec![b' '; 1024 * 1024 + 1]).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_visual-session"))
        .arg("validate")
        .arg(path)
        .output()
        .unwrap();
    assert_rejected(output);
}

#[test]
fn rejects_amplified_output_without_creating_or_overwriting_dashboard() {
    let sandbox = Sandbox::new();
    let mut amplified = fixture();
    let evidence_ids: Vec<String> = (0..100)
        .map(|index| format!("observation-{index}"))
        .collect();
    amplified["evidence"] = Value::Array(
        evidence_ids
            .iter()
            .map(|id| {
                json!({
                    "id": id,
                    "label": "x".repeat(1000),
                    "status": "passed",
                    "source": "Synthetic amplification fixture",
                    "as_of": "2026-10-28T00:59:00Z"
                })
            })
            .collect(),
    );
    amplified["goals"] = Value::Array(
        (0..100)
            .map(|index| {
                json!({
                    "id": format!("goal-{index}"),
                    "title": "A synthetic goal with many shared observations",
                    "status": "passed",
                    "evidence_ids": evidence_ids
                })
            })
            .collect(),
    );
    assert!(serde_json::to_vec_pretty(&amplified).unwrap().len() <= 1024 * 1024);
    assert_ok(sandbox.validate(&amplified));
    assert_rejected(sandbox.render(&amplified));
    assert!(!sandbox.0.join("dashboard.html").exists());

    assert_ok(sandbox.render(&fixture()));
    let original = sandbox.html();
    amplified["revision"] = json!(2);
    assert_rejected(sandbox.render(&amplified));
    assert_eq!(original, sandbox.html());
}
