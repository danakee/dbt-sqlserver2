# dbt-sqlserver2

SQL Server macro assets for dbt Core v2 (Rust/ADBC).

## Status

**Phase 1 complete.** These macros are extracted from the working SQL Server adapter
implementation in [danakee/dbt-core](https://github.com/danakee/dbt-core) on branch
`feature/sqlserver-adapter` (commit `8d688a0b4`).

## About dbt Core v2 Adapters

dbt Core v2 adapters live inside the `dbt-labs/dbt-core` monorepo as contributions
(PRs), not as standalone packages. This repo serves as:

- A staging area for the SQL Server macro assets
- Documentation of the macro implementation
- A reference for the upstream PR to `dbt-labs/dbt-core`

The Rust adapter code (auth, metadata, ADBC driver registration, profile config)
lives in the fork at `danakee/dbt-core` and will be contributed upstream via PR.

## Macro Coverage

- **Adapters:** catalog, columns, indexes, metadata, relation, schema, grants, show
- **Materializations:** view, table, incremental (append/delete+insert/merge), seeds, snapshots (Phase 1/unvalidated), tests
- **Utils:** T-SQL overrides for dateadd, datediff, hash, safe_cast, split_part, listagg, etc.

## Upstream PR

The full SQL Server adapter contribution is tracked at:
`danakee/dbt-core` → `feature/sqlserver-adapter`

Contact: reach out in `#adapter-ecosystem` on dbt Community Slack.
