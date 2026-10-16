"""Mechanical readers for normalized Core consumer views (not raw server JSON)."""
import json
import re


def base(schema):
    if '$ref' in schema:
        return schema['$ref'].rsplit('/', 1)[1]
    for union in ('allOf', 'anyOf', 'oneOf'):
        real = [v for v in schema.get(union, []) if v.get('type') != 'null']
        if len(real) == 1:
            return base(real[0])
    return None


def nullable(schema):
    return 'null' in schema.get('type', []) or any(v.get('type') == 'null' for v in schema.get('anyOf', []))


def default(value, schema):
    if value is None:
        return 'null'
    if isinstance(value, dict):
        name = base(schema)
        if name:
            return name + '(' + ', '.join('`'+key+'` = '+default(v, {}) for key, v in value.items()) + ')'
        return 'mapOf(' + ', '.join(json.dumps(k)+' to '+default(v, {}) for k, v in value.items()) + ')'
    if isinstance(value, list):
        return 'listOf(' + ', '.join(default(v, {}) for v in value) + ')'
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False)
    return str(value).lower()


def read(schema, owner, key):
    reference = base(schema)
    getter = lambda name: f'{owner}.{name}({key})'
    actual = schema
    for union in ('allOf', 'anyOf', 'oneOf'):
        if union in actual:
            actual = next(v for v in actual[union] if v.get('type') != 'null')
    kind = actual.get('type')
    if isinstance(kind, list):
        kind = next(v for v in kind if v != 'null')
    if reference:
        value = reference + '.fromJson(' + getter('getJSONObject') + ')'
    elif kind == 'array':
        value = getter('getJSONArray') + '.let { values -> List(values.length()) { index -> ' + read(actual['items'], 'values', 'index') + ' } }'
    elif kind == 'object':
        value = getter('getJSONObject') + '.let { values -> values.keys().asSequence().associateWith { key -> ' + read(actual.get('additionalProperties', {}), 'values', 'key') + ' } }'
    else:
        primitive = {'string': 'getString', 'boolean': 'getBoolean', 'number': 'getDouble', 'integer': 'getLong' if actual.get('format') in ('int64','uint64') else 'getInt'}
        value = getter(primitive.get(kind, 'get'))
    return f'if ({owner}.isNull({key})) null else {value}' if nullable(schema) else value


def fields(schema, kotlin_type):
    defaults = schema.get('x-kotlin-defaults', {})
    entries = []
    for key in schema.get("x-consumer-field-order") or schema["properties"]:
        field = schema["properties"][key]
        value = defaults.get(key)
        if value is None:
            value = default(field['default'], field) if 'default' in field else 'null' if nullable(field) else None
        entries.append((key, field, value))
    # Required fields precede dependent display defaults such as code/periodDays.
    if not schema.get("x-consumer-field-order"):
        entries.sort(key=lambda entry: entry[2] is not None)
    return ',\n'.join(f'    val `{key}`: {kotlin_type(field)}' + (f' = {value}' if value is not None else '') for key, field, value in entries)


def reader(name, schema):
    args = ',\n'.join(f'            `{key}` = {read(field, "value", json.dumps(key))}' for key,field in schema['properties'].items())
    return ' {\n' + writer(schema) + f'    companion object {{\n        fun fromJson(value: org.json.JSONObject): {name} = {name}(\n{args}\n        )\n    }}\n}}\n'


def encode(schema, expression):
    reference = base(schema)
    actual = schema
    for union in ('allOf','anyOf','oneOf'):
        if union in actual:
            actual = next(v for v in actual[union] if v.get('type') != 'null')
    if reference:
        body = expression+'.toJson()'
    elif actual.get('type') == 'array':
        body = 'org.json.JSONArray('+expression+'.map { item -> '+encode(actual['items'],'item')+' })'
    elif actual.get('type') == 'object':
        body = 'org.json.JSONObject('+expression+'.mapValues { (_, item) -> '+encode(actual.get('additionalProperties',{}),'item')+' })'
    else:
        body = expression
    if nullable(schema):
        stripped = dict(schema)
        if isinstance(stripped.get('type'),list):stripped['type']=[t for t in stripped['type'] if t!='null'][0]
        elif 'anyOf' in stripped:stripped=next(v for v in stripped['anyOf'] if v.get('type')!='null')
        return expression+'?.let { item -> '+encode(stripped,'item')+' } ?: org.json.JSONObject.NULL'
    return body


def writer(schema):
    return '    fun toJson(): org.json.JSONObject = org.json.JSONObject()\n' + ''.join(
        '        .put('+json.dumps(key)+', '+encode(field,'`'+key+'`')+')\n' for key,field in schema['properties'].items())
