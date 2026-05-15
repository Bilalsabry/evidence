# Security policy

## Reporting a vulnerability

Please report vulnerabilities through GitHub's [private vulnerability reporting](https://github.com/Bilalsabry/evidence/security/advisories/new) feature. Do **not** file public issues for security vulnerabilities.

You can expect an acknowledgement within 72 hours.

## Threat model (sketch)

`evidence` runs locally on the user's machine and reads documents the user provides. The primary risks:

- **Untrusted PDFs.** The parser must not crash on malformed input or escape into the rest of the system.
- **Untrusted model output.** Citations must resolve to real spans in the index before any text is shown to the user. Unresolved citations cause a refusal, not a guess.
- **Network egress.** By default, `evidence` makes no outbound network calls. Cloud inference is opt-in and clearly indicated in the UI.

A full threat model will live at `docs/THREAT_MODEL.md` before `v0.5.0`.

## Supported versions

Pre-`v1.0.0`: only the latest tagged release is supported.
