# Security Policy

AICAD is an evolving pre-1.0 project. Security fixes are handled on a best-effort basis; this repository does not currently publish a support SLA or guaranteed response window.

## Reporting a vulnerability

Do not publish exploit details, credentials, private data, or a working proof-of-concept in a normal public issue.

Use GitHub's private vulnerability-reporting mechanism for this repository if it is available. If that mechanism is not available, open a minimal issue stating that you need to report a security concern privately, without including sensitive technical details, so the repository owner can establish an appropriate private channel.

Include, once a private channel is available:

- affected commit/version;
- affected component;
- impact and prerequisites;
- minimal reproduction information;
- any suggested mitigation, if known.

## Scope

Security reports may include parser/compiler crashes reachable from untrusted source, unsafe native/kernel boundary behavior, artifact/path handling flaws, CI/repository automation vulnerabilities, or other defects that can cross a trust boundary.

Ordinary correctness bugs, modeling limitations, numerical robustness issues without a security impact, and unsupported roadmap capabilities should use the normal bug-report path described in [`CONTRIBUTING.md`](CONTRIBUTING.md).
