#!/usr/bin/env python3
"""
Random Sans file generator for fuzz testing.
Usage: gen_random_sans.py [random|structured]
"""

import sys
import random
import string
import struct

KEYWORDS = [
    "if", "else", "while", "for", "in", "struct", "enum", "match",
    "import", "return", "true", "false", "main", "pub", "break", "continue",
]

SHORT_TYPES = ["I", "S", "B", "F", "V"]
LONG_TYPES = ["Int", "String", "Bool", "Float"]
ALL_TYPES = SHORT_TYPES + LONG_TYPES

OPERATORS = [
    "=", ":=", "+=", "-=", "*=", "/=",
    "+", "-", "*", "/", "%",
    "==", "!=", "<", ">", "<=", ">=",
    "&&", "||", "!", "?", ":",
    "->", "=>", "|", "&", "^", "~",
    "..", "...", "@",
]

BUILTINS = ["print", "println", "len", "push", "pop", "append", "exit", "panic"]

DELIMITERS = ["(", ")", "{", "}", "[", "]", ",", ";", "\n"]

UNICODE_SAMPLES = [
    "\u00e9", "\u4e2d\u6587", "\U0001f600", "\u03b1\u03b2\u03b3",
    "\u0000", "\uffff", "\ud800",  # includes null byte and surrogates
    "\u200b",  # zero-width space
    "\u202e",  # right-to-left override
]


def rand_identifier(min_len=1, max_len=20):
    first = random.choice(string.ascii_letters + "_")
    rest_len = random.randint(min_len - 1, max_len - 1)
    rest = "".join(random.choices(string.ascii_letters + string.digits + "_", k=rest_len))
    return first + rest


def rand_long_identifier():
    length = random.randint(200, 1000)
    first = random.choice(string.ascii_letters)
    rest = "".join(random.choices(string.ascii_letters + string.digits + "_", k=length - 1))
    return first + rest


def rand_number():
    kind = random.randint(0, 4)
    if kind == 0:
        return str(random.randint(-2**63, 2**63 - 1))
    elif kind == 1:
        return str(random.uniform(-1e308, 1e308))
    elif kind == 2:
        return "0x" + "".join(random.choices("0123456789abcdefABCDEF", k=random.randint(1, 16)))
    elif kind == 3:
        return str(random.randint(0, 2**64))  # overflow
    else:
        return "0" * random.randint(1, 50)


def rand_string_literal():
    kind = random.randint(0, 5)
    if kind == 0:
        # normal string
        content = "".join(random.choices(string.printable.replace('"', '').replace('\\', ''), k=random.randint(0, 50)))
        return f'"{content}"'
    elif kind == 1:
        # unterminated string
        content = "".join(random.choices(string.ascii_letters, k=random.randint(1, 20)))
        return f'"{content}'
    elif kind == 2:
        # string with escape sequences
        escapes = ["\\n", "\\t", "\\r", "\\\\", '\\"', "\\0", "\\x41", "\\u0041", "\\U00000041"]
        content = "".join(random.choices(escapes + list(string.ascii_letters), k=random.randint(1, 20)))
        return f'"{content}"'
    elif kind == 3:
        # string with null bytes
        return '"hello\x00world"'
    elif kind == 4:
        # string with unicode
        sample = random.choice(UNICODE_SAMPLES)
        return f'"{sample}"'
    else:
        # very long string
        content = "a" * random.randint(1000, 5000)
        return f'"{content}"'


def rand_token():
    choice = random.randint(0, 9)
    if choice == 0:
        return random.choice(KEYWORDS)
    elif choice == 1:
        return random.choice(ALL_TYPES)
    elif choice == 2:
        return random.choice(OPERATORS)
    elif choice == 3:
        return rand_identifier()
    elif choice == 4:
        return rand_number()
    elif choice == 5:
        return rand_string_literal()
    elif choice == 6:
        return random.choice(DELIMITERS)
    elif choice == 7:
        return random.choice(BUILTINS)
    elif choice == 8:
        return rand_long_identifier()
    else:
        # random unicode/garbage
        return random.choice(UNICODE_SAMPLES)


def generate_random(num_tokens=None):
    if num_tokens is None:
        num_tokens = random.randint(0, 200)
    tokens = []
    for _ in range(num_tokens):
        tok = rand_token()
        sep = random.choice(["", " ", "\n", "\t", "  "])
        tokens.append(tok + sep)
    return "".join(tokens)


# --- Structured mode: plausible-but-broken templates ---

def tmpl_unclosed_brace():
    depth = random.randint(1, 10)
    lines = ["main() {"]
    for i in range(depth):
        lines.append("  " * i + "if true {")
    lines.append("    print(42)")
    # intentionally do not close all braces
    close_count = random.randint(0, depth)
    for i in range(close_count):
        lines.append("  " * (depth - i - 1) + "}")
    return "\n".join(lines)


def tmpl_unclosed_string():
    return 'main() {\n  x = "hello world\n  print(x)\n}\n'


def tmpl_deeply_nested():
    depth = random.randint(50, 200)
    expr = "1"
    for _ in range(depth):
        op = random.choice(["+", "-", "*", "/", "&&", "||"])
        expr = f"({expr} {op} 1)"
    return f"main() {{\n  x = {expr}\n  print(x)\n}}\n"


def tmpl_empty():
    return ""


def tmpl_binary_garbage():
    n = random.randint(16, 512)
    return bytes(random.randint(0, 255) for _ in range(n)).decode("latin-1")


def tmpl_long_line():
    length = random.randint(10000, 100000)
    return "x" * length + "\n"


def tmpl_duplicate_main():
    return "main() {\n  print(1)\n}\n\nmain() {\n  print(2)\n}\n"


def tmpl_invalid_utf8():
    # Mix valid UTF-8 with invalid sequences
    parts = [
        "main() {",
        "\n  x = ",
        b"\xff\xfe".decode("latin-1"),
        '"hello"',
        "\n}\n",
    ]
    return "".join(parts)


def tmpl_null_bytes():
    return "main() {\n  x\x00 = 42\n  print(\x00x)\n}\n"


def tmpl_only_whitespace():
    return " " * random.randint(0, 1000) + "\n" * random.randint(0, 100)


def tmpl_comment_bomb():
    lines = ["main() {"]
    for _ in range(random.randint(100, 500)):
        lines.append("  // " + "a" * random.randint(0, 200))
    lines.append("  print(1)")
    lines.append("}")
    return "\n".join(lines)


def tmpl_deep_type_nesting():
    depth = random.randint(20, 100)
    typ = "I"
    for _ in range(depth):
        typ = f"[{typ}]"
    return f"main() {{\n  x: {typ} = []\n}}\n"


def tmpl_large_struct():
    n = random.randint(500, 2000)
    fields = "\n".join(f"  field_{i}: I" for i in range(n))
    return f"struct Big {{\n{fields}\n}}\nmain() {{\n}}\n"


def tmpl_unicode_identifiers():
    lines = ["main() {"]
    for _ in range(random.randint(5, 20)):
        ident = random.choice(UNICODE_SAMPLES)
        lines.append(f"  {ident} = 42")
    lines.append("}")
    return "\n".join(lines)


def tmpl_operator_soup():
    ops = OPERATORS * 10
    random.shuffle(ops)
    return "main() {\n  x = " + " ".join(ops[:random.randint(10, 50)]) + "\n}\n"


def tmpl_mixed_indentation():
    lines = ["main() {"]
    for i in range(random.randint(10, 50)):
        indent = random.choice([" ", "\t", " \t", "\t "]) * random.randint(0, 8)
        lines.append(f"{indent}x_{i} = {i}")
    lines.append("}")
    return "\n".join(lines)


STRUCTURED_TEMPLATES = [
    tmpl_unclosed_brace,
    tmpl_unclosed_string,
    tmpl_deeply_nested,
    tmpl_empty,
    tmpl_binary_garbage,
    tmpl_long_line,
    tmpl_duplicate_main,
    tmpl_invalid_utf8,
    tmpl_null_bytes,
    tmpl_only_whitespace,
    tmpl_comment_bomb,
    tmpl_deep_type_nesting,
    tmpl_large_struct,
    tmpl_unicode_identifiers,
    tmpl_operator_soup,
    tmpl_mixed_indentation,
]


def generate_structured():
    template_fn = random.choice(STRUCTURED_TEMPLATES)
    return template_fn()


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "random"
    if mode == "structured":
        content = generate_structured()
    else:
        content = generate_random()
    # Write raw bytes to stdout to handle non-UTF-8 content
    sys.stdout.buffer.write(content.encode("utf-8", errors="replace"))
    sys.stdout.buffer.write(b"\n")


if __name__ == "__main__":
    main()
