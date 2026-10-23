"""Reads the SmokeLounge.AOtomation message definitions (C#, WTFPL) that ship in the AOSharp sources
and writes a JSON schema of the protocol: `crates/mao-net/schema/protocol.json`.

    python tools/gen_protocol_schema.py <path to .../SmokeLounge/AOtomation/Messaging> [out.json]

The schema holds facts about the wire format only (message numbers, field order, field types,
conditions); no code of those projects is copied. mao-net's `schema` module interprets it.

Per class: `fields` in `[AoMember(n)]` order, each with a type, and optionally
  size   how an array / string is sized: byte | int16 | int32 | x3f1 | nullterm | none
  fixed  a fixed length (IsFixedSize / FixedSizeLength)
  flag   the member's value is kept as a flag under this name (`[AoFlags]`)
  when   conditions (`[AoUsesFlags]`): [{flag, type, any: [values]}], the first that matches picks the type
"""
import json
import re
import sys
from pathlib import Path

root = Path(sys.argv[1])
out_path = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(__file__).resolve().parent.parent / "crates/mao-net/schema/protocol.json"


def strip_comments(text):
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    return re.sub(r"//[^\n]*", "", text)


def match_brace(text, start):
    """Index of the brace closing the one at `start`."""
    depth = 0
    for i in range(start, len(text)):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return i
    raise ValueError("unbalanced braces")


def split_top(text, sep=","):
    parts, depth, cur = [], 0, ""
    for ch in text:
        if ch in "([{<":
            depth += 1
        elif ch in ")]}>":
            depth -= 1
        if ch == sep and depth == 0:
            parts.append(cur)
            cur = ""
        else:
            cur += ch
    if cur.strip():
        parts.append(cur)
    return [p.strip() for p in parts]


SIZES = {"Byte": "byte", "Int16": "int16", "Int32": "int32", "X3F1": "x3f1", "NullTerminated": "nullterm", "NoSerialization": "none"}


def parse_attribute(text):
    """`AoMember(0, SerializeSize = ArraySizeType.X3F1)` -> (name, [args])."""
    m = re.match(r"(\w+)\s*(?:\((.*)\))?\s*$", text.strip(), flags=re.S)
    if not m:
        return None
    return m.group(1), split_top(m.group(2) or "")


def parse_body(body):
    """Members of a class body: [(attributes, type, name)] and nested type declarations."""
    members, i, attrs = [], 0, []
    nested = []
    while i < len(body):
        c = body[i]
        if c == "[":
            depth, j = 0, i
            while j < len(body):
                if body[j] == "[":
                    depth += 1
                elif body[j] == "]":
                    depth -= 1
                    if depth == 0:
                        break
                j += 1
            for a in split_top(body[i + 1:j]):
                p = parse_attribute(a)
                if p:
                    attrs.append(p)
            i = j + 1
            continue
        m = re.match(r"\s*(?:public|internal|protected)\s+(?:(?:abstract|sealed|static|partial)\s+)*(class|struct|enum|interface)\s+(\w+)", body[i:])
        if m and (i == 0 or body[i - 1] in " \t\r\n;{}]"):
            brace = body.index("{", i)
            end = match_brace(body, brace)
            nested.append((m.group(1), m.group(2), body[i:brace], body[brace + 1:end]))
            i = end + 1
            attrs = []
            continue
        m = re.match(r"\s*public\s+(?:virtual\s+|override\s+)?([\w<>\[\]?,.\s]+?)\s+(\w+)\s*\{\s*get;\s*(?:private\s+|protected\s+)?set;\s*\}", body[i:])
        if m:
            members.append((attrs, " ".join(m.group(1).split()), m.group(2)))
            attrs = []
            i += m.end()
            continue
        if c in "{":
            # a method / constructor body: skip it
            i = match_brace(body, i) + 1
            attrs = []
            continue
        if c == ";":
            attrs = []
        i += 1
    return members, nested


def member_record(attrs, ctype, name):
    rec = {"name": name, "type": ctype}
    order = None
    when = []
    for attr, args in attrs:
        if attr == "AoMember":
            order = int(args[0])
            for a in args[1:]:
                k, _, v = a.partition("=")
                k, v = k.strip(), v.strip()
                if k == "SerializeSize":
                    rec["size"] = SIZES[v.split(".")[-1]]
                elif k == "FixedSizeLength":
                    rec["fixed"] = int(v)
                elif k == "IsFixedSize":
                    if v == "true" and "fixed" not in rec:
                        rec["fixed"] = -1
                elif k == "Offset":
                    rec["offset"] = int(v)
                else:
                    rec.setdefault("other", {})[k] = v
        elif attr == "AoFlags":
            rec["flag"] = args[0].strip('"')
        elif attr == "AoUsesFlags":
            flag = args[0].strip().strip('"')
            t = re.match(r"typeof\((.*)\)", args[1].strip()).group(1)
            crit = args[2].strip().split(".")[-1]
            vals = [int(v.strip().replace("(int)", ""), 0) if re.fullmatch(r"-?(0x[0-9a-fA-F]+|\d+)", v.strip().replace("(int)", "")) else v.strip() for v in re.findall(r"\{([^}]*)\}", args[3])[0].split(",") if v.strip()] if len(args) > 3 else []
            when.append({"flag": flag, "type": t, "criteria": crit, "values": vals})
        elif attr == "AoKnownType":
            rec.setdefault("known", []).append(args)
    if when:
        rec["when"] = when
    return order, rec


classes, enums, contracts = {}, {}, {}


def add_type(kind, name, header, body):
    if kind == "enum":
        under = "int"
        m = re.search(r":\s*(\w+)", header)
        if m:
            under = m.group(1)
        values, nxt = {}, 0
        for item in split_top(body):
            item = item.strip()
            if not item:
                continue
            k, _, v = item.partition("=")
            k, v = k.strip(), v.strip()
            if v:
                v = re.sub(r"\(int\)|\(byte\)|\(short\)", "", v).strip()
                try:
                    nxt = int(v, 0)
                except ValueError:
                    values[k] = None
                    continue
            values[k] = nxt
            nxt += 1
        enums[name] = {"under": under, "values": {k: v for k, v in values.items() if v is not None}}
        return
    if kind == "interface":
        classes.setdefault(name, {"fields": [], "interface": True})
        return
    members, nested = parse_body(body)
    fields = []
    for attrs, ctype, mname in members:
        order, rec = member_record(attrs, ctype, mname)
        if order is None:
            continue
        fields.append((order, rec))
    fields.sort(key=lambda f: f[0])
    base = None
    m = re.search(r":\s*([\w.<>, ]+)", header)
    if m:
        base = m.group(1).split(",")[0].strip()
    cls = {"base": base, "fields": [f for _, f in fields]}
    classes[name] = cls
    for nk, nn, nh, nb in nested:
        add_type(nk, f"{name}.{nn}", nh, nb)
        # nested names are also reachable by their short name
        if nn not in classes and nn not in enums:
            if nk == "enum":
                enums[nn] = enums[f"{name}.{nn}"]
            else:
                classes[nn] = classes[f"{name}.{nn}"]


for path in sorted(root.rglob("*.cs")):
    if "Serialization" in path.parts:
        continue
    text = strip_comments(path.read_text(encoding="utf-8-sig", errors="replace"))
    contract_attrs = []
    # top-level types of the file (inside the namespace block)
    for m in re.finditer(r"((?:\[[^\]\n]*\]\s*)*)\s*public\s+(?:(?:abstract|sealed|static|partial)\s+)*(class|struct|enum|interface)\s+(\w+(?:<[^>]*>)?)([^{;]*)\{", text):
        kind, name, header = m.group(2), re.sub(r"<.*>", "", m.group(3)), m.group(4)
        brace = m.end() - 1
        end = match_brace(text, brace)
        body = text[brace + 1:end]
        # nested types are handled by add_type; skip matches that sit inside an earlier class body
        add_type(kind, name, header, body)
        for a in re.findall(r"AoContract\(\s*\(?(?:int\)\s*)?([\w.]+)\s*\)", m.group(1)):
            contracts[name] = a

# keep the top-level pass honest: a nested class was matched twice; the first (qualified) form is the same
for name in list(classes):
    pass

# Enums defined elsewhere in the sources (Stat, MovementAction, Profession ...) give the width of their fields.
extra = Path(sys.argv[3]) if len(sys.argv) > 3 else root.parents[3] / "GameData"
if extra.exists():
    for path in sorted(extra.rglob("*.cs")):
        text = strip_comments(path.read_text(encoding="utf-8-sig", errors="replace"))
        for m in re.finditer(r"public\s+enum\s+(\w+)([^{;]*)\{", text):
            if m.group(1) in enums:
                continue
            brace = m.end() - 1
            add_type("enum", m.group(1), m.group(2), text[brace + 1:match_brace(text, brace)])

kinds = {}
src = (root / "Messages/N3MessageType.cs").read_text(encoding="utf-8-sig")
for name, val in re.findall(r"(\w+)\s*=\s*(0x[0-9A-Fa-f]+)", src):
    kinds[name] = int(val, 16)
messages = {}
for cname, contract in contracts.items():
    short = contract.split(".")[-1]
    if short in kinds:
        messages[f"{kinds[short]:08x}"] = {"class": cname, "name": short}

def resolve(v):
    """`(int)GenericCmdAction.Repair` / `Type.Member` -> its number."""
    if isinstance(v, str):
        ref = v.replace("(int)", "").strip()
        if "." in ref:
            enum, member = ref.rsplit(".", 1)
            for key in (enum, enum.split(".")[-1]):
                if key in enums and member in enums[key]["values"]:
                    return enums[key]["values"][member]
    return v


for cls in classes.values():
    for field in cls["fields"]:
        for w in field.get("when", []):
            w["values"] = [resolve(v) for v in w["values"]]

schema = {"classes": classes, "enums": enums, "n3": messages}
out_path.parent.mkdir(parents=True, exist_ok=True)
out_path.write_text(json.dumps(schema, indent=1, sort_keys=True), encoding="utf-8")
print(f"{len(classes)} classes, {len(enums)} enums, {len(messages)} N3 messages -> {out_path}")
