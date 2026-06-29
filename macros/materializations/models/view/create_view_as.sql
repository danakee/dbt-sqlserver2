{% macro sqlserver__create_view_as(relation, sql) -%}
  EXEC [{{ relation.database }}].sys.sp_executesql N'CREATE OR ALTER VIEW {{ relation.include(database=False) }} AS
  {{ sql | replace("'", "''") }}'
{% endmacro %}

{% macro sqlserver__create_or_alter_view(relation, sql) -%}
  EXEC [{{ relation.database }}].sys.sp_executesql N'CREATE OR ALTER VIEW {{ relation.include(database=False) }} AS
  {{ sql | replace("'", "''") }}'
{% endmacro %}
