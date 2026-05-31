# Compliance Guardrails

This project is not a compliance guarantee. It is an open-source software project that should make compliant behavior easier to implement and unsafe behavior harder to trigger.

## Non-Negotiable Rules

- Do not commit PHI.
- Do not put PHI in GitHub issues, pull requests, screenshots, logs, prompts, examples, fixtures, or documentation.
- Do not send PHI to third-party APIs unless the provider, account, product, and service are covered by an executed BAA and locally approved.
- Do not treat generated billing codes or documentation as final without qualified human review.
- Do not treat local automation output as final without qualified human review.
- Do not position the project as autonomous diagnostic, radiology, or medical-image interpretation software.
- Do not use this for production patient care until it has undergone legal, privacy, security, and clinical review.

## BAA Gate

Before any AI provider can receive PHI, the app must verify:

```text
BAA status = executed
provider is approved for PHI
requested service/model is covered
approval is current
request metadata is logged
```

If any check fails, the app must block the request.

## Local Medical Storage

The repository may contain:

- schemas
- templates
- migrations
- synthetic fixtures
- deidentified fixtures
- code
- documentation

The repository must not contain:

- real patient charts
- identifiers
- clinical exports
- screenshots with PHI
- logs with PHI
- BAA contracts
- API keys

## Audit Scope

Audit events should cover:

- chart opened
- patient searched
- note created
- note edited
- note signed
- note amended
- billing code added or changed
- export created
- backup created
- AI call allowed
- AI call blocked
- local desktop observation performed
- local desktop action proposed
- local desktop action approved or denied
- local desktop action performed
- compliance vendor changed
- failed unlock/login

## Local Desktop Automation

Local desktop automation must be generic and user-authorized. Public documentation should not name proprietary medical or billing software products unless explicit permission and a reviewed integration plan exist. The first automation adapter should be macOS-first until the local authorization, capability-profile, and audit model is proven.

Automation targets must be allowlisted by local policy and must meet the required accessibility-tree completeness for the proposed action. A process allowlist alone is not enough. If the accessibility tree is missing, partial, or unreliable, action proposals should block instead of silently falling back to raw OCR text or coordinates.

Each target must have a local capability profile describing what the adapter can reliably observe and act on. Incomplete or stale profiles should downgrade the target to observe-only or blocked.

PHI redaction or suppression must happen at capture time for accessibility trees, screenshots, OCR text, and any visual metadata. Raw captures containing PHI must not be persisted in logs, examples, fixtures, screenshots, issue comments, or public documentation.

Automation must require human confirmation before submission, signing, export, deletion, or irreversible changes.

## Multimodal Data

Voice memos, screenshots, scanned documents, and extracted text can contain PHI. They must be stored, processed, logged, and exported under the same PHI rules as chart records.

The project should support documentation workflow assistance, not autonomous diagnosis or medical-image interpretation.

## Vendor Registry

Vendor records live in the local compliance registry. Public repository examples must be placeholders only.
