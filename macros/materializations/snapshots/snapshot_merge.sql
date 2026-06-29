{% macro sqlserver__snapshot_merge_sql(target, source, insert_cols) %}
    {%- set insert_cols_csv = insert_cols | join(', ') -%}
    MERGE {{ target.include(database=False) }} AS DBT_INTERNAL_DEST
    USING {{ source }} AS DBT_INTERNAL_SOURCE
    ON DBT_INTERNAL_SOURCE.dbt_scd_id = DBT_INTERNAL_DEST.dbt_scd_id
    WHEN MATCHED
     AND DBT_INTERNAL_DEST.dbt_valid_to IS NULL
     AND DBT_INTERNAL_SOURCE.dbt_change_type IN ('update', 'delete')
        THEN UPDATE SET dbt_valid_to = DBT_INTERNAL_SOURCE.dbt_valid_to
    WHEN NOT MATCHED
     AND DBT_INTERNAL_SOURCE.dbt_change_type = 'insert'
        THEN INSERT ({{ insert_cols_csv }})
        VALUES ({{ insert_cols_csv }});
{% endmacro %}
