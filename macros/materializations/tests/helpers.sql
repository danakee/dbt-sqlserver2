{#
  SQL Server test helper overrides.

  T-SQL does not support:
  1. Boolean expressions as SELECT-list columns (count(*) != 0 as col)
  2. WITH clauses nested inside subqueries or other CTE bodies

  For CTE test SQL, we flatten by splitting on the final SELECT and
  promoting it to a named CTE, so all CTEs are at the top level:

    ;WITH cte1 AS (...), cte2 AS (...), __dbt_test_wrapper AS (final_select)
    SELECT count(*) ... FROM (SELECT * FROM __dbt_test_wrapper) dbt_internal_test
#}

-- funcsign: (string, string, string, string, optional[integer]) -> string
{% macro sqlserver__get_test_sql(main_sql, fail_calc, warn_if, error_if, limit) -%}
  {%- set sql = main_sql.strip() -%}
  {%- set is_cte = sql.lower().startswith('with') -%}
  {%- if is_cte -%}
    {#
      Split the CTE chain on the last newline+select to separate
      CTE definitions from the final SELECT statement.
      rsplit('\nselect', 1) gives us [cte_defs, final_select_body]
    #}
    {%- set parts = sql.rsplit('\nselect', 1) -%}
    {%- if parts | length == 2 -%}
    ;{{ parts[0] }},
    __dbt_test_wrapper as (
      select{{ parts[1] }}
    )
    select
      {{ fail_calc }} as failures,
      case when {{ fail_calc }} {{ warn_if }} then 1 else 0 end as should_warn,
      case when {{ fail_calc }} {{ error_if }} then 1 else 0 end as should_error
    from (
      select * from __dbt_test_wrapper
    ) dbt_internal_test
    {%- else -%}
    select
      {{ fail_calc }} as failures,
      case when {{ fail_calc }} {{ warn_if }} then 1 else 0 end as should_warn,
      case when {{ fail_calc }} {{ error_if }} then 1 else 0 end as should_error
    from (
      {{ sql }}
    ) dbt_internal_test
    {%- endif -%}
  {%- else -%}
    select
      {{ fail_calc }} as failures,
      case when {{ fail_calc }} {{ warn_if }} then 1 else 0 end as should_warn,
      case when {{ fail_calc }} {{ error_if }} then 1 else 0 end as should_error
    from (
      {{ sql }}
      {{ "top " ~ limit if limit != none }}
    ) dbt_internal_test
  {%- endif %}
{%- endmacro %}

-- funcsign: (string) -> string
{% macro sqlserver__get_aggregated_test_sql(main_sql) -%}
    ;with aggregated_data as (
      {{ main_sql }}
    )
    select
      column_name,
      count(*) as failures,
      case when count(*) > 0 then 1 else 0 end as should_warn,
      case when count(*) > 0 then 1 else 0 end as should_error
    from aggregated_data
    group by column_name
    order by column_name
{%- endmacro %}

-- funcsign: (string, string, list[string]) -> string
{% macro sqlserver__get_unit_test_sql(main_sql, expected_fixture_sql, expected_column_names) -%}
;with dbt_internal_unit_test_actual as (
  select
    {% for expected_column_name in expected_column_names %}{{expected_column_name}}{% if not loop.last -%},{% endif %}{%- endfor -%}, {{ dbt.string_literal("actual") }} as {{ adapter.quote("actual_or_expected") }}
  from (
    {{ main_sql }}
  ) _dbt_internal_unit_test_actual
),
dbt_internal_unit_test_expected as (
  select
    {% for expected_column_name in expected_column_names %}{{expected_column_name}}{% if not loop.last -%}, {% endif %}{%- endfor -%}, {{ dbt.string_literal("expected") }} as {{ adapter.quote("actual_or_expected") }}
  from (
    {{ expected_fixture_sql }}
  ) _dbt_internal_unit_test_expected
)
select * from dbt_internal_unit_test_actual
union all
select * from dbt_internal_unit_test_expected
{%- endmacro %}
