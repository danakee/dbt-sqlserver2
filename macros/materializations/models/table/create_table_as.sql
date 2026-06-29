{% macro sqlserver__create_table_as(temporary, relation, compiled_code, language='sql') -%}
  {%- set sql = compiled_code.strip() -%}
  {%- set is_cte = sql.lower().startswith('with') -%}

  {%- if is_cte -%}
    {#
      SQL Server SELECT INTO with a CTE:

        ;WITH cte1 AS (...), cte2 AS (...)
        SELECT * INTO <relation> FROM <final_cte>

      The model SQL ends with a bare `select * from <cte_name>`.
      We replace that final select with SELECT * INTO <relation> FROM <cte_name>
      and prepend a semicolon to terminate any prior statement.

      This avoids nested CTEs (FROM (with ...) AS x) which SQL Server rejects.
    #}
    {%- set final_select_token = '\nselect * from ' -%}
    {%- set final_select_token_upper = '\nSELECT * FROM ' -%}
    {%- if final_select_token in sql -%}
      {%- set parts = sql.rsplit(final_select_token, 1) -%}
      ;{{ parts[0] }}
      SELECT * INTO {{ relation }} FROM {{ parts[1] }}
    {%- elif final_select_token_upper in sql -%}
      {%- set parts = sql.rsplit(final_select_token_upper, 1) -%}
      ;{{ parts[0] }}
      SELECT * INTO {{ relation }} FROM {{ parts[1] }}
    {%- else -%}
      {# Fallback: wrap in outer CTE if we can't find the final select pattern #}
      ;with __dbt_cte_src as (
        {{ sql }}
      )
      SELECT * INTO {{ relation }} FROM __dbt_cte_src
    {%- endif -%}
  {%- else -%}
    SELECT *
    INTO {{ relation }}
    FROM ({{ sql }}) AS __dbt_tmp
  {%- endif %}
{% endmacro %}
