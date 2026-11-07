"""Blender GLSL export from the Slang shader sources.

slangc emits GLSL 450 with Vulkan layouts and mangled names; this module strips the layouts
Blender declares itself through GPUShaderCreateInfo, restores the plain resource names the
addons bind by, and records those resources in a bindings dict."""

import json
import os
import re
import subprocess

SHADER_ROOT = "shaders"
IMAGE_FORMATS = {"rg16f": "RG16F"}
ROW_MAJOR_DEFAULT = "layout(row_major) uniform;"

_DECL_PENDING_LAYOUT = re.compile(r"^layout\((.*)\)\s*$")
_BLOCK_START = re.compile(r"^(?:layout\((?P<layout>[^)]*)\)\s*)?uniform\s+block_(?P<type>\w+)\s*$")
_BLOCK_END = re.compile(r"^\}\s*(\w+)\s*;\s*$")
_SAMPLER = re.compile(r"^uniform\s+sampler([23])D\s+(\w+)\s*;\s*$")
_IMAGE = re.compile(r"^uniform\s+(?:writeonly\s+|readonly\s+)?image([23])D\s+(\w+)\s*;\s*$")
_STAGE_IO = re.compile(r"^(in|out)\s+\w+\s+(\w+)\s*;\s*$")
_LOCAL_SIZE = re.compile(
    r"^layout\(local_size_x\s*=\s*(\d+),\s*local_size_y\s*=\s*(\d+),\s*local_size_z\s*=\s*(\d+)\)\s*in\s*;\s*$"
)
_MEMBER = re.compile(r"^\s*(\w[\w\s]*?)\s+(\w+)(\[\d+\])?\s*;\s*$")


def slang_root() -> str:
    root = os.environ.get("SLANG_ROOT") or os.path.join(os.path.expanduser("~"), ".local", "slang")
    if not os.path.isfile(os.path.join(root, "bin", "slangc")):
        raise SystemExit(f"slangc not found under {root}; set SLANG_ROOT")
    return root


def generate_glsl(entry: str, stage: str, defines: list[str], repo_root: str) -> str:
    """Run slangc on shaders/<entry> and return the GLSL text."""
    command = [
        os.path.join(slang_root(), "bin", "slangc"),
        os.path.join(repo_root, SHADER_ROOT, entry),
        "-I", os.path.join(repo_root, SHADER_ROOT),
        "-target", "glsl",
        "-stage", stage,
        "-entry", "main",
    ]
    command += [f"-D{name}" for name in defines]
    result = subprocess.run(command, capture_output=True, text=True)
    if result.returncode != 0:
        raise SystemExit(f"slangc failed for {entry}:\n{result.stderr}")
    return result.stdout


def demangle(name: str) -> str:
    return re.sub(r"_\d+$", "", name)


def _binding_of(pending: list[str]) -> int | None:
    for qualifier in pending:
        m = re.search(r"\bbinding\s*=\s*(\d+)", qualifier)
        if m:
            return int(m.group(1))
    return None


def _location_of(pending: list[str]) -> int | None:
    for qualifier in pending:
        m = re.search(r"\blocation\s*=\s*(\d+)", qualifier)
        if m:
            return int(m.group(1))
    return None


def _image_format_of(pending: list[str]) -> str:
    for qualifier in pending:
        if qualifier in IMAGE_FORMATS:
            return IMAGE_FORMATS[qualifier]
    raise SystemExit(f"unsupported image format in {pending}")


def _block_members(lines: list[str]) -> list[str]:
    members = []
    for line in lines:
        m = _MEMBER.match(line)
        if m:
            members.append(f"{m.group(1)} {demangle(m.group(2))}{m.group(3) or ''}")
    return members


class _Renames:
    def __init__(self) -> None:
        self.words: dict[str, str] = {}

    def add(self, mangled: str, plain: str) -> None:
        if mangled != plain:
            self.words[mangled] = plain

    def apply(self, text: str) -> str:
        if not self.words:
            return text
        pattern = re.compile(r"\b(" + "|".join(re.escape(w) for w in self.words) + r")\b")
        return pattern.sub(lambda m: self.words[m.group(1)], text)


def convert_to_blender_dialect(glsl_text: str, output_names: list[str]) -> tuple[list[str], dict]:
    """Drop the Vulkan declarations Blender supplies, demangle the bound names, collect bindings."""
    bindings: dict = {
        "samplers": [],
        "images": [],
        "ubos": [],
        "push_constants": [],
        "inputs": [],
        "outputs": [],
    }
    renames = _Renames()
    outputs_by_location: dict[int, str] = {}
    kept: list[str] = []
    pending: list[str] = []
    lines = glsl_text.split("\n")

    i = 0
    while i < len(lines):
        line = lines[i].rstrip()
        stripped = line.strip()
        i += 1

        if stripped.startswith("#version") or stripped.startswith("#line") or stripped == "layout(row_major) buffer;":
            continue
        if stripped == ROW_MAJOR_DEFAULT:
            continue

        local_size = _LOCAL_SIZE.match(stripped)
        if local_size:
            bindings["local_size"] = [int(local_size.group(k)) for k in (1, 2, 3)]
            continue

        layout = _DECL_PENDING_LAYOUT.match(stripped)
        if layout:
            pending.append(layout.group(1).strip())
            continue

        block = _BLOCK_START.match(stripped)
        if block:
            if block.group("layout"):
                pending.append(block.group("layout").strip())
            body: list[str] = []
            while i < len(lines) and not _BLOCK_END.match(lines[i].strip()):
                body.append(lines[i])
                i += 1
            instance = _BLOCK_END.match(lines[i].strip()).group(1)
            i += 1
            type_name = block.group("type")
            record = {"type": demangle(type_name), "name": demangle(instance)}
            renames.add(type_name, record["type"])
            renames.add(instance, record["name"])
            if any(q == "push_constant" for q in pending):
                record["members"] = _block_members(body)
                bindings["push_constants"].append(record)
            else:
                record["binding"] = _binding_of(pending)
                bindings["ubos"].append(record)
            pending = []
            continue

        sampler = _SAMPLER.match(stripped)
        if sampler:
            name = demangle(sampler.group(2))
            renames.add(sampler.group(2), name)
            bindings["samplers"].append(
                {"name": name, "binding": _binding_of(pending), "type": f"FLOAT_{sampler.group(1)}D"}
            )
            pending = []
            continue

        image = _IMAGE.match(stripped)
        if image:
            name = demangle(image.group(2))
            renames.add(image.group(2), name)
            bindings["images"].append(
                {"name": name, "binding": _binding_of(pending), "format": _image_format_of(pending)}
            )
            pending = []
            continue

        stage_io = _STAGE_IO.match(stripped)
        if stage_io and pending:
            mangled = stage_io.group(2)
            if stage_io.group(1) == "in":
                name = demangle(mangled)
                bindings["inputs"].append(name)
            else:
                location = _location_of(pending)
                if location is None or location >= len(output_names):
                    raise SystemExit(f"fragment output {mangled} at location {location} has no configured name")
                name = output_names[location]
                outputs_by_location[location] = name
            renames.add(mangled, name)
            pending = []
            continue

        if pending:
            raise SystemExit(f"unhandled layout qualifiers {pending} before: {stripped}")
        kept.append(line)

    bindings["outputs"] = [outputs_by_location[k] for k in sorted(outputs_by_location)]

    text = _demangle_struct_members(renames.apply("\n".join(kept)))

    output_lines = [ROW_MAJOR_DEFAULT] + [line for line in text.split("\n")]
    return _collapse_blank_runs(output_lines), bindings


def _demangle_struct_members(text: str) -> str:
    """Struct members keep their source names so the addons can specialize `flame.a.b` paths.

    A base name that occurs in several structs is mangled differently per struct; every
    variant maps back to the same plain name, which is unambiguous through the typed
    instance it is accessed on."""
    lines = text.split("\n")
    mangled: set[str] = set()
    inside = False
    for index, line in enumerate(lines):
        if re.match(r"^struct\s+\w+", line):
            inside = True
            continue
        if inside and line.strip() == "};":
            inside = False
            continue
        if inside:
            m = _MEMBER.match(line)
            if m and demangle(m.group(2)) != m.group(2):
                mangled.add(m.group(2))
                lines[index] = line.replace(m.group(2), demangle(m.group(2)), 1)
    if not mangled:
        return "\n".join(lines)
    access = re.compile(r"\.(" + "|".join(re.escape(name) for name in sorted(mangled, key=len, reverse=True)) + r")\b")
    return "\n".join(access.sub(lambda m: "." + demangle(m.group(1)), line) for line in lines)


def _collapse_blank_runs(lines: list[str]) -> list[str]:
    output: list[str] = []
    for line in lines:
        if line.strip() == "" and output and output[-1].strip() == "":
            continue
        output.append(line)
    return output


def export_shader(entry: str, stage: str, defines: list[str], output_names: list[str], repo_root: str) -> tuple[list[str], dict]:
    glsl_text = generate_glsl(entry, stage, defines, repo_root)
    return convert_to_blender_dialect(glsl_text, output_names)


def write_shader(out_dir: str, stem: str, lines: list[str], bindings: dict) -> None:
    os.makedirs(out_dir, exist_ok=True)
    glsl_path = os.path.join(out_dir, f"{stem}.glsl")
    with open(glsl_path, "w") as f:
        f.write("\n".join(lines).rstrip("\n") + "\n")

    json_path = os.path.join(out_dir, f"{stem}.bindings.json")
    with open(json_path, "w") as f:
        json.dump(bindings, f, indent=2)
        f.write("\n")

    print(f"Written {len(lines)} lines to {glsl_path}")
    print(f"Bindings written to {json_path}")


def find_balanced_call(text: str, start: int) -> int:
    """Index just past the ')' closing the '(' at text[start]."""
    depth = 0
    for index in range(start, len(text)):
        if text[index] == "(":
            depth += 1
        elif text[index] == ")":
            depth -= 1
            if depth == 0:
                return index + 1
    raise SystemExit("unbalanced parentheses in generated GLSL")


def rewrite_sampler_uv(text: str, sampler: str, rewrite) -> str:
    """Apply rewrite(uv_expression) to every texture(sampler, uv) call."""
    pattern = re.compile(rf"texture\(\s*\(?{re.escape(sampler)}\)?\s*,")
    result = []
    cursor = 0
    while True:
        m = pattern.search(text, cursor)
        if not m:
            result.append(text[cursor:])
            return "".join(result)
        uv_start = m.end()
        while text[uv_start] == " ":
            uv_start += 1
        call_end = find_balanced_call(text, m.start() + len("texture"))
        uv_expression = text[uv_start:call_end - 1].strip()
        result.append(text[cursor:m.start()])
        result.append(f"texture({sampler}, {rewrite(uv_expression)})")
        cursor = call_end
