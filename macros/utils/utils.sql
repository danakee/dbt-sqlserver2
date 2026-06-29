{% macro sqlserver__dateadd(datepart, interval, from_date_or_timestamp) %}
    dateadd({{ datepart }}, {{ interval }}, {{ from_date_or_timestamp }})
{% endmacro %}

{% macro sqlserver__datediff(datepart, startdate, enddate) %}
    datediff({{ datepart }}, {{ startdate }}, {{ enddate }})
{% endmacro %}

{# Requires SQL Server 2022+. On SQL Server 2019 or earlier, DATE_TRUNC is unavailable. #}
{% macro sqlserver__date_trunc(datepart, date) %}
    cast(date_trunc('{{ datepart }}', cast({{ date }} as datetime2)) as date)
{% endmacro %}

{% macro sqlserver__hash(field) %}
    convert(varchar(32), hashbytes('MD5', {{ field }}), 2)
{% endmacro %}

{% macro sqlserver__safe_cast(field, type) %}
    try_cast({{ field }} as {{ type }})
{% endmacro %}

{% macro sqlserver__split_part(string_text, delimiter_text, part_number) %}
    {#-- SQL Server has no native SPLIT_PART; use STRING_SPLIT (SQL Server 2016+) --#}
    (
        select value from (
            select value, row_number() over (order by (select null)) as rn
            from string_split({{ string_text }}, {{ delimiter_text }})
        ) _split
        where rn = {{ part_number }}
    )
{% endmacro %}

{% macro sqlserver__listagg(measure, delimiter_text="','", order_by_clause=none) -%}
    string_agg({{ measure }}, {{ delimiter_text }})
    {%- if order_by_clause -%}
        within group ({{ order_by_clause }})
    {%- endif %}
{%- endmacro %}

{% macro sqlserver__any_value(expression) %}
    min({{ expression }})
{% endmacro %}

{% macro sqlserver__last_day(date, datepart) %}
    eomonth({{ date }})
{% endmacro %}

{% macro sqlserver__position(substring_text, string_text) %}
    charindex({{ substring_text }}, {{ string_text }})
{% endmacro %}

{% macro sqlserver__length(expression) %}
    len({{ expression }})
{% endmacro %}

{% macro sqlserver__cast_bool_to_text(field) %}
    case when {{ field }} then 'true' else 'false' end
{% endmacro %}

{% macro sqlserver__concat(fields) %}
    concat({{ fields | join(', ') }})
{% endmacro %}

{% macro sqlserver__current_timestamp() %}
    sysdatetimeoffset()
{% endmacro %}

{% macro sqlserver__current_timestamp_backcompat() %}
    getdate()
{% endmacro %}

