# Request To The Codex CLI Team

Cc: Codex CLI maintainers and upstream agent-runtime contributors.

This fork is a request for a dedicated agentic harness for medical workflows in Codex CLI: local-first, terminal-native, auditable, and designed around clinical documentation, chart editing, billing support, and supervised medical workflow automation.

## Core Ask

We would like to see a multimodal medical agent harness that supports:

- Local agentic loops for chart review, note editing, audit, and billing-support workflows.
- TUI dashboards for visual audit directly inside the terminal.
- macOS authorization and accessibility-aware automation boundaries.
- Human approval checkpoints before irreversible actions.
- PHI-aware local execution with BAA-gated outbound model calls.
- Plugin surfaces for medical coding, Medicare rules, ICD-10-CM, CPT/HCPCS, payer policy, documentation checks, and compliance review.
- Multimodal local inputs, including PDFs, screenshots, scanned documents, voice notes, and structured chart data.
- Robust integrations first, with OCR and point-click automation only as supervised fallbacks when stronger APIs are unavailable.

## Why This Matters

Medical workflows need more than a generic coding agent or plain terminal logs.

A useful clinical terminal harness should show:

- Active patient/chart context.
- Current note section.
- Agent state.
- Current tool call.
- Pending human approvals.
- PHI boundary status.
- BAA/provider status.
- Documentation warnings.
- Billing-readiness warnings.
- Audit trail events.
- What the agent is allowed to do next.

The terminal should function as a supervised medical workflow dashboard, not just a command runner.

## Agentic Loop Requirements

Medical agent loops should be bounded, inspectable, interruptible, and reviewable.

Example state model:

```text
idle
thinking
running local tool
reviewing chart context
editing draft note
checking documentation support
checking billing readiness
waiting for human approval
blocked by PHI policy
blocked by missing BAA
blocked by missing clinical context
blocked by unsafe automation fallback
done
```

A loop should be able to:

- Observe local chart/workflow state.
- Plan the next safe step.
- Run an approved local tool.
- Update the TUI dashboard.
- Ask for approval when needed.
- Stop with a clear blocked reason.
- Write an audit event for each meaningful action.

## Automation Boundary

The desired harness should prefer structured APIs, local databases, and accessibility-tree integrations.

OCR and coordinate-based clicking may be useful as last-resort fallbacks, but they should be supervised, auditable, and blocked for irreversible actions such as signing notes, submitting claims, exporting PHI, deleting records, or finalizing billing.

## Non-Goals

This is not a request for autonomous diagnosis, autonomous billing submission, clinical decision-making, or a production EHR.

The goal is a safe agentic harness for supervised medical workflows, with local-first storage, explicit compliance gates, and terminal-native visual audit.

## Desired Upstream Primitive

The ideal Codex CLI contribution would be a reusable regulated-workflow harness with:

- Typed agent states.
- Inspectable tool calls.
- TUI dashboard hooks.
- Approval gates.
- Policy preflights.
- Local audit events.
- Resumable bounded loops.
- Plugin support for domain-specific tools.
- Clear separation between local tools and outbound model providers.

Medical workflows are the motivating use case, but the same architecture would also help legal, finance, security, and other regulated terminal workflows.
