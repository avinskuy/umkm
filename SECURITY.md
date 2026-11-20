# Security Policy

## Reporting a vulnerability

Do not publish exploitable vulnerability details in public issues or pull requests.

Report security concerns privately to the project maintainers through a private channel. Include the affected component, impact, reproduction context, and any suggested mitigation. Avoid sharing real user data or credentials.

## Security principles

- Enforce authentication, role checks, and resource ownership on the backend.
- Store passwords using a modern password-hashing algorithm such as Argon2id.
- Keep secrets outside source control.
- Apply rate limits to authentication, OTP, and other abuse-sensitive endpoints.
- Use secure session cookies and appropriate CSRF protections.
- Minimize personal data collection and apply explicit access controls.
- Record sensitive administrative actions in an audit trail.
- Do not expose private business or customer-level data to investors without explicit authorization.

This document is an initial policy and will be expanded before production release.