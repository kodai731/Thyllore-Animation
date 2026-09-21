"""Rename OpName strings ending in _std140/_std430 so Slang block names match the GLSL stages."""
import struct, sys

OP_NAME = 5
SUFFIXES = ("_std140", "_std430")

def encode_string(s: str) -> list[int]:
    data = s.encode() + b"\0"
    data += b"\0" * (-len(data) % 4)
    return list(struct.unpack(f"<{len(data)//4}I", data))

def rewrite(path_in: str, path_out: str) -> int:
    raw = open(path_in, "rb").read()
    words = list(struct.unpack(f"<{len(raw)//4}I", raw))
    out = words[:5]
    i = 5
    renamed = 0
    while i < len(words):
        count = words[i] >> 16
        opcode = words[i] & 0xFFFF
        inst = words[i:i + count]
        if opcode == OP_NAME:
            text = struct.pack(f"<{count-2}I", *inst[2:]).split(b"\0", 1)[0].decode()
            for suffix in SUFFIXES:
                if text.endswith(suffix):
                    new = encode_string(text[: -len(suffix)])
                    inst = [(len(new) + 2) << 16 | OP_NAME, inst[1], *new]
                    renamed += 1
                    break
        out.extend(inst)
        i += count
    open(path_out, "wb").write(struct.pack(f"<{len(out)}I", *out))
    return renamed

if __name__ == "__main__":
    print(sys.argv[1], "renamed", rewrite(sys.argv[1], sys.argv[2]))
