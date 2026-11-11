#!/usr/bin/env python3
"""Collect every format-string placeholder used in formatting macros in the
test-stripped ccc tree.

Usage: fmt_specs.py STRIPPED_ROOT
"""
import os
import re
import sys
import collections

sys.path.insert(0, os.path.dirname(__file__))
from strip_tests import skip_literal  # noqa: E402

root = sys.argv[1]
MACROS = ('format', 'format_args', 'write', 'writeln', 'print', 'println', 'eprint', 'eprintln',
          'panic', 'assert', 'assert_eq', 'assert_ne', 'debug_assert', 'debug_assert_eq',
          'unreachable', 'emit', 'todo', 'unimplemented')
MAC_RE = re.compile(r'\b(' + '|'.join(MACROS) + r')!\s*[\(\[\{]')
# position of the format string argument: 0 for format/print..., 1 for write/emit, after cond for assert
SKIP_ARGS = {'write': 1, 'writeln': 1, 'emit': 1, 'assert': 1, 'debug_assert': 1,
             'assert_eq': 2, 'assert_ne': 2, 'debug_assert_eq': 2}

specs = collections.Counter()
spec_ex = {}
argkinds = collections.Counter()
argkind_ex = {}
per_macro = collections.Counter()
no_fmt = collections.Counter()


def parse_placeholders(s):
    i = 0
    out = []
    while i < len(s):
        if s.startswith('{{', i) or s.startswith('}}', i):
            i += 2
            continue
        if s[i] == '{':
            j = s.find('}', i)
            out.append(s[i + 1:j])
            i = j + 1
            continue
        i += 1
    return out


def unescape_str(lit):
    if lit.startswith('r'):
        body = lit[lit.index('"') + 1:lit.rindex('"')]
        return body
    body = lit[1:-1]
    # handle line continuation
    body = re.sub(r'\\\n\s*', '', body)
    return body


for d, _, fs in os.walk(os.path.join(root, 'src')):
    for f in fs:
        if not f.endswith('.rs'):
            continue
        p = os.path.join(d, f)
        rel = os.path.relpath(p, root)
        src = open(p).read()
        for m in MAC_RE.finditer(src):
            line = src.count('\n', 0, m.start()) + 1
            # skip if in a comment line
            ls = src.rfind('\n', 0, m.start()) + 1
            if src[ls:m.start()].lstrip().startswith('//'):
                continue
            name = m.group(1)
            per_macro[name] += 1
            # find args: scan to the format string literal, skipping k top-level commas
            i = m.end()
            need = SKIP_ARGS.get(name, 0)
            depth = 0
            commas = 0
            lit = None
            while i < len(src):
                if src[i] in '"r' and commas == need and depth == 0:
                    e = skip_literal(src, i)
                    if e is not None and src[i] != "'":
                        lit = src[i:e]
                        break
                c = src[i]
                if c in "'\"":
                    e = skip_literal(src, i)
                    if e:
                        i = e
                        continue
                if c in '([{':
                    depth += 1
                elif c in ')]}':
                    if depth == 0:
                        break
                    depth -= 1
                elif c == ',' and depth == 0:
                    commas += 1
                    if commas > need:
                        break
                i += 1
            if lit is None:
                no_fmt[name] += 1
                continue
            s = unescape_str(lit)
            for ph in parse_placeholders(s):
                if ':' in ph:
                    arg, spec = ph.split(':', 1)
                    spec = ':' + spec
                else:
                    arg, spec = ph, ''
                if arg == '':
                    ak = 'implicit-next'
                elif arg.isdigit():
                    ak = 'positional-index'
                elif arg.endswith('$'):
                    ak = 'weird'
                else:
                    ak = 'named/inline-captured'
                argkinds[ak] += 1
                argkind_ex.setdefault(ak, f'{rel}:{line} {{{ph}}}')
                # generalise spec: keep it verbatim (small set)
                specs[spec] += 1
                spec_ex.setdefault(spec, f'{rel}:{line}')

print('## macro invocation counts (non-comment)')
for k, v in per_macro.most_common():
    print(f'{v:6d} {k}!   (without literal fmt string: {no_fmt[k]})')
print('\n## placeholder argument kinds')
for k, v in argkinds.most_common():
    print(f'{v:6d} {k}   e.g. {argkind_ex[k]}')
print('\n## distinct format specs (after the colon)')
for k, v in specs.most_common():
    print(f'{v:6d} {{{k}}}   e.g. {spec_ex[k]}')
