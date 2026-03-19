# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

## Reporting a Vulnerability

If you discover a security vulnerability, please report it responsibly:

1. **Do not** open a public GitHub issue
2. Email security concerns to the maintainers
3. Include a description of the vulnerability and steps to reproduce
4. Allow reasonable time for a fix before public disclosure

We aim to acknowledge reports within 48 hours and provide a fix or mitigation plan within 7 days.

## Security Measures

- All CI actions use hash-pinned versions
- Dependencies are audited via `cargo-deny` on every PR
- Automated dependency updates via Renovate
- License compliance checked on every build
