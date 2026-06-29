{% macro sqlserver__information_schema_name(database) -%}
  information_schema
{%- endmacro %}

{% macro sqlserver__list_schemas(database) %}
  {% call statement('list_schemas', fetch_result=True, auto_begin=False) -%}
    USE [{{ database }}];
    select name as [schema]
    from sys.schemas
  {% endcall %}
  {{ return(load_result('list_schemas').table) }}
{% endmacro %}

{% macro sqlserver__check_schema_exists(information_schema, schema) -%}
  {% call statement('check_schema_exists', fetch_result=True, auto_begin=False) -%}
    SELECT count(*) as schema_exist FROM sys.schemas WHERE name = '{{ schema }}'
  {%- endcall %}
  {{ return(load_result('check_schema_exists').table) }}
{% endmacro %}

{% macro sqlserver__list_relations_without_caching(schema_relation) -%}
  {% call statement('list_relations_without_caching', fetch_result=True) -%}
    USE [{{ schema_relation.database }}];
    with base as (
      select
        DB_NAME() as [database],
        t.name as [name],
        SCHEMA_NAME(t.schema_id) as [schema],
        'table' as table_type
      from sys.tables as t
      union all
      select
        DB_NAME() as [database],
        v.name as [name],
        SCHEMA_NAME(v.schema_id) as [schema],
        'view' as table_type
      from sys.views as v
    )
    select * from base
    where [schema] like '{{ schema_relation.schema }}'
  {% endcall %}
  {{ return(load_result('list_relations_without_caching').table) }}
{% endmacro %}

{% macro sqlserver__get_relation_without_caching(schema_relation) -%}
  {% call statement('get_relation_without_caching', fetch_result=True) -%}
    USE [{{ schema_relation.database }}];
    with base as (
      select
        DB_NAME() as [database],
        t.name as [name],
        SCHEMA_NAME(t.schema_id) as [schema],
        'table' as table_type
      from sys.tables as t
      union all
      select
        DB_NAME() as [database],
        v.name as [name],
        SCHEMA_NAME(v.schema_id) as [schema],
        'view' as table_type
      from sys.views as v
    )
    select * from base
    where [schema] like '{{ schema_relation.schema }}'
    and [name] like '{{ schema_relation.identifier }}'
  {% endcall %}
  {{ return(load_result('get_relation_without_caching').table) }}
{% endmacro %}

{% macro sqlserver__get_relation_last_modified(information_schema, relations) -%}
  {%- call statement('last_modified', fetch_result=True) -%}
    select
        o.name as [identifier],
        s.name as [schema],
        o.modify_date as last_modified,
        current_timestamp as snapshotted_at
    from sys.objects o
    inner join sys.schemas s on o.schema_id = s.schema_id and [type] = 'U'
    where (
        {%- for relation in relations -%}
        (upper(s.name) = upper('{{ relation.schema }}') and
            upper(o.name) = upper('{{ relation.identifier }}')){%- if not loop.last %} or {% endif -%}
        {%- endfor -%}
    )
  {%- endcall -%}
  {{ return(load_result('last_modified')) }}
{% endmacro %}

{% macro sqlserver__current_timestamp() -%}
  sysdatetimeoffset()
{%- endmacro %}

{% macro sqlserver__current_timestamp_backcompat() -%}
  getdate()
{%- endmacro %}
