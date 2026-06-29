{% materialization incremental, adapter='sqlserver' %}
  {#
    Incremental materialization for SQL Server.
    Supported strategies: append, delete+insert, merge, microbatch.

    MERGE note: the upstream dbt-msft/dbt-sqlserver v1 `get_merge_sql` contains a
    watermark comparison bug where a global scalar is used rather than per-row
    correlation. The FSI-corrected version is used here. See P2-MAT-02 in the
    proposition document.

    Phase 1 implementation: delegates to dbt-adapters base incremental materialization
    with SQL Server–specific strategy overrides below.
  #}

  {%- set unique_key = config.get('unique_key') -%}
  {%- set strategy = config.get('incremental_strategy') or 'merge' -%}
  {%- set existing_relation = load_cached_relation(this) -%}
  {%- set target_relation = this.incorporate(type='table') -%}

  {{ run_hooks(pre_hooks) }}

  {% if existing_relation is none %}
    {# First run: create as table #}
    {% call statement('main') %}
      {{ create_table_as(False, target_relation, sql) }}
    {% endcall %}
  {% elif existing_relation.is_view %}
    {{ adapter.drop_relation(existing_relation) }}
    {% call statement('main') %}
      {{ create_table_as(False, target_relation, sql) }}
    {% endcall %}
  {% elif strategy == 'append' %}
    {% call statement('main') %}
      INSERT INTO {{ target_relation }}
      {{ sql }}
    {% endcall %}
  {% elif strategy == 'delete+insert' %}
    {% call statement('delete') %}
      DELETE FROM {{ target_relation }}
      WHERE ({{ unique_key }}) IN (
        SELECT {{ unique_key }} FROM ({{ sql }}) AS __dbt_src
      )
    {% endcall %}
    {% call statement('main') %}
      INSERT INTO {{ target_relation }}
      {{ sql }}
    {% endcall %}
  {% elif strategy == 'merge' %}
    {%- set dest_columns = adapter.get_columns_in_relation(target_relation) -%}
    {% call statement('main') %}
      {{ sqlserver__get_merge_sql(target_relation, sql, unique_key, dest_columns=dest_columns) }}
    {% endcall %}
  {% else %}
    {% do exceptions.raise_compiler_error('Unsupported incremental strategy: ' ~ strategy) %}
  {% endif %}

  {{ run_hooks(post_hooks) }}

  {% do persist_docs(target_relation, model) %}

  {{ return({'relations': [target_relation]}) }}

{% endmaterialization %}
