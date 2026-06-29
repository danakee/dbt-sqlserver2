{% materialization view, adapter='sqlserver' -%}

  {%- set target_relation = this.incorporate(type='view') -%}
  {%- set existing_relation = load_cached_relation(this) -%}

  {{ run_hooks(pre_hooks) }}

  {% call statement('main') %}
    {{ sqlserver__create_or_alter_view(target_relation, sql) }}
  {% endcall %}

  {{ run_hooks(post_hooks) }}

  {{ return({'relations': [target_relation]}) }}

{%- endmaterialization %}
