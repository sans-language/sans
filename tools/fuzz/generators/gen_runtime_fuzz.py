#!/usr/bin/env python3
"""
Runtime fuzz test generator for Sans.
Generates valid Sans programs that exercise JSON parsing, string operations,
and HTTP request handling with adversarial inputs.

Usage: gen_runtime_fuzz.py [json|string|http|random]
"""

import sys
import random
import string
import struct


def escape_sans_string(s):
    """Escape a string for use inside Sans double quotes."""
    out = []
    for c in s:
        if c == '"':
            out.append('\\"')
        elif c == '\\':
            out.append('\\\\')
        elif c == '\n':
            out.append('\\n')
        elif c == '\r':
            out.append('\\r')
        elif c == '\t':
            out.append('\\t')
        elif c == '\0':
            out.append('\\0')
        elif ord(c) < 32 or ord(c) > 126:
            out.append(f'\\x{ord(c):02x}')
        else:
            out.append(c)
    return ''.join(out)


# --- JSON fuzz generators ---

def gen_json_deep_nesting():
    """Deeply nested JSON that should hit the depth limit."""
    depth = random.randint(100, 1000)
    s = '[' * depth + '1' + ']' * depth
    return f'''main() I {{
  r = json_parse("{escape_sans_string(s)}")
  if r.is_err() {{ 0 }} else {{ 1 }}
}}
'''


def gen_json_malformed():
    """Various malformed JSON strings."""
    samples = [
        '',           # empty
        '{',          # unclosed object
        '[',          # unclosed array
        '{"a":}',     # missing value
        '{:1}',       # missing key
        '{"a"::1}',   # double colon
        '[1,]',       # trailing comma
        '[,1]',       # leading comma
        '{"a":1,}',   # trailing comma in object
        '{1:2}',      # numeric key
        'truee',      # misspelled literal
        'nul',        # truncated null
        'fals',       # truncated false
        '01',         # leading zero
        '1.',         # trailing dot
        '.1',         # leading dot
        '1e',         # truncated exponent
        '1e+',        # truncated exponent sign
        '--1',        # double negative
        '++1',        # double positive
        '{"a":1"b":2}',  # missing comma
        '"\\"',       # backslash at end
        '"\\u"',      # truncated unicode escape
        '"\\u00"',    # truncated unicode escape
        '"\\uXXXX"',  # invalid unicode escape
        '{"":""}',    # empty key and value
        '"' + 'a' * 10000 + '"',  # very long string
        '0.' + '0' * 1000,  # very long decimal
        '1e999999',   # huge exponent
        '-1e999999',  # huge negative exponent
        '1e-999999',  # tiny exponent
    ]
    s = random.choice(samples)
    return f'''main() I {{
  r = json_parse("{escape_sans_string(s)}")
  if r.is_err() {{ 0 }} else {{ 1 }}
}}
'''


def gen_json_random_bytes():
    """Random byte sequences as JSON input."""
    n = random.randint(1, 200)
    chars = []
    for _ in range(n):
        c = random.randint(1, 127)  # ASCII range, avoid null
        if c == ord('"'):
            chars.append('\\"')
        elif c == ord('\\'):
            chars.append('\\\\')
        elif c < 32:
            chars.append(f'\\x{c:02x}')
        else:
            chars.append(chr(c))
    s = ''.join(chars)
    return f'''main() I {{
  r = json_parse("{s}")
  if r.is_err() {{ 0 }} else {{ 1 }}
}}
'''


def gen_json_large_object():
    """JSON object with many keys."""
    n = random.randint(50, 200)
    pairs = ','.join(f'"k{i}":{i}' for i in range(n))
    s = '{' + pairs + '}'
    return f'''main() I {{
  r = json_parse("{escape_sans_string(s)}")
  if r.is_err() {{ 0 }} else {{
    j = r!
    j["k0"].int()
  }}
}}
'''


def gen_json_large_array():
    """JSON array with many elements."""
    n = random.randint(100, 500)
    elems = ','.join(str(i) for i in range(n))
    s = '[' + elems + ']'
    return f'''main() I {{
  r = json_parse("{escape_sans_string(s)}")
  if r.is_err() {{ 0 }} else {{ 1 }}
}}
'''


def gen_json_float_edge():
    """Edge case float values in JSON."""
    floats = [
        '0.0', '-0.0', '1e308', '-1e308', '5e-324',
        '2.2250738585072014e-308',  # smallest normal
        '1.7976931348623157e308',    # largest double
        '2.2250738585072011e-308',   # denormal boundary
        '0.1', '0.2', '0.3',        # common float imprecision
    ]
    f = random.choice(floats)
    return f'''main() I {{
  r = json_parse("{f}")
  if r.is_err() {{ 0 }} else {{ 1 }}
}}
'''


def gen_json_stringify_roundtrip():
    """Parse then stringify then parse again."""
    samples = [
        '{"a":1,"b":"hello","c":true,"d":null,"e":[1,2,3]}',
        '[1,"two",3.14,true,null,{"k":"v"}]',
        '{"nested":{"deep":{"value":42}}}',
    ]
    s = random.choice(samples)
    return f'''main() I {{
  j = json_parse("{escape_sans_string(s)}")!
  s = json_stringify(j)
  j2 = json_parse(s)!
  s2 = json_stringify(j2)
  if s == s2 {{ 0 }} else {{ 1 }}
}}
'''


# --- String operation fuzz generators ---

def gen_string_ops():
    """Exercise string operations with adversarial input."""
    ops = [
        'x = ""\n  l = x.len()',
        'x = "' + 'a' * random.randint(1000, 5000) + '"\n  l = x.len()',
        'x = "hello"\n  y = x + x + x + x + x + x + x + x + x + x\n  l = y.len()',
        'x = "hello world"\n  y = x.split(" ")\n  l = y.len()',
        'x = "abc"\n  y = x.contains("b") ? 1 : 0',
        'x = "HELLO"\n  y = x.lower()\n  l = y.len()',
        'x = "hello"\n  y = x.upper()\n  l = y.len()',
        'x = "  hello  "\n  y = x.trim()\n  l = y.len()',
        'x = "hello"\n  y = x.replace("l", "r")\n  l = y.len()',
        'x = "a,b,c,d,e"\n  y = x.split(",")\n  l = y.len()',
    ]
    op = random.choice(ops)
    return f'''main() I {{
  {op}
  l
}}
'''


def gen_string_concat_stress():
    """Stress test string concatenation."""
    n = random.randint(100, 500)
    return f'''main() I {{
  s := ""
  i := 0
  while i < {n} {{
    s = s + "x"
    i += 1
    0
  }}
  s.len()
}}
'''


def gen_string_split_edge():
    """Edge cases for string split."""
    cases = [
        ('""', '","'),           # empty string
        ('","', '","'),          # only delimiter
        ('",,,"', '","'),       # multiple delimiters
        ('"hello"', '""'),      # empty delimiter
        ('"hello"', '"x"'),     # delimiter not found
    ]
    s, delim = random.choice(cases)
    return f'''main() I {{
  x = {s}
  y = x.split({delim})
  y.len()
}}
'''


# --- HTTP request fuzz generators (if server primitives are available) ---

def gen_map_stress():
    """Stress test map with many insertions and deletions."""
    n = random.randint(100, 500)
    return f'''main() I {{
  m = M()
  i := 0
  while i < {n} {{
    k = "key_" + str(i)
    m.set(k i)
    i += 1
    0
  }}
  // Delete half
  j := 0
  while j < {n} / 2 {{
    m.delete("key_" + str(j))
    j += 1
    0
  }}
  m.len()
}}
'''


def gen_map_int_stress():
    """Stress test integer key map."""
    n = random.randint(100, 500)
    return f'''main() I {{
  m = M<I I>()
  i := 0
  while i < {n} {{
    m.set(i i * i)
    i += 1
    0
  }}
  ok := 0
  j := 0
  while j < {n} {{
    v = m.get(j)!
    if v == j * j {{ ok += 1 }}
    j += 1
    0
  }}
  ok == {n} ? 0 : 1
}}
'''


JSON_GENERATORS = [
    gen_json_deep_nesting,
    gen_json_malformed,
    gen_json_random_bytes,
    gen_json_large_object,
    gen_json_large_array,
    gen_json_float_edge,
    gen_json_stringify_roundtrip,
]

STRING_GENERATORS = [
    gen_string_ops,
    gen_string_concat_stress,
    gen_string_split_edge,
]

MAP_GENERATORS = [
    gen_map_stress,
    gen_map_int_stress,
]

ALL_GENERATORS = JSON_GENERATORS + STRING_GENERATORS + MAP_GENERATORS


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "random"

    if mode == "json":
        gen = random.choice(JSON_GENERATORS)
    elif mode == "string":
        gen = random.choice(STRING_GENERATORS)
    elif mode == "map":
        gen = random.choice(MAP_GENERATORS)
    else:
        gen = random.choice(ALL_GENERATORS)

    content = gen()
    sys.stdout.write(content)


if __name__ == "__main__":
    main()
