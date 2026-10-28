# Local files and updates

## Private destination

Use the user's existing destination when supplied. Otherwise use `.visual-session/` in the working project and keep it out of version control. Before writing real data in a Git checkout, verify the paths are not already tracked and that a local ignore covers both state and HTML. A local `.git/info/exclude` entry for `.visual-session/` avoids changing shared project policy; resolve the actual exclude location with `git rev-parse --git-path info/exclude` for worktrees. Respect existing policies and permissions.

Create the directory and state with a restrictive umask, for example `umask 077` before `mkdir -p .visual-session` and saving JSON. Verify existing permissions before updating. The renderer writes the HTML privately on supported Unix systems; it does not secure the input state or its parent directory for you. Local/private means excluded from the repository and deliberately handled, not encrypted or safe to sync everywhere.

If files are already tracked, do not silently untrack or rewrite them. Preserve them and choose a new private destination or resolve the user's intended sharing scope. If the local exclusion file is unavailable, use an appropriate private directory outside the checkout. Do not change access controls or global Git settings to force the default location.

## First render

Write state from the explicit task and compact authorized evidence using [the contract](state-contract.md). Start with only the milestones that help explain the session. Empty sections are valid; fabricated content is not necessary. Validate before rendering. An existing non-dashboard file at the output path is a collision, not an invitation to replace it.

The HTML includes its styling and status visualization in one file and needs no server. It has no telemetry, remote scripts, or automatic network calls. The installed template is trusted code; session strings are rendered as escaped text. Do not introduce scripts, external fonts, embeds, or raw state interpolation when restyling it.

## Updating one artifact

1. Read the current state, dashboard, and relevant new evidence. Keep user-authored notes and stable IDs; edit the smallest set of fields needed. A goal can change state without changing identity.
2. Save a private recoverable copy of the state when useful, then increase `revision` and update the snapshot `as_of`. Update evidence times only for new observations. Save atomically with the existing file tools where possible.
3. Validate and render to the same output path. Inspect the exit status and resulting dashboard before reporting success. Do not claim an update succeeded merely because state was saved.
4. Report the current dashboard path, meaningful status changes, and any unresolved uncertainty. Reload the same local page to view the new snapshot.

The renderer uses an identified managed region with an edit-detection checksum. It updates that region while preserving surrounding bytes. Keep human-written HTML, styling, and additional notes outside the managed markers. State `notes` are appropriate for user notes that should travel with the structured snapshot.

Edits inside the managed region produce a conflict. Preserve the edited page and state, show the conflicting content to the user when needed, and offer to move the edit into state or outside the region. Reconciliation must retain the user's intent; removing the checksum or recreating the dashboard to evade the refusal would lose the protection. Also preserve the existing artifact on identity mismatch, stale revision, invalid input, or a busy lock. Investigate an uncertain prior render before retrying it. Do not guess that a lock is stale.

Changes to the installed template affect newly created pages. Existing pages preserve their surrounding HTML so user customizations survive. Applying a redesigned template to an existing artifact is a separate migration that requires deliberate reconciliation.

## Useful recommendations

Prefer a few grounded suggestions over a generic checklist:

- `prompt`: A copyable next request tied to the unresolved task, such as asking to compare a failed check with the expected result.
- `skill`: An available skill that addresses the next step; explain why it fits. Label availability as unverified if it cannot be checked.
- `context`: The decision, constraint, evidence reference, or unresolved question to carry into the next turn or handoff. Keep it short and exclude hidden instructions.
- `loop`: One bounded next action, the evidence that determines success, and the stop condition. For example: “Check one example; record its actual output; stop after recording success or a missing prerequisite.”

A recommendation never authorizes tool use, outside communications, publication, scheduling, retries, or resource consumption. Execution follows the user's task scope and the environment's permissions.
