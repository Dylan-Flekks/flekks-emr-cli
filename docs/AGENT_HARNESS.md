# Medical Agent Harness

The medical agent harness is the structured bridge between the local chart repository, the Ratatui dashboard, local multimodal datasets, optional local desktop automation, and optional model-provider API calls.

The harness is not a generic coding agent and should not be limited to one vendor, one model, one billing workflow, or one local application. It is a model-agnostic medical workflow harness that must obey local-only storage, auditability, human review, and BAA-gated PHI boundaries.

## Goals

- Help draft and edit structured clinical documentation.
- Help audit notes before signing.
- Help prepare billing-support drafts.
- Keep the TUI informed about agent state.
- Support bounded long-running and recursive agent loops.
- Support local multimodal inputs such as text, PDFs, screenshots, scanned documents, and voice memos.
- Support user-authorized local desktop software interactions through generic UI automation abstractions.
- Prevent outbound PHI unless compliance checks pass.
- Keep all chart source-of-truth data local in SQLite.

## Package Boundary

```text
med-agent
  agent turn state
  tool registry
  medical tool names
  BAA preflight before outbound model calls
  long-running loop controls
  TUI-facing agent events

med-ai
  provider abstraction
  model-provider API boundary
  request/response structs

med-store
  local SQLite repository

med-compliance
  BAA and vendor approval records

med-tui
  dashboard, status, review, and human confirmation UI
```

## Harness Flow

```text
TUI/CLI request
  -> MedicalAgentHarness::start_turn
  -> classify request: PHI | deidentified | non-PHI
  -> if outbound provider is requested:
       run BAA/vendor preflight
       block if missing, expired, revoked, not approved, or service not covered
  -> select allowed medical tools
  -> execute local tools against SQLite/service layer
  -> stream state/events back to TUI
  -> require human review before signing/exporting/billing finalization
  -> append audit events
```

## Recursive Loop Model

Long-running agent loops must be bounded, inspectable, and interruptible.

```text
observe local state
  -> plan next step
  -> run allowed local tool
  -> verify result
  -> update local memory/state
  -> decide continue | wait for approval | stop
```

Required loop controls:

- maximum step count
- maximum wall-clock duration
- cancellation from TUI/CLI
- human approval checkpoints
- local audit trail
- PHI-safe logs
- explicit blocked state with reason
- no irreversible action without confirmation

## Initial Tool Set

```text
chart.search_patients
chart.read_patient_summary
chart.list_encounters
note.create_draft
note.update_draft
note.run_documentation_audit
billing.prepare_superbill_draft
compliance.check_vendor_baa
ai.draft_note_with_provider
desktop.observe_window
desktop.propose_action
desktop.verify_state
audio.import_voice_memo
document.extract_text
```

Chart, note, audit, billing, desktop observation, document extraction, and local memory tools are local. Outbound AI tools are disabled for PHI until the BAA gate passes.

## Model Provider Boundary

Model-provider adapters must not be called directly from UI code. OpenAI is one provider, not the harness itself.

Required preflight:

```text
if request.contains_phi:
  require configured model provider
  require local vendor compliance record exists
  require BAA status == executed
  require requested service/model is covered
  require approval.approved == true
  append attempted AI audit event
  block if any check fails
```

## Local Desktop Automation Boundary

The harness may eventually operate user-authorized local desktop software in the same way a user would interact with it: observe the screen/window, propose an action, request approval when needed, perform a bounded action, then verify state.

Public Flekks EMR CLI documentation must stay vendor-neutral. Do not name or imply official integration with proprietary desktop medical or billing software unless the project has explicit permission and a reviewed integration strategy.

The detailed interface and policy plan lives in [DESKTOP_AUTOMATION.md](DESKTOP_AUTOMATION.md). The first implementation boundary treats accessibility-tree completeness as a per-target policy gate, not just a process allowlist. Coordinate-only fallback is blocked by default and must never be used for signing, submission, export, deletion, finalization, or other irreversible actions.

Allowed public language:

- local desktop software
- user-authorized local applications
- generic UI automation
- local billing software workflows
- supervised data entry

Avoid public language:

- naming proprietary software vendors or products
- implying partnership, endorsement, or official compatibility
- claiming automated submission or final billing authority

All local automation must be:

- user-authorized
- window/process allowlisted
- gated by accessibility-tree completeness
- PHI-handled at capture time
- auditable
- interruptible
- reversible where possible
- confirmed by a human before submission, signing, export, or irreversible changes

## Multimodal Boundary

The harness may process local multimodal datasets such as PDFs, screenshots, scanned documents, and voice memos for documentation support and workflow automation.

Medical image interpretation is not an MVP goal. Do not position the project as software that diagnoses, interprets radiology, or performs autonomous clinical image review. Any future medical-image feature must be reviewed for clinical, regulatory, privacy, and safety requirements before implementation.

## Dashboard Integration

The TUI should show:

- agent state: idle, thinking, running local tool, waiting for approval, blocked, done
- current tool name
- AI BAA lock status
- blocked-request reason
- human-review warnings
- local-only storage indicator

## Dependencies

Baseline:

- Rust
- Ratatui
- Crossterm
- SQLite via `rusqlite`
- SQLCipher feature for PHI-capable builds
- Serde
- UUID

Future OpenAI adapter:

- `reqwest`
- `tokio`
- `eventsource-stream` or streaming-compatible parser
- `schemars` if JSON schema tool definitions are generated

The default build should not require an OpenAI API key.
