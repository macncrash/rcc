#!/usr/bin/env python3
"""Copy the ccc crate's non-test source to OUTDIR, following the module tree
from src/lib.rs, src/main.rs and src/bin/*.rs, and blanking out every item
annotated #[cfg(test)] or #[test] (replaced by spaces/newlines so that line
numbers are preserved).  Also strips items under #[cfg(feature = ...)] (no
features are enabled in a default release build) and #[cfg(not(unix))]-style
dead predicates.

Usage: strip_tests.py CCC_ROOT OUTDIR
Prints a summary of what was stripped.
"""
import os
import re
import sys

root = outdir = None

# cfg predicates and their value for a release build on arm64-macos AND x86_64-linux
# (all the predicates ccc uses evaluate identically on both hosts except target_os/arch,
# which ccc only uses inside cfg!() expressions, not attributes).
def cfg_value(pred):
    pred = pred.strip()
    if pred == 'test':
        return False
    if pred == 'unix':
        return True
    if pred == 'debug_assertions':
        return False
    m = re.fullmatch(r'feature\s*=\s*"[^"]*"', pred)
    if m:
        return False
    m = re.fullmatch(r'not\((.*)\)', pred, re.S)
    if m:
        return not cfg_value(m.group(1))
    m = re.fullmatch(r'(any|all)\((.*)\)', pred, re.S)
    if m:
        parts = split_commas(m.group(2))
        vals = [cfg_value(p) for p in parts if p.strip()]
        return any(vals) if m.group(1) == 'any' else all(vals)
    raise ValueError('unknown cfg predicate: ' + pred)


def split_commas(s):
    out, depth, cur = [], 0, ''
    for c in s:
        if c == '(':
            depth += 1
        elif c == ')':
            depth -= 1
        if c == ',' and depth == 0:
            out.append(cur)
            cur = ''
        else:
            cur += c
    out.append(cur)
    return out


def skip_ws_comments(s, i):
    n = len(s)
    while i < n:
        if s[i].isspace():
            i += 1
        elif s.startswith('//', i):
            j = s.find('\n', i)
            i = n if j < 0 else j + 1
        elif s.startswith('/*', i):
            depth = 0
            while i < n:
                if s.startswith('/*', i):
                    depth += 1
                    i += 2
                elif s.startswith('*/', i):
                    depth -= 1
                    i += 2
                    if depth == 0:
                        break
                else:
                    i += 1
        else:
            break
    return i


def skip_literal(s, i):
    """If s[i] starts a string/char/raw string literal, return index after it, else None."""
    n = len(s)
    m = re.compile(r'b?r(#*)"').match(s, i)
    if m and (i == 0 or not (s[i - 1].isalnum() or s[i - 1] == '_')):
        hashes = m.group(1)
        end = s.find('"' + hashes, m.end())
        return end + 1 + len(hashes)
    if s[i] == '"' or (s[i] == 'b' and i + 1 < n and s[i + 1] == '"' and (i == 0 or not (s[i - 1].isalnum() or s[i - 1] == '_'))):
        j = i + (2 if s[i] == 'b' else 1)
        while j < n:
            if s[j] == '\\':
                j += 2
            elif s[j] == '"':
                return j + 1
            else:
                j += 1
        return n
    if s[i] == "'" or (s[i] == 'b' and i + 1 < n and s[i + 1] == "'"):
        j = i + (2 if s[i] == 'b' else 1)
        # char literal vs lifetime
        m = re.compile(r"(\\(x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]+\}|.)|[^\\'])'").match(s, j)
        if m:
            return m.end()
        return None
    return None


def item_end(s, i):
    """Return end index of the item starting at i (after attributes)."""
    n = len(s)
    depth = 0
    i = skip_ws_comments(s, i)
    # also skip further attributes
    while s.startswith('#[', i):
        k = s.find(']', i)
        i = skip_ws_comments(s, k + 1)
    is_item = re.compile(r'(pub\b|fn\b|mod\b|impl\b|struct\b|enum\b|use\b|const\b|static\b|type\b|let\b|trait\b|macro_rules!|extern\b|unsafe\b)').match(s, i) is not None
    while i < n:
        i2 = skip_ws_comments(s, i)
        if i2 != i:
            i = i2
            continue
        lit = skip_literal(s, i)
        if lit is not None:
            i = lit
            continue
        c = s[i]
        if c in '([{':
            depth += 1
        elif c in ')]}':
            depth -= 1
            if depth == 0 and c == '}':
                # end of a braced item (fn/mod/impl/struct body) unless followed by more (e.g. `else`)
                return i + 1
            if depth < 0:
                return i
        elif c == ';' and depth == 0:
            return i + 1
        elif c == ',' and depth == 0 and not is_item:
            # struct field / enum variant / match arm terminated by comma
            return i + 1
        i += 1
    return n


ATTR_RE = re.compile(r'#\[(cfg|test)\b')


def strip(src, stats, fname):
    out = list(src)
    i = 0
    n = len(src)
    while i < n:
        j = skip_ws_comments(src, i)
        if j != i:
            i = j
            continue
        lit = skip_literal(src, i)
        if lit is not None:
            i = lit
            continue
        if src.startswith('#[', i) and not src.startswith('#![', i):
            # parse the attribute
            k = i + 1
            depth = 0
            while k < n:
                if src[k] == '[':
                    depth += 1
                elif src[k] == ']':
                    depth -= 1
                    if depth == 0:
                        break
                k += 1
            attr = src[i + 2:k]
            dead = False
            m = re.fullmatch(r'\s*cfg\s*\((.*)\)\s*', attr, re.S)
            if m:
                dead = not cfg_value(m.group(1))
                stats.setdefault('cfg(' + m.group(1).strip() + ')', 0)
                stats['cfg(' + m.group(1).strip() + ')'] += 1
            elif attr.strip() == 'test':
                dead = True
                stats['#[test]'] = stats.get('#[test]', 0) + 1
            if dead:
                e = item_end(src, k + 1)
                for p in range(i, e):
                    if out[p] != '\n':
                        out[p] = ' '
                stats.setdefault('_lines_removed', 0)
                stats['_lines_removed'] += src.count('\n', i, e)
                stats.setdefault('_where', []).append(f'{fname}:{src.count(chr(10), 0, i) + 1} {attr.strip()} ({src.count(chr(10), i, e)} lines)')
                i = e
                continue
            i = k + 1
            continue
        i += 1
    return ''.join(out)


MOD_RE = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([a-z_0-9]+)\s*;', re.M)


def process(path, modrel_dir, stats, seen):
    if path in seen:
        return
    seen.add(path)
    with open(path) as f:
        src = f.read()
    stripped = strip(src, stats, os.path.relpath(path, root))
    rel = os.path.relpath(path, root)
    dst = os.path.join(outdir, rel)
    os.makedirs(os.path.dirname(dst), exist_ok=True)
    with open(dst, 'w') as f:
        f.write(stripped)
    for m in MOD_RE.finditer(stripped):
        name = m.group(1)
        cands = [os.path.join(modrel_dir, name + '.rs'), os.path.join(modrel_dir, name, 'mod.rs')]
        for c in cands:
            if os.path.exists(c):
                sub = os.path.dirname(c) if c.endswith('mod.rs') else os.path.join(modrel_dir, name)
                process(c, sub, stats, seen)
                break
        else:
            print('missing module', name, 'from', path, file=sys.stderr)


if __name__ == '__main__':
    root, outdir = sys.argv[1], sys.argv[2]
    stats = {}
    seen = set()
    src = os.path.join(root, 'src')
    process(os.path.join(src, 'lib.rs'), src, stats, seen)
    process(os.path.join(src, 'main.rs'), src, stats, seen)
    for b in sorted(os.listdir(os.path.join(src, 'bin'))):
        process(os.path.join(src, 'bin', b), os.path.join(src, 'bin', b[:-3]), stats, seen)
    allrs = set()
    for d, _, fs in os.walk(src):
        for f in fs:
            if f.endswith('.rs'):
                allrs.add(os.path.join(d, f))
    print('files in tree:', len(seen), 'unreached .rs files:', sorted(os.path.relpath(p, root) for p in allrs - seen))
    for k, v in sorted(stats.items()):
        if k != '_where':
            print(k, v)
    with open(os.path.join(outdir, 'STRIPPED.txt'), 'w') as f:
        f.write('\n'.join(stats.get('_where', [])) + '\n')
