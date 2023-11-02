#!/usr/bin/env python3
import subprocess
import json
from pprint import pprint


def from_camel_case(name):
    """Convert a camelCase identifier to snake case."""
    l = len(name)
    name += "X"
    res = ""
    start = 0
    for i in range(len(name)):
        if i > 0 and name[i].isupper():
            res += name[start:i].lower()
            if i != l:
                res += "_"
            start = i
    return res


def generate_method(method):
    assert method["paramStructure"] == "by-position"
    name = method["name"]
    params = method["params"]
    args_typed = ", ".join(
        [
            from_camel_case(param["name"])
            + ": "
            + (decode_type(param["schema"]) or "Any")
            for param in params
        ]
    )
    args = ", ".join([from_camel_case(param["name"]) for param in params])
    result_type = decode_type(method["result"]["schema"])
    print(f"def {name}({args_typed}) -> {result_type}:")
    if "description" in method:
        description = method["description"]
        if "\n" in description:
            print(f'    """{method["description"].lstrip()}\n    """')
        else:
            print(f'    """{method["description"].lstrip()}"""')
    print(f"    rpc_call({args})")
    print()


def generate_openrpc_methods(openrpc_spec):
    for method in openrpc_spec["methods"]:
        generate_method(method)


def decode_type(property_desc):
    if "anyOf" in property_desc:
        t = property_desc["anyOf"]
        assert len(t) == 2
        assert t[1] == {"type": "null"}
        ref = t[0]["$ref"]
        assert ref.startswith("#/components/schemas/")
        return f'Optional["{ref.removeprefix("#/components/schemas/")}"]'
    elif "$ref" in property_desc:
        t = property_desc["$ref"]
        assert t.startswith("#/components/schemas/")
        t = t.removeprefix("#/components/schemas/")
        return f'"{t}"'
    elif property_desc["type"] == "null":
        return "None"
    elif "null" in property_desc["type"]:
        assert len(property_desc["type"]) == 2
        assert property_desc["type"][1] == "null"
        property_desc["type"] = property_desc["type"][0]
        t = decode_type(property_desc)
        if t:
            return f"Optional[{t}]"
    elif property_desc["type"] == "boolean":
        return "bool"
    elif property_desc["type"] == "integer":
        return "int"
    elif property_desc["type"] == "number" and property_desc["format"] == "double":
        return "float"
    elif property_desc["type"] == "string":
        return "str"
    elif property_desc["type"] == "array":
        if isinstance(property_desc["items"], list):
            items_desc = ", ".join(decode_type(x) for x in property_desc["items"])
            return f"Tuple[{items_desc}]"
        else:
            items_type = decode_type(property_desc["items"])
            return f"list[{items_type}]"
    elif "additionalProperties" in property_desc:
        additional_properties = property_desc["additionalProperties"]
        return f"dict[Any, {decode_type(additional_properties)}]"
    return None


def generate_variant(variant) -> str:
    """Prints generated type for enum variant.

    Returns the name of the generated type.
    """
    assert variant["type"] == "object"
    kind = variant["properties"]["kind"]
    assert kind["type"] == "string"
    assert len(kind["enum"]) == 1
    kind_name = kind["enum"][0]
    kind_name = kind_name[0].upper() + kind_name[1:]

    print(f"    @dataclass(kw_only=True)")
    print(f"    class {kind_name}:")
    print(f"        kind: str = \"{kind_name}\"")
    for property_name, property_desc in variant["properties"].items():
        property_name = from_camel_case(property_name)
        if property_name == "kind":
            continue
        if t := decode_type(property_desc):
            print(f"        {property_name}: {t}")
        else:
            print("# TODO")
            pprint(property_name)
            pprint(property_desc)
    print()

    return kind_name


def generate_type(type_name, schema):
    if "oneOf" in schema:
        if all(x["type"] == "string" for x in schema["oneOf"]):
            # Simple enumeration consisting only of various string types.
            print(f"class {type_name}(Enum):")
            for x in schema["oneOf"]:
                for e in x["enum"]:
                    print(f'    {from_camel_case(e).upper()} = "{e}"')
        else:
            # Union type.
            namespace = f"{type_name}Enum"
            print(f"class {namespace}:")
            kind_names = [f"{namespace}.{generate_variant(x)}" for x in schema["oneOf"]]

            print(f"{type_name}: TypeAlias = {' | '.join(kind_names)}")
    elif schema["type"] == "string":
        print(f"class {type_name}(Enum):")
        for e in schema["enum"]:
            print(f'    {from_camel_case(e).upper()} = "{e}"')
    else:
        print("@dataclass(kw_only=True)")
        print(f"class {type_name}:")
        for property_name, property_desc in schema["properties"].items():
            property_name = from_camel_case(property_name)
            if decode_type(property_desc):
                print(f"    {property_name}: {decode_type(property_desc)}")
            else:
                print(f"# TODO {property_name}")
                pprint(property_desc)

    print()


def generate_openrpc_types(openrpc_spec):
    for type_name, schema in openrpc_spec["components"]["schemas"].items():
        generate_type(type_name, schema)


def main():
    openrpc_spec = json.loads(
        subprocess.run(
            ["deltachat-rpc-server", "--openrpc"], capture_output=True
        ).stdout
    )
    print("from dataclasses import dataclass")
    print("from enum import Enum")
    print("from typing import TypeAlias, Union, Optional, Tuple, Any")
    generate_openrpc_types(openrpc_spec)
    generate_openrpc_methods(openrpc_spec)


if __name__ == "__main__":
    main()
