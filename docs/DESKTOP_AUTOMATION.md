# Vendor-Neutral Desktop Automation Framework

This document defines the first desktop automation boundary for Flekks EMR CLI.
It is intentionally generic. Public documentation and code must not name, target,
or imply official compatibility with proprietary local applications unless the
project has explicit permission and a reviewed integration plan.

## Scope

The desktop automation framework is for user-authorized local applications. It
may later let the agent observe a local window, propose a bounded action, wait
for human approval, perform the action, and verify the result.

The MVP deliverable is only:

- a vendor-neutral design
- minimal Rust interfaces for observation, proposals, approval, action, and verification
- policy gates that can be wired into the agent harness later

The MVP does not implement platform adapters, OCR, screenshots, autonomous
clinical decisions, autonomous diagnosis, radiology review, or medical-image
interpretation.

## Core Loop

```text
explicit user authorization
  -> allowlisted target selected
  -> observe local application
  -> evaluate accessibility-tree completeness
  -> propose action with semantic control selectors
  -> human approval when required
  -> act within bounded run limits
  -> verify by observing again
  -> append local PHI-safe audit events
```

Every run must be bounded, interruptible, and auditable. A user must be able to
stop the run before any action executes.

## Why Tree Completeness Is a First-Class Gate

Desktop accessibility APIs are not uniform across operating systems. Windows
UI Automation and macOS accessibility can expose different control models, and
many applications expose incomplete trees because of custom controls, embedded
webviews, Electron shells, or canvas-like UI layers.

The framework must not treat a process allowlist as sufficient. Each automation
target needs a policy for accessibility-tree completeness:

- `verified_complete`: semantic controls are present and stable enough to act on
- `semantic_controls_present`: enough named controls exist for non-irreversible actions
- `partial`: observation may be useful, but action proposals should usually block
- `empty` or `unknown`: automation must not act

Coordinate-only proposals are high risk because they say "click this pixel" rather
than "activate this reviewed control." Coordinate fallback is disabled by default
and must never be used for signing, submission, export, deletion, finalization, or
other irreversible actions.

## Target Allowlist

An automation target is allowlisted by stable local metadata, not by medical
workflow assumptions:

- target id chosen by the user or administrator
- platform
- process name
- optional executable hash
- optional window class or non-PHI structural identifier
- minimum accessibility-tree completeness
- allowed observation modes
- allowed action classes
- coordinate fallback policy

Window titles, extracted text, screenshots, OCR text, and accessibility names may
contain PHI. They must not be stored in public fixtures, public logs, GitHub
issues, examples, screenshots, or documentation. When identifiers are needed,
store hashes or local-only references.

## Authorization

Before observing or acting, the user must explicitly authorize the target and the
run. Authorization should include:

- who authorized it
- when authorization was granted
- optional expiration
- the target id
- the reason for the run
- allowed observation modes and action classes from the target policy

Authorization is local state. It is not a public compatibility claim.

## Observation

Observation adapters should prefer semantic accessibility trees. Visual fallback
such as screenshots or OCR is not part of the MVP. If added later, it must be
explicitly enabled per target and must apply PHI suppression or redaction at
capture time before anything is persisted.

Observation records should keep PHI-safe metadata:

- observation id
- target id
- mode
- timestamp
- tree completeness
- whether raw capture was persisted
- whether PHI handling was applied at capture
- hashed or local-only control identifiers

The raw accessibility tree itself can contain PHI. Treat it as sensitive capture
data.

## Proposal

An action proposal should be understandable without exposing PHI. It should use
semantic selectors whenever possible:

```text
target id
observation id
control id or hash
control role
action class
risk level
redacted rationale
bounded run limits
verification expectation
```

Sensitive values should be passed by local value reference instead of stored in
the proposal body. For example, the proposal can say "enter value from local
session ref note-field-4" while the actual PHI remains in local runtime memory or
encrypted local storage.

## Approval And Irreversible Actions

Human confirmation is required before:

- signing
- submission
- export
- deletion
- finalization
- irreversible local writes
- outbound PHI transfer

Approval records should be PHI-safe and local. A proposal that would perform an
irreversible action must be rejected unless it explicitly requires human
confirmation and the approval includes the matching irreversible action kind.

## Action

The action layer is intentionally absent from the MVP. Future platform-specific
adapters must implement the generic trait boundary and obey the policy gates.

Action execution must be:

- bounded by max steps and max wall-clock duration
- cancellable
- local-only unless an explicit outbound policy allows otherwise
- audited without raw PHI
- verified after execution

## Verification

Verification should observe again and compare against a PHI-safe expectation:

- expected control exists
- expected control changed state
- expected workflow state changed
- action had no observable effect
- verification inconclusive

Verification must not claim that a clinical or billing decision is correct. It
only confirms that the local UI appears to have changed as expected.

## Audit Events

The audit trail should store local metadata only:

- run started
- target allowed or blocked
- observation requested
- tree completeness result
- action proposed
- approval granted or denied
- action attempted
- verification passed, failed, or inconclusive
- run cancelled or completed

Do not store raw screenshots, OCR text, accessibility text, clinical content,
patient identifiers, or copied form values in audit messages.

## Future Adapter Work

Future implementation can study platform APIs and existing open-source
abstractions, but adapters must remain replaceable behind the generic traits.
The public framework should stay vendor-neutral even if a local user privately
configures an allowlist for their own applications.

Candidate adapter layers:

- Windows UI Automation
- macOS accessibility
- Linux accessibility, if support is practical
- OCR or screenshot fallback behind explicit policy gates

No adapter should be merged until it has tests for target allowlisting,
authorization, tree completeness, capture-time PHI handling, bounded execution,
human confirmation, cancellation, and local audit output.
