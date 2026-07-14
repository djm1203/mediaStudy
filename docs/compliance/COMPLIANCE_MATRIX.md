---
title: "COMPLIANCE MATRIX"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: ComplianceMatrix
author: Derek Martinez
---

# Compliance Matrix — The Librarian

The Librarian is an **MIT-licensed, open-source, local single-user CLI**. It runs no hosted service, holds no user accounts, and processes no third-party PII on anyone's behalf. As a result, **most data-protection and security-compliance regimes have no default scope here** — there is no controller-of-others'-data relationship and no processing infrastructure to certify. What remains relevant is licensing hygiene, honest documentation of the third-party data flow to Groq, and the internal BEACON governance rules.

## Applicability Overview

| Requirement / regime | Applies? | Status / notes |
|----------------------|----------|----------------|
| **MIT license (own project)** | Yes | `LICENSE` present; `Cargo.toml` declares `license = "MIT"`. Keep the copyright/permission notice intact. |
| **Third-party crate licenses** | Yes | Dependencies pulled from crates.io (see `Cargo.toml`). Should run a license check (e.g. `cargo deny check licenses` or `cargo-license`) to confirm all transitive licenses are MIT-compatible. Not yet automated in CI. |
| **Embedding model license (all-MiniLM-L6-v2 / ONNX)** | Yes | Model downloaded by fastembed at runtime; confirm its upstream license terms permit redistribution/use. |
| **Optional external binaries (FFmpeg, Tesseract)** | Indirect | Invoked as user-installed subprocesses, not bundled or redistributed. Their licenses are the user's concern at install time. |
| **Groq API Terms of Service** | Yes (for data sent) | Any prompt/context/audio sent to Groq is subject to Groq's ToS and privacy terms. Documented in `docs/compliance/DATA_GOVERNANCE.md`; users are responsible for reviewing them. |
| **Copyright of ingested content** | User's responsibility | The user chooses what to ingest (PDFs, videos, web pages, YouTube). Respecting copyright/fair-use and site terms is the user's responsibility, not the tool's. |
| **GDPR / CCPA (data controller of others)** | No (by default) | No collection or processing of other people's personal data; the user is controller of their own local files only. |
| **HIPAA / PCI-DSS / SOC 2** | No | No PHI/cardholder data processing and no hosted service to certify. Would only apply if a user chose to ingest such data — then it is the user's regulatory obligation, and Groq would become a third party they must vet. |
| **Telemetry / tracking consent** | N/A | No telemetry, analytics, or phone-home exists. |

## Internal Governance (BEACON) Rule Compliance

| Rule | Description | Status | Evidence |
|------|-------------|--------|----------|
| R-1.1 | No invented APIs | Enforced | Code review, guardrails |
| R-1.2 | No fabricated paths | Enforced | Code review, guardrails |
| R-2.1 | No behavior change in refactor | Enforced | Test suite (`cargo test`), code review |
| R-3.1 | Read before write | Enforced | Guardrails engine |
| R-8.1 | No hardcoded secrets | Enforced | API key from `config.toml`/env only (`src/config.rs`); code review |
| R-8.2 | No logged secrets | Enforced | Key used only in `Authorization` header, never printed |
| R-8.3 | No committed secrets | Enforced | `.gitignore` excludes `config.toml`, `*.db` |
| R-8.4 | Sanitize/validate user input | Enforced | Panic-guarded PDF parsing; `validate_path` for subprocess args |
| R-10.2 | No push without instruction | Enforced | Guardrails engine, commit policy |
| R-10.5 | Explicit-commit, no AI attribution | Enforced | Commit policy in `CLAUDE.md` |
| R-11.1 | Read governance files first | Enforced | Session protocol |
| R-11.3 | No self-approval | Enforced | PR review policy |

## Recommended Compliance Actions

| Action | Cadence | Owner |
|--------|---------|-------|
| License check on dependency tree (`cargo deny`/`cargo-license`) | On dependency change / before release | Maintainer |
| `cargo audit` for advisories (see `VULNERABILITY_POLICY.md`) | On dependency change / periodically | Maintainer |
| Re-confirm embedding-model + Groq ToS still permit intended use | Annually or on upstream change | Maintainer |
| Verify `.gitignore` still excludes secrets/DBs | On config changes | Maintainer |

## Evidence Collection

- Governance compliance is expressed in `docs/governance/RULESET.md` and enforced behaviorally + via the optional pre-commit hook.
- All changes tracked in git; decisions in `docs/state/DECISIONS.md`; changes in `docs/state/CHANGELOG.md`.
- CI (`.github/workflows/ci.yml`) provides check/fmt/clippy/test evidence per PR.
