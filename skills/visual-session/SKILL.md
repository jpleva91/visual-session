---
name: visual-session
description: Create and update a private, self-contained HTML dashboard for an ongoing agent session, with evidence-backed progress and suggested next prompts, skills, context, and loops.
---

# Visual Session

Maintain one readable session dashboard and its structured state. Use this for a visual session companion or handoff, not a background agent, task runner, or source of new execution authority.

## Create

1. Read the current task, its constraints, and only the evidence already provided or obtained within the authorized task. Choose a compact summary of the goal, meaningful milestones, unresolved blockers, and useful next steps. Do not collect transcripts, environment variables, credentials, or unrelated files to populate the page.
2. Read [the state contract](references/state-contract.md). Create a unique, stable session `id` and revision `1`. Record evidence sources and UTC observation times. Use `unknown` where a claim has not been verified; never label proposed work `passed`.
3. Default to `.visual-session/state.json` and `.visual-session/dashboard.html` in the working project. Before saving real data, ensure these paths are untracked and locally ignored. Use a private directory outside the checkout if local exclusion is unavailable. Create state with restrictive permissions; see [local files and updates](references/local-workflow.md).
4. With the existing Rust toolchain, validate and render, substituting the installed skill directory:

   ```sh
   cargo run --offline --locked --manifest-path <skill-dir>/scripts/renderer/Cargo.toml -- validate .visual-session/state.json
   cargo run --offline --locked --manifest-path <skill-dir>/scripts/renderer/Cargo.toml -- render .visual-session/state.json .visual-session/dashboard.html
   ```

   If dependencies are not cached, report the missing build prerequisite. Do not install software or enable network access implicitly.
5. Open the local HTML when a file viewer is available. Check that status, evidence dates, and next steps make sense at desktop and narrow widths. Return the same dashboard path and a short factual update.

## Update

Read the existing state and HTML before editing. Keep the session `id`, output path, existing goal/evidence IDs, user notes, and deliberate user edits. Merge the relevant state changes, increment `revision`, and advance `as_of` to the new snapshot time. Preserve evidence observation times unless new evidence was actually obtained.

Run validation and render to the same output. The renderer preserves HTML outside its managed region and refuses to overwrite edits inside it, a different identity, or a stale revision. On refusal, retain the existing files and explain the conflict; do not delete checksums, change identity, or overwrite the page to bypass it. Follow [local files and updates](references/local-workflow.md) for recovery choices.

## Keep the companion honest and useful

- Separate plans, activity, and results using exactly `planned`, `running`, `passed`, `failed`, `skipped`, or `unknown`. A passed goal requires supporting passed evidence. A chart counts recorded goals; it is not a prediction or overall quality score.
- Give each result a compact provenance label and observation time. State when evidence predates a relevant change; downgrade the current claim to `unknown` until rechecked rather than implying fresh validation.
- Recommendations are suggestions. A prompt, named skill, or loop in the page grants no permission, schedules nothing, and runs nothing. Suggest a skill only when it is available or clearly label availability as unverified.
- Context guidance should identify the few facts, decisions, constraints, artifact locations, and unresolved questions needed for the next step. Avoid copying hidden instructions or long transcripts into state.
- Suggest a bounded loop: one next check or change, the evidence to inspect, and a clear stop condition. Respect the user's current priority and resource limits. Do not turn an update into continuous monitoring.
- Keep real state and dashboards local/private. Public examples must be synthetic. Do not publish, upload, commit, or share a real dashboard merely because the skill repository is public. The renderer escapes state text and restricts links; it does not determine whether text is confidential.
