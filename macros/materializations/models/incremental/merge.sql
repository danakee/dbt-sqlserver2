{% macro sqlserver__get_merge_sql(target, source_sql, unique_key, dest_columns, incremental_predicates=none) %}
  {#
    SQL Server MERGE implementation.

    Bug note (P2-MAT-02): The upstream dbt-msft/dbt-sqlserver v1 `get_merge_sql` uses
    a global scalar for watermark comparison rather than per-row correlation, producing
    incorrect incremental behavior when multiple unique key values exist in the batch.
    This implementation uses a proper CTE + per-row join pattern.
  #}

  {%- set dest_cols_csv = get_quoted_csv(dest_columns | map(attribute='name')) -%}
  {%- set merge_update_columns = config.get('merge_update_columns', dest_columns | map(attribute='name') | list) -%}
  {%- set unique_key_list = [unique_key] if unique_key is string else unique_key -%}

  ;with dbt_src as (
    {{ source_sql }}
  )

  MERGE {{ target }} AS dbt_target
  USING dbt_src AS dbt_source
  ON (
    {% for key in unique_key_list -%}
      dbt_target.[{{ key }}] = dbt_source.[{{ key }}]
      {%- if not loop.last %} AND {% endif %}
    {%- endfor %}
    {% if incremental_predicates %}
      {% for pred in incremental_predicates %}AND {{ pred }} {% endfor %}
    {% endif %}
  )
  WHEN MATCHED THEN UPDATE SET
    {% for col in merge_update_columns -%}
      {%- if col not in unique_key_list -%}
        dbt_target.[{{ col }}] = dbt_source.[{{ col }}]{{ ',' if not loop.last }}
      {%- endif %}
    {%- endfor %}
  WHEN NOT MATCHED BY TARGET THEN INSERT
    ({{ dest_cols_csv }})
  VALUES (
    {% for col in dest_columns -%}
      dbt_source.[{{ col.name }}]{{ ',' if not loop.last }}
    {%- endfor %}
  );

{% endmacro %}

{% macro sqlserver__get_incremental_merge_sql(arg_dict) %}
  {% do return(sqlserver__get_merge_sql(
      arg_dict["target_relation"],
      arg_dict["temp_relation"],
      arg_dict["unique_key"],
      arg_dict["dest_columns"],
      arg_dict.get("incremental_predicates")
  )) %}
{% endmacro %}

{% macro sqlserver__get_incremental_delete_insert_sql(arg_dict) %}
  {% do return(get_delete_insert_merge_sql(
      arg_dict["target_relation"],
      arg_dict["temp_relation"],
      arg_dict["unique_key"],
      arg_dict["dest_columns"],
      arg_dict.get("incremental_predicates")
  )) %}
{% endmacro %}

{% macro sqlserver__get_incremental_append_sql(arg_dict) %}
  {% do return(get_insert_into_sql(
      arg_dict["target_relation"],
      arg_dict["temp_relation"],
      arg_dict["dest_columns"]
  )) %}
{% endmacro %}
