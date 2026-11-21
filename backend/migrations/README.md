# Database migrations

This directory is intentionally free of schema migrations while the domain model is
being finalized. The SQLx migrator is wired and exercised by the PostgreSQL
integration test; the first versioned SQL migration should be added alongside
the first approved domain schema.

Never edit a migration after it has been applied to a shared database. Add a new
versioned migration for every subsequent schema change.
