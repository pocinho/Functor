# Security Policy

Functors is an early-stage project, but security is still important. This document
explains how to report vulnerabilities, how disclosures are handled, and what
versions are supported.

---

## Supported Versions

Functors is currently in active development. Until the first stable release
(`v1.0.0`), all security issues should be reported regardless of version.

After `v1.0.0`, only the following versions will receive security updates:

| Version | Supported |
|--------|-----------|
| Latest stable (`main`) | Yes |
| Previous stable | Yes |
| Development branch (`develop`) | Yes |
| Older releases | No |

---

## Reporting a Vulnerability

If you discover a security issue, **do not open a public GitHub issue**.

Instead, please report it privately:

- by contacting the maintainers directly through GitHub

Please include:

- A clear description of the vulnerability  
- Steps to reproduce  
- Potential impact  
- Any suggested fixes  

You will receive an acknowledgment within **48 hours**.

---

## Disclosure Policy

We follow a responsible disclosure process:

1. You report the issue privately.
2. We investigate and confirm the vulnerability.
3. We prepare a fix and coordinate a release.
4. We publish a security advisory.
5. You may publicly disclose the issue after the fix is released.

We may request additional time for complex issues.

---

## Scope

Security issues include:

- Memory safety violations  
- Undefined behavior  
- Rendering pipeline crashes caused by untrusted input  
- Notebook runtime vulnerabilities  
- Agent protocol injection or sandbox escapes  
- WASM/WASI execution vulnerabilities  
- File parsing vulnerabilities  
- Any issue that compromises user data or system integrity  

Non-security issues (bugs, feature requests, refactors) should be filed using
GitHub Issues.

---

## Best Practices for Contributors

To help maintain security:

- Avoid unsafe Rust unless necessary  
- Validate all external input  
- Keep dependencies updated  
- Use `clippy` and `rustfmt`  
- Prefer pure functions in MVU code  
- Avoid blocking operations in rendering code  
- Isolate OS-specific logic  

Security is a shared responsibility.

---

If you have questions about this policy, please contact the maintainer.
