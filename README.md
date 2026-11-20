# USahapintar

**Business Promotion, Intelligence & Investor Networking Platform**

USahapintar connects business owners and investors through business discovery, partnership opportunities, and data-informed business intelligence.

> Project status: foundation phase. Product scope, architecture, and implementation are being developed incrementally.

## Project structure

- `frontend/` — React + TypeScript web application.
- `backend/` — Rust API built with Axum.
- `docs/` — product, architecture, security, and engineering documentation.
- `infra/` — deployment and infrastructure assets, added when needed.
- `scripts/` — repeatable development and maintenance tasks.

## Development principles

- Modular monolith; no premature microservices.
- Feature-oriented frontend modules and domain-oriented backend modules.
- Clear ownership for every file and responsibility.
- Small, reviewable changes with tests.
- No secrets in source control.
- Role and resource authorization enforced by the backend.

## Getting started

See [CONTRIBUTING.md](CONTRIBUTING.md) for the development workflow and the documentation under [docs/](docs/).

The frontend and backend setup instructions will be expanded alongside their first functional modules.