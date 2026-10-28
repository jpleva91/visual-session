use crate::{badge, escape, Session, Suggestion, STATUSES};
use std::fmt::Write as _;

fn readable_time(timestamp: &str, seconds: bool) -> String {
    // The parser has already checked ASCII, UTC shape, and calendar validity.
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let month = timestamp[5..7].parse::<usize>().expect("validated month");
    let day = timestamp[8..10].parse::<u8>().expect("validated day");
    let clock = if seconds {
        &timestamp[11..19]
    } else {
        &timestamp[11..16]
    };
    format!(
        "{day} {} {} · {clock} UTC",
        months[month - 1],
        &timestamp[..4]
    )
}

fn suggestion(html: &mut String, s: &Suggestion) {
    let _ = write!(html, "<article class=\"suggestion\"><p class=\"kind\">{} SUGGESTION</p><h3>{}</h3><p class=\"detail\">{}</p></article>", s.kind.to_uppercase(), escape(&s.title), escape(&s.detail));
}

fn preview(label: &str) -> String {
    let mut characters = label.chars();
    let mut result: String = characters.by_ref().take(110).collect();
    if characters.next().is_some() {
        result.pop();
        result.push('…');
    }
    result
}

fn goal_ring(html: &mut String, s: &Session) {
    let total = s.goals.len();
    let passed = s.goals.iter().filter(|g| g.status == "passed").count();
    html.push_str("<section class=\"status-map\" aria-labelledby=\"status-map-title\"><div class=\"section-head\"><h2 id=\"status-map-title\">The shape of the work</h2><p>One segment per goal. Each state stays visible.</p></div><div class=\"ring-layout\"><div class=\"ring-wrap\"><svg class=\"goal-ring\" viewBox=\"0 0 184 184\" role=\"group\" aria-label=\"Goal states; each linked segment represents one goal\">");
    if total == 0 {
        html.push_str("<circle class=\"ring-empty\" cx=\"92\" cy=\"92\" r=\"68\" fill=\"none\" stroke-width=\"15\"><title>No goals recorded</title></circle>");
    } else {
        let circumference = 2.0 * std::f64::consts::PI * 68.0;
        let step = circumference / total as f64;
        let gap = 4.0_f64.min(step / 4.0);
        let arc = step - gap;
        for (index, goal) in s.goals.iter().enumerate() {
            let label = escape(&format!("{} · {}", goal.title, goal.status));
            let offset = -(index as f64 * step + gap / 2.0);
            let _ = write!(html, "<a href=\"#goal-{}\" aria-label=\"{}\"><title>{}</title><circle class=\"ring-segment s-{}\" cx=\"92\" cy=\"92\" r=\"68\" fill=\"none\" stroke-width=\"15\" stroke-dasharray=\"{:.4} {:.4}\" stroke-dashoffset=\"{:.4}\" transform=\"rotate(-90 92 92)\" /></a>", goal.id, label, label, goal.status, arc, circumference - arc, offset);
        }
    }
    html.push_str("</svg><div class=\"ring-center\">");
    if total == 0 {
        html.push_str("<strong>—</strong><span>No goals recorded</span>");
    } else {
        let _ = write!(
            html,
            "<p><strong>{passed}</strong><span> / {total}</span></p><span>goals passed</span>"
        );
    }
    html.push_str("</div></div><div class=\"status-legend\">");
    for status in STATUSES {
        let count = s.goals.iter().filter(|g| g.status == status).count();
        let _ = write!(html, "<div class=\"legend-item\"><span class=\"dot s-{status}\" aria-hidden=\"true\"></span><span>{status}</span><strong>{count}</strong></div>");
    }
    html.push_str("</div></div><p class=\"ring-caption\">Passed counts require linked passed evidence. This is not an effort estimate.</p></section>");
}

pub(super) fn content(s: &Session) -> String {
    let mut html = String::new();
    let _ = write!(html, "<div class=\"session-content\"><header class=\"hero\"><div class=\"hero-copy\"><p class=\"eyebrow\">YOUR SESSION, IN VIEW</p><h1>{}</h1><p class=\"summary\">{}</p></div><div class=\"snapshot-meta\"><span class=\"revision\">Revision {}</span><time datetime=\"{}\" title=\"{}\">{}</time></div></header><div class=\"session-overview\"><section class=\"next-move\" aria-labelledby=\"next-move-title\"><p class=\"eyebrow\">SUGGESTED NEXT MOVE</p>", escape(&s.title), escape(&s.summary), s.revision, s.as_of, s.as_of, readable_time(&s.as_of, false));
    if let Some(prompt) = s.suggestions.iter().find(|item| item.kind == "prompt") {
        let _ = write!(
            html,
            "<h2 id=\"next-move-title\">{}</h2><span class=\"prompt-label\">Suggested prompt · select text to copy</span><p class=\"focus-prompt\">{}</p>",
            escape(&prompt.title),
            escape(&prompt.detail)
        );
    } else {
        html.push_str("<h2 id=\"next-move-title\">No prompt suggested yet</h2><p class=\"focus-prompt empty\">Review the current goal and evidence before choosing the next move.</p>");
    }
    html.push_str("<p class=\"focus-footer\">Suggestions only. Nothing runs from this page.</p>");
    html.push_str("<a class=\"quiet-link\" href=\"#goals\">Review the goals →</a><svg class=\"move-arrow\" viewBox=\"0 0 56 56\" aria-hidden=\"true\"><path d=\"M12 44 44 12M12 12h32v32\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" /></svg></section>");
    html.push_str("<section class=\"trajectory\" id=\"goals\"><div class=\"section-head\"><p class=\"eyebrow\">01 / WORK IN MOTION</p><h2>A clear path forward</h2></div><ol class=\"goal-list\">");
    for (index, goal) in s.goals.iter().enumerate() {
        let _ = write!(html, "<li class=\"goal\" id=\"goal-{}\"><span class=\"goal-index\" aria-hidden=\"true\">{:02}</span><div class=\"goal-body\"><h3>{}</h3><p class=\"evidence-refs\">", goal.id, index + 1, escape(&goal.title));
        let latest = goal
            .evidence_ids
            .iter()
            .map(|id| {
                s.evidence
                    .iter()
                    .find(|evidence| evidence.id == *id)
                    .expect("validated evidence reference")
            })
            .reduce(|latest, observation| {
                if observation.as_of > latest.as_of {
                    observation
                } else {
                    latest
                }
            });
        if let Some(observation) = latest {
            let _ = write!(
                html,
                "<a class=\"latest-evidence\" href=\"#evidence-{}\">Latest · {} →</a>",
                observation.id,
                escape(&preview(&observation.label))
            );
            for id in goal.evidence_ids.iter().filter(|id| **id != observation.id) {
                let _ = write!(
                    html,
                    " · <a href=\"#evidence-{id}\">View evidence · {id}</a>"
                );
            }
        } else {
            html.push_str("No evidence linked");
        }
        let _ = write!(html, "</p></div>{}</li>", badge(&goal.status));
    }
    if s.goals.is_empty() {
        html.push_str("<li class=\"empty\">No goals recorded yet. Add a concrete goal when scope is known.</li>");
    }
    html.push_str("</ol></section>");
    goal_ring(&mut html, s);
    html.push_str("<section class=\"attention\"><div class=\"section-head\"><p class=\"eyebrow\">02 / ATTENTION</p><h2>What needs attention</h2></div>");
    for blocker in &s.blockers {
        let _ = write!(html, "<article class=\"blocker\"><div class=\"blocker-head\"><h3>{}</h3>{}</div><p>{}</p></article>", escape(&blocker.title), badge(&blocker.status), escape(&blocker.detail));
    }
    if s.blockers.is_empty() {
        html.push_str(
            "<p class=\"empty\">No blockers recorded. This does not prove there are none.</p>",
        );
    }
    html.push_str("</section></div><section class=\"continuity\" id=\"continuity\"><div class=\"context-brief\"><div class=\"section-head\"><p class=\"eyebrow\">03 / KEEP THE THREAD</p><h2>Carry what matters</h2></div>");
    let contexts = s
        .suggestions
        .iter()
        .filter(|item| item.kind == "context")
        .collect::<Vec<_>>();
    for context in &contexts {
        suggestion(&mut html, context);
    }
    if contexts.is_empty() {
        html.push_str("<p class=\"empty\">No context guidance recorded.</p>");
    }
    html.push_str("<div class=\"skill-hints\">");
    let skills = s
        .suggestions
        .iter()
        .filter(|item| item.kind == "skill")
        .collect::<Vec<_>>();
    for skill in &skills {
        let _ = write!(html, "<details class=\"skill-hint\"><summary><span class=\"kind\">SKILL SUGGESTION</span>{}</summary><p class=\"detail\">{}</p></details>", escape(&skill.title), escape(&skill.detail));
    }
    if skills.is_empty() {
        html.push_str("<p class=\"empty\">No skill suggestions recorded.</p>");
    }
    html.push_str("</div><div class=\"more-prompts\">");
    for prompt in s
        .suggestions
        .iter()
        .filter(|item| item.kind == "prompt")
        .skip(1)
    {
        let _ = write!(html, "<details><summary><span class=\"kind\">ANOTHER POSSIBLE MOVE</span>{}</summary><p class=\"detail\">{}</p></details>", escape(&prompt.title), escape(&prompt.detail));
    }
    html.push_str("</div></div><div class=\"loop-brief\"><div class=\"section-head\"><p class=\"eyebrow\">04 / A SUGGESTED RHYTHM</p><h2>Make the next loop count</h2></div><ol class=\"loop-track\" aria-label=\"Suggested work loop, not execution status\"><li>Act</li><li>Check</li><li>Record</li><li>Stop or continue</li></ol>");
    let loops = s
        .suggestions
        .iter()
        .filter(|item| item.kind == "loop")
        .collect::<Vec<_>>();
    for item in &loops {
        suggestion(&mut html, item);
    }
    if loops.is_empty() {
        html.push_str("<p class=\"empty\">No session-specific loop guidance recorded. Review the current goal and authorization before continuing.</p>");
    }
    html.push_str("<p class=\"advisory\">All next moves are suggestions. This page does not grant permission or run work.</p></div></section><section class=\"evidence-section\" id=\"evidence\"><div class=\"section-head\"><p class=\"eyebrow\">05 / THE EVIDENCE TRAIL</p><h2>What we actually know</h2><p>Newest observations first. Source and observation time travel with every result.</p></div><div class=\"evidence-list\">");
    let mut observations = s.evidence.iter().collect::<Vec<_>>();
    observations.sort_by(|left, right| right.as_of.cmp(&left.as_of));
    for evidence in observations {
        let _ = write!(html, "<article class=\"evidence\" id=\"evidence-{}\"><div class=\"evidence-time\"><time datetime=\"{}\">{}</time></div><div class=\"evidence-body\"><div class=\"evidence-head\"><h3>{}</h3>{}</div><p class=\"source\">{}</p><footer><time datetime=\"{}\">{}</time>", evidence.id, evidence.as_of, &evidence.as_of[11..16], escape(&evidence.label), badge(&evidence.status), escape(&evidence.source), evidence.as_of, readable_time(&evidence.as_of, true));
        if let Some(url) = &evidence.url {
            let _ = write!(
                html,
                "<a href=\"{}\" rel=\"noreferrer noopener\">Open evidence ↗</a>",
                escape(url)
            );
        }
        html.push_str("</footer></div></article>");
    }
    if s.evidence.is_empty() {
        html.push_str(
            "<p class=\"empty\">No observations recorded. Unknown is an honest starting point.</p>",
        );
    }
    let _ = write!(
        html,
        "</div></section><details class=\"notes\"><summary>Session notes · {}</summary><ul>",
        s.notes.len()
    );
    for note in &s.notes {
        let _ = write!(html, "<li>{}</li>", escape(note));
    }
    if s.notes.is_empty() {
        html.push_str("<li class=\"empty\">No session notes recorded.</li>");
    }
    html.push_str("</ul></details><footer class=\"snapshot-footer\">A local snapshot, not a live monitor. Refresh from verified evidence when the session changes.</footer></div>\n");
    html
}
