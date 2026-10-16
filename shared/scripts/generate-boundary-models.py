#!/usr/bin/env python3
"""Export Rust serde DTOs to JSON Schema, then generate consumer models at build time."""
from __future__ import annotations

import argparse
import json
import os
import re
from pathlib import Path
import shutil
import subprocess
import sys

# The pinned Windows embedded interpreter omits the script directory from sys.path.
sys.path.insert(0, str(Path(__file__).resolve().parent))
import kotlin_json

ROOT = Path(__file__).resolve().parents[2]
HEADER = "// @generated from shared/core serde JSON Schema; do not edit.\n"


def reference(schema: dict) -> str:
    return schema["$ref"].rsplit("/", 1)[1]


def ts_type(schema: dict) -> str:
    if isinstance(schema, bool):
        return "unknown" if schema else "never"
    if "$ref" in schema:
        return reference(schema)
    if len(schema.get("allOf", [])) == 1:
        return ts_type(schema["allOf"][0])
    if "enum" in schema:
        return " | ".join(json.dumps(value, ensure_ascii=False) for value in schema["enum"])
    for union in ("anyOf", "oneOf"):
        if union in schema:
            return " | ".join(ts_type(item) for item in schema[union])
    kind = schema.get("type")
    if isinstance(kind, list):
        return " | ".join(ts_type(dict(schema, type=item)) for item in kind)
    if kind == "array":
        items = schema.get('items', {})
        if isinstance(items, list):
            return "[" + ", ".join(ts_type(item) for item in items) + "]"
        return f"Array<{ts_type(items)}>"
    if kind == "object":
        if "properties" in schema:
            return "{ " + "; ".join(f"{json.dumps(name)}: {ts_type(value)}"
                                      for name, value in schema["properties"].items()) + " }"
        additional = schema.get("additionalProperties", {})
        return f"Record<string, {ts_type(additional) if isinstance(additional, dict) else 'unknown'}>"
    return {"string": "string", "integer": "number", "number": "number",
            "boolean": "boolean", "null": "null"}.get(kind, "unknown")


def ts_declaration(name: str, schema: dict) -> str:
    if isinstance(schema, bool):
        return f"export type {name} = {ts_type(schema)};\n"
    if schema.get("type") == "object" and "properties" in schema:
        # These are serialized snapshots: serde emits every DTO field, including null Options.
        fields = "".join(f"  {key}{'?' if isinstance(value, dict) and value.get('x-serde-omitted') else ''}: {ts_type(value)};\n"
                         for key, value in schema["properties"].items())
        return f"export interface {name} {{\n{fields}}}\n"
    return f"export type {name} = {ts_type(schema)};\n"


def kotlin_type(schema: dict) -> str:
    if isinstance(schema, bool):
        return "Any?" if schema else "Nothing"
    if "$ref" in schema:
        return reference(schema)
    if len(schema.get("allOf", [])) == 1:
        return kotlin_type(schema["allOf"][0])
    if "enum" in schema:
        return "String"  # Wire enum values are kept, not normalized by a consumer.
    for union in ("anyOf", "oneOf"):
        if union in schema:
            choices = schema[union]
            real = [item for item in choices if item.get("type") != "null"]
            if len(real) == 1:
                result = kotlin_type(real[0])
                return result if result.endswith("?") or len(real) == len(choices) else result + "?"
            raise ValueError("A tagged union requires an explicit generated sealed model")
    kind = schema.get("type")
    if isinstance(kind, list):
        real = [item for item in kind if item != "null"]
        if len(real) != 1:
            raise ValueError("Unsupported union: " + repr(kind))
        result = kotlin_type(dict(schema, type=real[0]))
        return result if result.endswith("?") or "null" not in kind else result + "?"
    if kind == "array":
        items = schema.get('items', {})
        if isinstance(items, list):
            if len(items) in (2, 3):
                return ("Pair" if len(items) == 2 else "Triple") + "<" + ", ".join(kotlin_type(item) for item in items) + ">"
            raise ValueError("Unsupported tuple size: " + str(len(items)))
        return f"List<{kotlin_type(items)}>"
    if kind == "object":
        additional = schema.get("additionalProperties", {})
        return f"Map<String, {kotlin_type(additional) if isinstance(additional, dict) else 'Any?'}>"
    if kind == "integer":
        return "Long" if schema.get("format") in ("uint64", "int64") else "Int"
    return {"string": "String", "number": "Double", "boolean": "Boolean",
            "null": "Nothing?"}.get(kind, "Any?")


def kotlin_declaration(name: str, schema: dict) -> str:
    if isinstance(schema, bool):
        return f"typealias {name} = {kotlin_type(schema)}\n"
    for union in ("oneOf", "anyOf"):
        variants = schema.get(union, [])
        if variants and all(isinstance(item, dict) and item.get("type") == "object"
                            and item.get("properties") for item in variants):
            return kotlin_union(name, variants)
    if "enum" in schema and all(isinstance(value, str) for value in schema["enum"]):
        variants = []
        for value in schema["enum"]:
            symbol = re.sub(r"[^A-Z0-9_]+", "_", re.sub(r"(?<!^)(?=[A-Z])", "_", value).upper()).strip("_")
            if not symbol or symbol[0].isdigit():
                symbol = "VALUE_" + symbol
            variants.append(f"    {symbol}({json.dumps(value)})")
        return (f"enum class {name}(val wireValue: String) {{\n" + ",\n".join(variants)
                + f";\n    companion object {{\n        fun fromWire(value: String): {name}? =\n"
                + "            entries.firstOrNull { it.wireValue == value }\n    }\n}\n")
    if schema.get("type") == "object" and "properties" in schema:
        if schema.get("x-consumer-view"):
            fields = kotlin_json.fields(schema, kotlin_type)
            return f"data class {name}(\n{fields}\n)" + kotlin_json.reader(name, schema)
        fields = ",\n".join(f"    val `{key}`: {kotlin_type(value)}"
                            for key, value in schema["properties"].items())
        return f"data class {name}(\n{fields}\n)\n"
    return f"typealias {name} = {kotlin_type(schema)}\n"


def kotlin_union(name: str, variants: list[dict]) -> str:
    common = set(variants[0]["properties"])
    for item in variants[1:]:
        common.intersection_update(item["properties"])
    tag = next((key for key in sorted(common) if all(
        len(item["properties"][key].get("enum", [])) == 1
        and isinstance(item["properties"][key]["enum"][0], str) for item in variants)), None)
    if tag is None and not all(len(item["properties"]) == 1 for item in variants):
        raise ValueError("Unsupported union without an unambiguous wire tag: " + name)
    result = f"sealed interface {name}" + (f" {{ val `{tag}`: String }}\n" if tag else "\n")
    labels = set()
    for item in variants:
        props = item["properties"]
        literal = props[tag]["enum"][0] if tag else next(iter(props))
        label = re.sub(r"[^A-Za-z0-9_]", "_", literal)
        if label in labels:
            raise ValueError("Duplicate wire variant in " + name)
        labels.add(label)
        fields = ",\n".join(f"    val `{key}`: {kotlin_type(value)}"
                            for key, value in props.items() if key != tag)
        child = f"{name}_{label}"
        if fields:
            result += f"data class {child}(\n{fields}\n) : {name}"
        else:
            result += f"data object {child} : {name}"
        result += (f" {{ override val `{tag}`: String = {json.dumps(literal)} }}\n" if tag else "\n")
    return result


def outputs(bundle: dict) -> dict[Path, str]:
    result: dict[Path, str] = {}
    kotlin: dict[str, dict] = {}
    for name, schema in sorted(bundle["types"].items()):
        result[ROOT / "shared/schema/types" / f"{name}.schema.json"] = json.dumps(
            schema, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
        kotlin[name] = schema
    for feature, names in sorted(bundle["features"].items()):
        definitions = {}
        for name in names:
            schema = bundle["types"][name]
            definitions.update(schema.get("definitions", {}))
            definitions[name] = schema
        # Root annotations/defaults take precedence over nested schema copies.
        definitions.update({name: bundle["types"][name] for name in names})
        prefix = "".join(word.capitalize() for word in feature.split("-"))
        aliases = {name: prefix + name for name in definitions if name not in names}
        def qualify(value):
            if isinstance(value, list): return [qualify(item) for item in value]
            if not isinstance(value, dict): return value
            result = {key: qualify(item) for key, item in value.items()}
            if "$ref" in result:
                old = reference(result)
                if old in aliases: result["$ref"] = "#/definitions/" + aliases[old]
            return result
        definitions = {aliases.get(name, name): qualify(model) for name, model in definitions.items()}
        declarations = {name: ts_declaration(name, model) for name, model in sorted(definitions.items())}
        combined = HEADER + "".join(declarations.values())
        directory = ROOT / "desktop/src/ipc/generated"
        if len(combined.splitlines()) <= 500:
            result[directory / f"{feature}.ts"] = combined
        else:
            result[directory / f"{feature}.ts"] = HEADER + "".join(
                f"export type {{ {name} }} from './{feature}/{name}';\n" for name in declarations)
            for name, declaration in declarations.items():
                dependencies = sorted(references(definitions[name]) - {name})
                imports = "".join(f"import type {{ {other} }} from './{other}';\n" for other in dependencies)
                result[directory / feature / f"{name}.ts"] = HEADER + imports + declaration
        for name, model in definitions.items():
            kotlin[name] = model
    for name, model in sorted(kotlin.items()):
        result[ROOT / "shared/schema/kotlin" / f"{name}.kt"] = (
            HEADER + "package com.govorun.core.generated\n\n" + kotlin_declaration(name, model))
    return result


def references(schema) -> set[str]:
    if isinstance(schema, dict):
        own = {reference(schema)} if "$ref" in schema else set()
        # Root definitions are sibling declarations, not imports of this type.
        return own | set().union(*(references(value) for key, value in schema.items() if key != "definitions"))
    if isinstance(schema, list):
        return set().union(*(references(value) for value in schema))
    return set()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--schema-input", type=Path, help="Reuse a build's exact Rust export")
    args = parser.parse_args()
    if args.schema_input:
        bundle = json.loads(args.schema_input.read_text())
    else:
        cargo = shutil.which("cargo") or str(Path.home() / ".cargo/bin/cargo")
        completed = subprocess.run([cargo, "run", "--locked", "--offline", "--quiet",
                                    "--manifest-path", str(ROOT / "shared/core/Cargo.toml"),
                                    "--features", "schema", "--example", "export-schema"],
                                   check=True, capture_output=True, text=True, env=dict(os.environ))
        bundle = json.loads(completed.stdout)
    generated = outputs(bundle)
    changed = []
    candidates = list((ROOT / "shared/schema/types").glob("*.schema.json"))
    for directory, pattern in (("desktop/src/ipc/generated", "*.ts"), ("shared/schema/kotlin", "*.kt")):
        candidates += [path for path in (ROOT / directory).rglob(pattern) if path.read_text().startswith(HEADER)]
    for path in candidates:
        if path not in generated:
            changed.append(str(path.relative_to(ROOT)))
            if not args.check: path.unlink()
    for path, content in generated.items():
        if path.exists() and path.read_text() == content:
            continue
        changed.append(str(path.relative_to(ROOT)))
        if not args.check:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content)
    if args.check and changed:
        raise SystemExit("FAIL stale generated models: " + ", ".join(changed))
    print(f"PASS generated boundary models: {len(outputs(bundle))} files; {len(changed)} changed")


if __name__ == "__main__":
    main()
