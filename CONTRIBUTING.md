# Contributing to USahapintar

## Working agreement

1. Start from an up-to-date `main` and use a focused branch.
2. Keep each change scoped to one feature or engineering concern.
3. Read the relevant architecture and engineering documentation before changing boundaries.
4. Add or update tests with behavior changes.
5. Run formatting, linting, type checks, and tests before opening a pull request.
6. Explain trade-offs and known limitations in the pull request.
7. Do not merge failing or unreviewed changes.

## Branch naming

- `feat/<scope>` — product features.
- `fix/<scope>` — bug fixes.
- `chore/<scope>` — tooling and maintenance.
- `docs/<scope>` — documentation-only changes.
- `test/<scope>` — test infrastructure or coverage.

## Commit messages

Use Conventional Commits, for example:

- `feat(auth): add owner login flow`
- `fix(api): reject mismatched account roles`
- `chore(ci): add Rust quality checks`
- `docs(architecture): document module boundaries`

## Definition of Done

- Scope and acceptance criteria are met.
- No unrelated formatting or generated-file noise.
- Frontend: format, lint, typecheck, unit tests, and build pass where applicable.
- Backend: rustfmt, Clippy, unit/integration tests, and build checks pass where applicable.
- Authorization and input validation are tested for security-sensitive behavior.
- Documentation and environment examples are updated.
- No credentials, personal data, or production data are committed.
- CI is green and the diff has been reviewed.

Never commit secrets. Use local environment files and keep them out of version control.