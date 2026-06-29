{% macro sqlserver__get_incremental_default_sql(arg_dict) %}
  {% do return(sqlserver__get_incremental_merge_sql(arg_dict)) %}
{% endmacro %}
