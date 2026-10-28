# Session state contract, version 1

The state is one UTF-8 JSON object. All fields shown below are required except evidence `url`. Unknown fields are rejected so misspellings do not silently disappear. Strings are plain text, not HTML or Markdown.

```json
{
  "schema_version": 1,
  "id": "sample-session",
  "revision": 1,
  "title": "A small documentation refresh",
  "summary": "Clarify one guide and verify its examples.",
  "as_of": "2026-01-15T10:00:00Z",
  "goals": [
    {"id": "scope", "title": "Agree the narrow scope", "status": "passed", "evidence_ids": ["scope-note"]},
    {"id": "examples", "title": "Check the guide examples", "status": "planned", "evidence_ids": []}
  ],
  "evidence": [
    {"id": "scope-note", "label": "Scope confirmed", "status": "passed", "source": "Synthetic planning note, item 1", "as_of": "2026-01-15T09:55:00Z"}
  ],
  "blockers": [],
  "suggestions": [
    {"kind": "loop", "title": "Check one example", "detail": "Run the first example and inspect its output. Stop when its result is recorded or a missing prerequisite is identified."}
  ],
  "notes": ["This example is entirely synthetic."]
}
```

| Field | Meaning and rules |
| --- | --- |
| `schema_version` | Integer `1`. |
| `id` | Stable session identity using lowercase letters, digits, and hyphens, at most 80 bytes, with no leading or trailing hyphen. Use a generic slug rather than personal or confidential identifiers. |
| `revision` | Positive integer. Increase for each changed snapshot; keep it unchanged only for an identical rerender. |
| `title`, `summary` | A useful title and concise current account of the session. |
| `as_of` | Snapshot time, strictly `YYYY-MM-DDTHH:MM:SSZ`, using a valid UTC calendar date. |
| `goals` | `{id, title, status, evidence_ids}` records. IDs are unique within goals. References must resolve to evidence IDs. A `passed` goal needs at least one `passed` evidence record. |
| `evidence` | `{id, label, status, source, as_of, url?}` records. IDs are unique within evidence. `source` identifies the observation or check without leaking private details. `as_of` cannot be later than the snapshot. |
| `blockers` | `{id, title, status, detail}` records with unique IDs. Describe the missing information, failed check, or dependency and what would resolve it. |
| `suggestions` | `{kind, title, detail}` records. `kind` is `prompt`, `skill`, `context`, or `loop`. They are proposals for the reader. |
| `notes` | Plain-text user/context notes. Preserve existing notes when updating. |

Every status is exactly one of:

| Status | Use it for |
| --- | --- |
| `planned` | Intended work that has not started. |
| `running` | Work actually in progress at the recorded snapshot time. |
| `passed` | An observed successful result, with evidence for goals. |
| `failed` | An observed unsuccessful result. Include its practical meaning in the evidence or blocker detail. |
| `skipped` | A deliberate omission; state why. It does not count as passed. |
| `unknown` | Unverified, unavailable, stale for the current change, or uncertain outcome. |

There is no separate `blocked` status. Record an unresolved dependency under `blockers`, use a factual status such as `unknown` or `failed`, and explain it in `detail`. A blocker marked `passed` means it has been resolved; keep the resolution evidence in `evidence` if relevant.

All arrays must be present and may be empty. The validator limits the input to 1 MiB, each array to 100 entries, and each string to 8,000 UTF-8 bytes. Strings must be nonempty after trimming and must not contain control characters other than newline or tab. IDs follow the slug rules. Do not use reserved fields to hide unrelated data. Rendering additionally rejects a resulting or existing HTML file over 8 MiB; reduce verbose repeated evidence references if that limit is reached.

Optional evidence URLs must start with lowercase `https://`, have an ASCII DNS-style hostname and optional port `1`–`65535`, and contain no credentials, whitespace, control characters, quotes, backticks, angle brackets, or backslashes. IPv6 literals, other schemes, local paths, and protocol-relative links are rejected. Omit a link when a short source label is sufficient. No link is fetched by the renderer; opening one remains a reader action.

`as_of` describes the recorded observation, not guaranteed freshness. Do not replace an old check's time with the current time to make it look current. If a relevant change invalidates old evidence, retain the historical observation and mark the current goal `unknown` until revalidated.
