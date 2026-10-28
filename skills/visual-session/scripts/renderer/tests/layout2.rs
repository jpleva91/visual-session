use serde_json::{json, Value};
use visual_session::{parse, render};

const LEGACY_HTML: &str = include_str!("fixtures/legacy.html");
const LEGACY_STATE: &str = r#"
{
  "schema_version": 1,
  "id": "seed-guide-demo",
  "revision": 1,
  "title": "The seed library field guide",
  "summary": "A fully synthetic session: turn a small collection of planting notes into an accessible, printable guide. The structure is proven; the illustrations and print checks need another pass.",
  "as_of": "2026-01-15T14:30:00Z",
  "goals": [
    {"id": "structure", "title": "Make the guide easy to navigate", "status": "passed", "evidence_ids": ["outline-review"]},
    {"id": "illustrations", "title": "Draw three simple planting diagrams", "status": "running", "evidence_ids": ["drawing-check"]},
    {"id": "print", "title": "Meet the print contrast target", "status": "failed", "evidence_ids": ["print-check"]},
    {"id": "reading", "title": "Check the reading order on a small screen", "status": "planned", "evidence_ids": []},
    {"id": "translation", "title": "Translate the guide after the first edition", "status": "skipped", "evidence_ids": ["scope-decision"]},
    {"id": "paper", "title": "Confirm the recycled-paper print result", "status": "unknown", "evidence_ids": []}
  ],
  "evidence": [
    {"id": "outline-review", "label": "Outline has one clear path from seed to harvest", "status": "passed", "source": "Synthetic outline walkthrough, fixture A", "as_of": "2026-01-15T13:40:00Z"},
    {"id": "drawing-check", "label": "Two diagrams drafted; the third is still in progress", "status": "running", "source": "Synthetic drawing inventory, fixture B", "as_of": "2026-01-15T14:20:00Z"},
    {"id": "print-check", "label": "Small caption contrast did not meet the chosen target", "status": "failed", "source": "Synthetic print check, fixture C; no actual measurement", "as_of": "2026-01-15T14:10:00Z"},
    {"id": "scope-decision", "label": "Translation intentionally deferred to the next edition", "status": "skipped", "source": "Synthetic scope decision, fixture D", "as_of": "2026-01-15T13:00:00Z", "url": "https://example.com/synthetic-scope"}
  ],
  "blockers": [
    {"id": "caption-contrast", "title": "The smallest captions need a darker ink", "status": "failed", "detail": "The synthetic print check failed. Adjust the caption treatment, then run the same check again before calling the print goal passed."},
    {"id": "paper-sample", "title": "No paper sample is available yet", "status": "unknown", "detail": "There is no observed result for the paper test. Leave the goal unknown until a sample is checked."}
  ],
  "suggestions": [
    {"kind": "prompt", "title": "Resolve the smallest unproven step", "detail": "Suggested prompt: Darken the small captions, repeat the print check, and attach the observed result. Keep the other goals unchanged."},
    {"kind": "skill", "title": "Consider an accessibility review", "detail": "If an accessibility skill is available, use it for the reading-order goal after the diagrams settle. This is a suggestion, not an invocation."},
    {"kind": "context", "title": "Carry the decision, not the whole conversation", "detail": "Retain the edition scope, caption requirement, evidence identifiers, and unresolved paper test in the next handoff. Omit draft chatter and unrelated history."},
    {"kind": "loop", "title": "One adjustment → one observed check", "detail": "Suggested loop: change captions → run the print check → record evidence → update this page. Stop after one attempt if the check cannot run; report the blocker instead of inferring success."}
  ],
  "notes": [
    "Every name, event, decision, and result in this demo is synthetic.",
    "Passed / total counts describe goals with supporting evidence. They are not a prediction of remaining effort.",
    "This is a snapshot at the displayed time. An earlier result does not prove that later changes pass."
  ]
}
"#;

fn fixture() -> Value {
    let statuses = [
        "planned", "running", "passed", "failed", "skipped", "unknown",
    ];
    json!({
        "schema_version": 1,
        "id": "layout-demo",
        "revision": 1,
        "title": "A synthetic layout session",
        "summary": "All observations and suggestions are synthetic.",
        "as_of": "2026-01-15T15:00:00Z",
        "goals": statuses.iter().map(|status| json!({
            "id": format!("goal-{status}"),
            "title": format!("A {status} goal"),
            "status": status,
            "evidence_ids": if *status == "passed" { vec!["e-new"] } else { vec![] }
        })).collect::<Vec<_>>(),
        "evidence": [
            {"id":"e-old", "label":"Earlier observation", "status":"unknown", "source":"Synthetic source A", "as_of":"2026-01-15T13:00:00Z"},
            {"id":"e-tie-a", "label":"First same-time observation", "status":"running", "source":"Synthetic source B", "as_of":"2026-01-15T14:00:00Z"},
            {"id":"e-new", "label":"Most recent observation", "status":"passed", "source":"Synthetic source C", "as_of":"2026-01-15T14:50:00Z"},
            {"id":"e-tie-b", "label":"Second same-time observation", "status":"failed", "source":"Synthetic source D", "as_of":"2026-01-15T14:00:00Z"}
        ],
        "blockers": [],
        "suggestions": [
            {"kind":"context", "title":"Context handoff suggestion", "detail":"Carry only the current goal and evidence."},
            {"kind":"prompt", "title":"First prompt suggestion", "detail":"Inspect the smallest uncertain observation."},
            {"kind":"loop", "title":"Bounded loop suggestion", "detail":"Observe once, record the result, then review."},
            {"kind":"skill", "title":"Optional skill suggestion", "detail":"Consider a relevant skill if available."},
            {"kind":"prompt", "title":"Second prompt suggestion", "detail":"Ask which remaining question matters most."}
        ],
        "notes": ["A fully synthetic test session."]
    })
}

fn html(state: &Value) -> String {
    render(&parse(&state.to_string()).unwrap(), None).unwrap()
}

#[test]
fn new_dashboards_advertise_layout_two() {
    let html = html(&fixture());
    assert!(html.contains("<meta name=\"visual-session-layout\" content=\"2\">"));
}

#[test]
fn features_first_prompt_before_goals_and_preserves_each_suggestion_once() {
    let html = html(&fixture());
    let first_prompt = html.find("First prompt suggestion").unwrap();
    let goals = html.find("id=\"goals\"").unwrap();
    assert!(first_prompt < goals);
    for title in [
        "First prompt suggestion",
        "Second prompt suggestion",
        "Context handoff suggestion",
        "Bounded loop suggestion",
        "Optional skill suggestion",
    ] {
        assert_eq!(
            html.matches(title).count(),
            1,
            "lost or duplicated: {title}"
        );
    }
    assert!(html.contains("Carry only the current goal and evidence."));
    assert!(html.contains("Observe once, record the result, then review."));
    assert!(html.contains("Consider a relevant skill if available."));
}

#[test]
fn overview_dom_order_prioritizes_the_next_move_then_goals_then_status_and_attention() {
    let html = html(&fixture());
    let wrapper = "<div class=\"session-overview\">";
    assert_eq!(html.matches(wrapper).count(), 1);
    assert!(!html.contains("class=\"launch-grid\""));
    assert!(!html.contains("class=\"work-grid\""));
    let start = html.find(wrapper).unwrap();
    let mut depth = 0;
    let mut end = None;
    for (offset, _) in html[start..].match_indices('<') {
        let tag = &html[start + offset..];
        if tag.starts_with("<div ") || tag.starts_with("<div>") {
            depth += 1;
        } else if tag.starts_with("</div>") {
            depth -= 1;
            if depth == 0 {
                end = Some(start + offset);
                break;
            }
        }
    }
    let overview = &html[start..end.expect("overview wrapper must close")];
    let positions: Vec<_> = [
        "<section class=\"next-move\"",
        "<section class=\"trajectory\" id=\"goals\"",
        "<section class=\"status-map\"",
        "<section class=\"attention\"",
    ]
    .iter()
    .map(|section| {
        overview
            .find(section)
            .expect("section must be inside overview")
    })
    .collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn ledger_is_newest_first_with_stable_input_order_for_ties() {
    let html = html(&fixture());
    let positions: Vec<_> = ["e-new", "e-tie-a", "e-tie-b", "e-old"]
        .iter()
        .map(|id| html.find(&format!("id=\"evidence-{id}\"")).unwrap())
        .collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
}

fn goal_item<'a>(html: &'a str, id: &str) -> &'a str {
    let start = html.find(&format!("id=\"goal-{id}\"")).unwrap();
    let rest = &html[start..];
    &rest[..rest.find("</li>").unwrap()]
}

#[test]
fn goal_preview_uses_latest_observation_keeps_other_refs_and_preserves_status() {
    let mut state = fixture();
    state["goals"] = json!([{
        "id":"review",
        "title":"A failed synthetic goal with several observations",
        "status":"failed",
        "evidence_ids":["e-old", "e-new", "e-tie-a"]
    }]);
    let html = html(&state);
    let goal = goal_item(&html, "review");
    assert!(goal.contains("href=\"#evidence-e-new\""));
    assert!(goal.contains("Latest · Most recent observation →"));
    for id in ["e-old", "e-tie-a"] {
        let link = goal.find(&format!("href=\"#evidence-{id}\"")).unwrap();
        let anchor = &goal[link..];
        let text_start = anchor.find('>').unwrap() + 1;
        let text_end = anchor.find("</a>").unwrap();
        assert!(anchor[text_start..text_end].contains(id));
    }
    assert!(goal.contains("class=\"badge s-failed\">failed</span>"));
    assert!(!goal.contains("class=\"badge s-passed\""));
    for id in ["e-old", "e-new", "e-tie-a", "e-tie-b"] {
        assert_eq!(html.matches(&format!("id=\"evidence-{id}\"")).count(), 1);
    }
}

#[test]
fn latest_goal_preview_breaks_timestamp_ties_by_reference_order() {
    let mut state = fixture();
    state["goals"] = json!([{
        "id":"review",
        "title":"A synthetic goal with same-time observations",
        "status":"unknown",
        "evidence_ids":["e-tie-b", "e-tie-a", "e-old"]
    }]);
    let html = html(&state);
    let goal = goal_item(&html, "review");
    assert!(goal.contains("Latest · Second same-time observation →"));
    assert!(!goal.contains("Latest · First same-time observation →"));
    assert!(goal.contains("href=\"#evidence-e-tie-a\""));
    assert!(goal.contains("class=\"badge s-unknown\">unknown</span>"));
}

#[test]
fn preview_truncates_unicode_before_html_escaping_and_keeps_complete_ledger_label() {
    let mut state = fixture();
    let raw_label = format!("{}<<script>alert('synthetic')</script>", "🌱".repeat(108));
    state["evidence"][2]["label"] = json!(raw_label);
    let rendered = html(&state);
    let goal = goal_item(&rendered, "goal-passed");
    let expected_preview = format!("Latest · {}&lt;… →", "🌱".repeat(108));
    assert!(goal.contains(&expected_preview));
    assert_eq!(format!("{}<…", "🌱".repeat(108)).chars().count(), 110);
    assert!(!goal.contains("script"));
    assert!(!rendered.contains("<script"));
    assert!(rendered.contains(&visual_session::escape(&raw_label)));

    let full_boundary_label = format!("<{}&", "🌱".repeat(108));
    assert_eq!(full_boundary_label.chars().count(), 110);
    state["evidence"][2]["label"] = json!(full_boundary_label);
    let rendered = html(&state);
    let goal = goal_item(&rendered, "goal-passed");
    assert!(goal.contains(&format!(
        "Latest · {} →",
        visual_session::escape(&full_boundary_label)
    )));
    assert!(!goal.contains('…'));
}

#[test]
fn donut_segments_link_each_goal_with_its_honest_status() {
    let html = html(&fixture());
    assert!(html.contains("class=\"goal-ring\""));
    for status in [
        "planned", "running", "passed", "failed", "skipped", "unknown",
    ] {
        let link = format!("href=\"#goal-goal-{status}\"");
        assert_eq!(
            html.matches(&link).count(),
            1,
            "missing goal link: {status}"
        );
        let link_at = html.find(&link).unwrap();
        let after_link = &html[link_at..];
        let closing_anchor = after_link.find("</a>").unwrap();
        let anchor = &after_link[..closing_anchor];
        assert!(anchor.contains(&format!("A {status} goal · {status}")));
        assert!(anchor.contains(&format!("class=\"ring-segment s-{status}\"")));
        assert!(html.contains(&format!("id=\"goal-goal-{status}\"")));
        assert!(html.contains(&format!("<span>{status}</span><strong>1</strong>")));
    }
    assert_eq!(html.matches("class=\"ring-segment s-").count(), 6);
    assert!(html.contains("<strong>1</strong><span> / 6</span>"));
}

#[test]
fn empty_goal_state_remains_accessible_and_does_not_imply_completion() {
    let mut state = fixture();
    state["goals"] = json!([]);
    let html = html(&state);
    assert!(html.contains("No goals recorded yet"));
    assert!(!html.contains("class=\"ring-segment s-"));
    assert!(!html.contains("100% complete"));
    assert!(!html.contains("100% passed"));
    assert!(
        html.contains("Goal status") || html.contains("Goal counts") || html.contains("No goals")
    );
}

#[test]
fn legacy_same_state_stays_byte_identical_and_updates_keep_legacy_layout() {
    assert!(!LEGACY_HTML.contains("name=\"visual-session-layout\""));
    let original_state = parse(LEGACY_STATE).unwrap();
    assert_eq!(
        render(&original_state, Some(LEGACY_HTML)).unwrap(),
        LEGACY_HTML
    );

    let mut updated: Value = serde_json::from_str(LEGACY_STATE).unwrap();
    updated["revision"] = json!(2);
    updated["summary"] = json!("A second synthetic observation in the original layout.");
    let html = render(&parse(&updated.to_string()).unwrap(), Some(LEGACY_HTML)).unwrap();
    assert!(html.contains("A second synthetic observation in the original layout."));
    assert!(html.contains("class=\"status-chart\""));
    assert!(!html.contains("class=\"goal-ring\""));
    assert!(!html.contains("name=\"visual-session-layout\""));
    let begin = "<!-- visual-session:begin ";
    let end = "<!-- visual-session:end -->";
    assert_eq!(html.split(begin).next(), LEGACY_HTML.split(begin).next());
    assert_eq!(html.split(end).nth(1), LEGACY_HTML.split(end).nth(1));
}

#[test]
fn amplified_legacy_update_rejects_without_replacing_the_original() {
    use std::fs;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Sandbox(std::path::PathBuf);
    impl Drop for Sandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let sandbox = Sandbox(std::env::temp_dir().join(format!(
        "visual-session-legacy-amplification-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    )));
    fs::create_dir(&sandbox.0).unwrap();
    let output = sandbox.0.join("dashboard.html");
    let input = sandbox.0.join("session.json");
    fs::write(&output, LEGACY_HTML).unwrap();

    let mut amplified: Value = serde_json::from_str(LEGACY_STATE).unwrap();
    amplified["revision"] = json!(2);
    let evidence_ids: Vec<_> = (0..100)
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
                    "source": "Synthetic legacy amplification fixture",
                    "as_of": "2026-01-15T14:00:00Z"
                })
            })
            .collect(),
    );
    amplified["goals"] = Value::Array(
        (0..100)
            .map(|index| {
                json!({
                    "id": format!("goal-{index}"),
                    "title": "A synthetic goal with shared observations",
                    "status": "passed",
                    "evidence_ids": evidence_ids
                })
            })
            .collect(),
    );
    let data = amplified.to_string();
    assert!(data.len() <= 1024 * 1024);
    parse(&data).unwrap();
    fs::write(&input, data).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_visual-session"))
        .arg("render")
        .arg(input)
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert_eq!(fs::read_to_string(output).unwrap(), LEGACY_HTML);
}
