{% materialization table, adapter='sqlserver' %}

  {%- set existing_relation = load_cached_relation(this) -%}
  {%- set target_relation = this.incorporate(type='table') -%}
  {%- set intermediate_relation = make_intermediate_relation(target_relation) -%}
  {%- set preexisting_intermediate_relation = load_cached_relation(intermediate_relation) -%}
  {%- set backup_relation_type = 'table' if existing_relation is none else existing_relation.type -%}
  {%- set backup_relation = make_backup_relation(target_relation, backup_relation_type) -%}
  {%- set preexisting_backup_relation = load_cached_relation(backup_relation) -%}

  {{ run_hooks(pre_hooks) }}

  -- drop any pre-existing backup and intermediate relations
  {% if preexisting_intermediate_relation is not none %}
    {{ adapter.drop_relation(preexisting_intermediate_relation) }}
  {% endif %}
  {% if preexisting_backup_relation is not none %}
    {{ adapter.drop_relation(preexisting_backup_relation) }}
  {% endif %}

  -- build the new table in an intermediate relation, then swap
  {% call statement('main') %}
    {{ create_table_as(False, intermediate_relation, sql) }}
  {% endcall %}

  -- swap target with intermediate
  {% if existing_relation is not none %}
    {{ adapter.rename_relation(existing_relation, backup_relation) }}
  {% endif %}
  {{ adapter.rename_relation(intermediate_relation, target_relation) }}

  -- cleanup backup
  {% if existing_relation is not none %}
    {{ adapter.drop_relation(backup_relation) }}
  {% endif %}

  {{ run_hooks(post_hooks) }}

  {% do persist_docs(target_relation, model) %}

  {{ return({'relations': [target_relation]}) }}

{% endmaterialization %}
