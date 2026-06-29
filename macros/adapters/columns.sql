{% macro sqlserver__get_empty_subquery_sql(select_sql, select_sql_header=none) %}
    with __dbt_sbq_tmp as (
        {{ select_sql }}
    )
    select * from __dbt_sbq_tmp
    where 0 = 1
{% endmacro %}

{% macro sqlserver__get_columns_in_relation(relation) -%}
  {% call statement('get_columns_in_relation', fetch_result=True) %}
    with mapping as (
        select
            row_number() over (partition by object_name(c.object_id) order by c.column_id) as ordinal_position,
            c.name collate database_default as column_name,
            t.name as data_type,
            case
                when (t.name in ('nchar', 'nvarchar', 'sysname') and c.max_length <> -1) then c.max_length / 2
                else c.max_length
            end as character_maximum_length,
            c.precision as numeric_precision,
            c.scale as numeric_scale
        from sys.columns c
        inner join sys.types t
        on c.user_type_id = t.user_type_id
        where c.object_id = object_id('{{ 'tempdb..' ~ relation.include(database=false, schema=false) if '#' in relation.identifier else relation }}')
    )

    select
        column_name,
        data_type,
        character_maximum_length,
        numeric_precision,
        numeric_scale
    from mapping
    order by ordinal_position

  {% endcall %}
  {% set table = load_result('get_columns_in_relation').table %}
  {{ return(sql_convert_columns_in_relation(table)) }}
{% endmacro %}

{% macro sqlserver__get_columns_in_query(select_sql) %}
  {% call statement('get_columns_in_query', fetch_result=True, auto_begin=False) -%}
    with __dbt_sbq as
    (
        {{ select_sql }}
    )
    select top 0 *
    from __dbt_sbq
    where 0 = 1
  {% endcall %}

  {{ return(load_result('get_columns_in_query').table.columns | map(attribute='name') | list) }}
{% endmacro %}

{% macro sqlserver__alter_column_type(relation, column_name, new_column_type) %}
  {#-- SQL Server does not support ALTER COLUMN for type changes in all cases.
       This implementation creates a temp table, drops the original, and recreates. --#}
  {%- set table_name = relation.identifier -%}
  {%- set schema_name = relation.schema -%}

  {% set generate_tmp_relation_script %}
    SELECT STRING_AGG('"' + CAST(c.COLUMN_NAME AS VARCHAR(128)) + '"', ', ')  AS ColumnDef
    FROM INFORMATION_SCHEMA.TABLES t
    JOIN INFORMATION_SCHEMA.COLUMNS c
        ON t.TABLE_SCHEMA = c.TABLE_SCHEMA
        AND t.TABLE_NAME = c.TABLE_NAME
    WHERE t.TABLE_NAME = REPLACE('{{ table_name }}', '"', '')
      AND t.TABLE_SCHEMA = REPLACE('{{ schema_name }}', '"', '')
      AND c.COLUMN_NAME <> REPLACE('{{ column_name }}', '"', '')
  {% endset %}

  {%- set query_result = run_query(generate_tmp_relation_script) -%}
  {%- set query_result_text = query_result.rows[0][0] -%}

  {% set temp_table_name %}
    {{ relation.schema }}.{{ relation.identifier }}_{{ range(1300, 19000) | random }}
  {% endset %}

  {% call statement('create_temp_table') -%}
    SELECT {{ query_result_text }}, CAST([{{ column_name }}] AS {{ new_column_type }}) AS [{{ column_name }}]
    INTO {{ temp_table_name }}
    FROM {{ relation }}
  {%- endcall %}

  {% call statement('drop_table') -%}
    DROP TABLE {{ relation }}
  {%- endcall %}

  {% call statement('rename_temp_table') -%}
    EXEC sp_rename '{{ temp_table_name }}', '{{ relation.identifier }}'
  {%- endcall %}
{% endmacro %}

{% macro sqlserver__alter_relation_add_remove_columns(relation, add_columns, remove_columns) %}
  {% call statement('add_drop_columns') -%}
    {% if add_columns %}
      alter {{ relation.type }} {{ relation }}
      add {% for column in add_columns %}[{{ column.name }}] {{ column.data_type }}{{ ', ' if not loop.last }}{% endfor %};
    {% endif %}

    {% if remove_columns %}
      alter {{ relation.type }} {{ relation }}
      drop column {% for column in remove_columns %}[{{ column.name }}]{{ ',' if not loop.last }}{% endfor %};
    {% endif %}
  {%- endcall -%}
{% endmacro %}
