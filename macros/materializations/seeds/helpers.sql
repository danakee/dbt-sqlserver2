{% macro sqlserver__get_binding_char() %}
  {{ return('?') }}
{% endmacro %}

{% macro sqlserver__get_batch_size() %}
  {{ return(400) }}
{% endmacro %}

{% macro sqlserver__calc_batch_size(num_columns) %}
    {#
        SQL Server allows for a max of 2100 parameters in a single statement.
        Reduce the batch size if needed so total parameters stay under 2100.
    #}
    {% set max_batch_size = get_batch_size() %}
    {% set calculated_batch = ((2100 / num_columns) - 1) | int %}
    {% set batch_size = [max_batch_size, calculated_batch] | min %}
    {{ return(batch_size) }}
{% endmacro %}

{% macro sqlserver__render_value(val, col_type) %}
    {#
        Render a seed value as an inline SQL literal.
        Parameter binding is not used because the ADBC mssql driver renders
        Python booleans as 'false'/'true' which T-SQL rejects (ErrorNumber 207).
        Rendering all values inline avoids the binding mismatch entirely.
    #}
    {% if val is none or val == '' %}
        NULL
    {% elif col_type and col_type.lower() == 'bit' %}
        {# Boolean: render as 0/1 #}
        {% if val == true or val == 'True' or val == 'true' or val == '1' or val == 1 %}
            1
        {% else %}
            0
        {% endif %}
    {% elif val is number %}
        {{ val }}
    {% else %}
        '{{ val | replace("'", "''") }}'
    {% endif %}
{% endmacro %}

{% macro sqlserver__load_csv_rows(model, agate_table) %}
    {#
        SQL Server seed row loader — renders all values as inline SQL literals.

        The ADBC mssql driver renders Python booleans as 'false'/'true' string
        literals when using parameter binding (?), which T-SQL rejects as invalid
        column names (ErrorNumber 207). Rendering values inline avoids this entirely.
    #}
    {% set cols_sql = get_seed_column_quoted_csv(model, agate_table.column_names) %}
    {% set batch_size = sqlserver__calc_batch_size(agate_table.column_names | length) %}
    {% set statements = [] %}

    {# Pre-compute column types for the full table #}
    {% set col_types = [] %}
    {% for col_name in agate_table.column_names %}
        {% set col_type = adapter.convert_type(agate_table, loop.index0) %}
        {% do col_types.append(col_type) %}
    {% endfor %}

    {% for chunk in agate_table.rows | batch(batch_size) %}
        {% set row_sqls = [] %}

        {% for row in chunk %}
            {% set col_sqls = [] %}
            {% for col_idx in range(agate_table.column_names | length) %}
                {% set val = row[col_idx] %}
                {% set col_type = col_types[col_idx] %}
                {% do col_sqls.append(sqlserver__render_value(val, col_type)) %}
            {% endfor %}
            {% do row_sqls.append('(' ~ col_sqls | join(', ') ~ ')') %}
        {% endfor %}

        {% set sql %}
            insert into {{ this.render() }} ({{ cols_sql }}) values
            {{ row_sqls | join(',\n') }}
        {% endset %}

        {% do adapter.add_query(sql, abridge_sql_log=True) %}

        {% if loop.index0 == 0 %}
            {% do statements.append(sql) %}
        {% endif %}
    {% endfor %}

    {{ return(statements[0] if statements else '') }}
{% endmacro %}
