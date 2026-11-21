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

## Run locally

From the repository root:

```sh
HOST=127.0.0.1 PORT=5000 cargo run --locked -p usahapintar-api
```

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
