{#
  SQL Server snapshot strategy helpers.

  Phase 1 status: UNVALIDATED — no snapshot models exist in the reference EDW.
  Delegates to dbt-adapters default strategy dispatch for 'timestamp' and 'check'.

  Known open items before Phase 2 validation:
  - Composite unique key suffix naming (loop.index vs 0-based) unverified against dbt v2 core
  - snapshot_get_time() fix applied (removed erroneous Jinja evaluation of sysdatetimeoffset)
  - Full snapshot run against a real model required before marking production-ready
#}
