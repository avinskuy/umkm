# Backend

The API is a Rust/Axum service in the workspace package `usahapintar-api`.

## Runtime configuration

The server reads its bind address from environment variables:

| Variable | Default | Description |
| --- | --- | --- |
| `HOST` | `127.0.0.1` | IP address to bind to. IPv4 and IPv6 literals are supported. |
| `PORT` | `5000` | TCP port. Must be an integer from 1 to 65535. |

If either variable is omitted, its default is used. Invalid values stop startup with a configuration error instead of silently falling back.

The default loopback address keeps the API local to the host. Set `HOST=0.0.0.0` only when the deployment environment requires an externally reachable interface, and protect the service with the intended network controls or reverse proxy. Do not expose the API directly to the public internet without an explicit deployment security review.

The application does not load `.env` files itself. Configure environment variables through the shell, service manager, container runtime, or deployment platform.

## PostgreSQL

The database foundation uses SQLx with PostgreSQL and a Tokio/Rustls runtime.

| Variable | Required | Default | Description |
| --- | --- | --- | --- |
| `DATABASE_URL` | Yes, when database access is initialized | — | PostgreSQL connection URL. Supply it through a secret manager or runtime environment. |
| `DATABASE_MAX_CONNECTIONS` | No | `10` | Pool limit; accepted range is 1–100. |

The database layer is initialized explicitly with `DatabaseConfig::from_env()` and `database::connect()`; the HTTP server does not connect to a database until application startup is deliberately wired to do so. This keeps the current health-only application behavior unchanged.

SQLx migrations are embedded from `backend/migrations` and run explicitly through `database::migrations::run(&pool)`. The directory intentionally has no schema migration yet: domain tables must be introduced only after their data model is approved. Never edit an applied migration; add a new versioned migration instead.

Do not put database credentials in source control, tests, logs, or documentation. CI uses a disposable PostgreSQL service with test-only credentials.

## Run locally

From the repository root:

```sh
HOST=127.0.0.1 PORT=5000 cargo run --locked -p usahapintar-api
```

To exercise PostgreSQL integration locally, create a disposable development database and set `DATABASE_URL` before running the ignored integration test:

```sh
DATABASE_URL='postgres://app_user:local_password@127.0.0.1:5432/usahapintar_test' cargo test --locked -p usahapintar-api --test postgres -- --ignored
```

Use local-only credentials and never reuse production credentials in development or CI.

Health endpoint:

```
http://127.0.0.1:5000/api/v1/health
```

## Quality checks

From the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
```
