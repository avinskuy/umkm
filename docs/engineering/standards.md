# Engineering Standards

## General

- Prefer simple, explicit, maintainable solutions.
- One file should have one clear primary responsibility.
- Keep modules cohesive and APIs small.
- Avoid premature abstractions and empty placeholder directories.
- Avoid giant files; split by responsibility when a module becomes difficult to navigate.
- Do not duplicate business rules across frontend and backend. The backend is authoritative.

## Naming

- React components and component files: PascalCase.
- Hooks: `useSomething`.
- TypeScript variables and functions: camelCase.
- Rust modules, functions, and variables: snake_case.
- Rust types and traits: PascalCase.
- Use domain language consistently.

## Frontend

- TypeScript strict mode.
- Feature-first structure under `src/features/<feature>`.
- Reusable, domain-neutral primitives under `src/shared/ui`.
- Route composition under `src/app`.
- API access and external integrations under explicit service modules.
- Prefer CSS Modules for component/page styles.
- Keep global CSS limited to design tokens, reset, and global document rules.
- Avoid one application-wide stylesheet containing feature-specific selectors.
- Components should not own unrelated network, domain, and presentation concerns.
- Provide accessible labels, keyboard behavior, and clear loading/error/empty states.

## Backend

- Keep handlers thin: parse transport input, call application behavior, map results to HTTP.
- Validate untrusted input at boundaries.
- Enforce authorization and resource ownership in backend application paths.
- Use typed errors and predictable API responses.
- Avoid `unwrap()` and `expect()` in normal request paths.
- Keep database queries and external provider clients behind infrastructure boundaries.
- Never return internal secrets or sensitive fields.

## Styling ownership

- Global tokens: `src/styles/tokens/`.
- Global reset/document styles: `src/styles/base/`.
- Component styles: colocated `.module.css`.
- Feature page styles: colocated with the owning page/feature.
- Do not create a new global stylesheet to fix a local component issue.

## Testing

- Unit-test domain logic and reusable UI behavior.
- Integration-test API, persistence, and authorization boundaries.
- Add regression tests for every fixed bug.
- Test both allowed and denied role/resource access.
- E2E-test critical user journeys when those journeys exist.

## Quality gates

Run the relevant checks before a pull request:
- Frontend formatting, lint, typecheck, tests, and production build.
- Backend rustfmt, Clippy with warnings denied, tests, and build.
- Review dependency changes and generated lockfiles.
- CI must be green before merge.