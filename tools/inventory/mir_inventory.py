#!/usr/bin/env python3
"""Mine a rustc MIR dump of ccc for the std API surface it uses.

Produce the dump on a COPY of ccc (never in ~/dev/ccc itself):

  RUSTC_BOOTSTRAP=1 cargo rustc --lib --release -- -Zunpretty=mir \
      -Zmir-opt-level=0 -Ztrim-diagnostic-paths=false -Zmir-include-spans=yes > mir.txt

Release profile => cfg(debug_assertions) off, cfg(test) off, no features.
-Zmir-opt-level=0 => no MIR inlining, so every call written in ccc source (and
every call produced by desugaring: for-loops, `?`, indexing, format_args,
derives) is still a call terminator.

Usage: mir_inventory.py mir.txt OUTDIR
Writes OUTDIR/{calls.tsv, fnrefs.tsv, casts.tsv, binops.tsv, types.tsv, dyn.tsv,
inst.tsv, fnptr_calls.tsv, closures.tsv}
"""
import re
import sys
import collections

SPAN_RE = re.compile(r'(?:(?<= at )|(?<=span: ))(src/(?!library/)[^\s:]+\.rs):(\d+):(\d+)')
DECL_RE = re.compile(r'^\s+let (?:mut )?_(\d+): (.*);\s+//')
HDR_RE = re.compile(r'^(fn|const|static|promoted)')


def match_close(s, i):
    """s[i] is an opener; return index of its matching closer (bracket-aware,
    treats '->' as not-a-closer)."""
    depth = 0
    j = i
    n = len(s)
    while j < n:
        c = s[j]
        if c in '<([{':
            depth += 1
        elif c == '>':
            if j > 0 and s[j - 1] == '-':
                pass
            else:
                depth -= 1
        elif c in ')]}':
            depth -= 1
        if depth == 0:
            return j
        j += 1
    return -1


def split_top(s, sep):
    """Split s on sep at bracket depth 0."""
    out = []
    depth = 0
    cur = 0
    i = 0
    n = len(s)
    L = len(sep)
    while i < n:
        c = s[i]
        if c in '<([{':
            depth += 1
        elif c == '>' and not (i > 0 and s[i - 1] == '-'):
            depth -= 1
        elif c in ')]}':
            depth -= 1
        elif depth == 0 and s.startswith(sep, i):
            out.append(s[cur:i])
            i += L
            cur = i
            continue
        i += 1
    out.append(s[cur:])
    return out


def find_call_paren(s):
    """Return index of the '(' that starts the argument list of a call rvalue."""
    depth = 0
    for i, c in enumerate(s):
        if c == '(' and depth == 0:
            return i
        if c in '<[{(':
            depth += 1
        elif c == '>' and not (i > 0 and s[i - 1] == '-'):
            depth -= 1
        elif c in ']})':
            depth -= 1
    return -1


PRIMS = {'u8', 'u16', 'u32', 'u64', 'u128', 'usize', 'i8', 'i16', 'i32', 'i64',
         'i128', 'isize', 'f32', 'f64', 'bool', 'char', 'str'}


def strip_refs(t):
    t = t.strip()
    while True:
        m = re.match(r"^(&'[a-z_0-9]+ mut |&'[a-z_0-9]+ |&mut |&|\*const |\*mut )", t)
        if not m:
            return t
        t = t[m.end():]


def strip_generics(p):
    """std::vec::Vec<u8> -> std::vec::Vec ; also drops ::<..> segments."""
    out = []
    depth = 0
    i = 0
    while i < len(p):
        c = p[i]
        if c == '<' and depth == 0 and not p.startswith('<impl', i):
            j = match_close(p, i)
            if j < 0:
                break
            i = j + 1
            continue
        out.append(c)
        i += 1
    s = ''.join(out)
    s = s.replace('::::', '::')
    return s.rstrip(':')


def norm_ty(t):
    """Normalise a self type to a 'type family' key."""
    t = strip_refs(t)
    if t.startswith('['):
        j = match_close(t, 0)
        inner = t[1:j]
        parts = split_top(inner, ';')
        if len(parts) > 1:
            return '[T; N]'
        return '[T]'
    if t == '()':
        return '()'
    if t.startswith('(dyn '):
        return 'dyn ' + strip_generics(t[5:-1]).split(' + ')[0]
    if t.startswith('('):
        return '(tuple)'
    if t.startswith('{closure'):
        return '{closure}'
    if t.startswith('dyn '):
        return 'dyn ' + strip_generics(t[4:]).split(' + ')[0]
    if t.startswith('fn(') or t.startswith('for<') or t.startswith('unsafe fn') or t.startswith('extern'):
        return 'fn-ptr'
    if t.startswith('impl '):
        return 'impl Trait'
    if t.startswith('<'):
        # projection like <T as Trait>::Assoc
        return 'projection'
    base = strip_generics(t)
    if base in PRIMS:
        return base
    if re.fullmatch(r'[A-Z][A-Za-z0-9]*', base) and '::' not in base:
        return 'T(param)'
    return base


def is_std(p):
    return p.startswith(('std::', 'core::', 'alloc::'))


def classify_callee(callee):
    """Return (kind, owner, trait, name).
    kind in {'trait', 'inherent', 'free'}"""
    c = callee.strip()
    if c.startswith('<'):
        j = match_close(c, 0)
        inner = c[1:j]
        rest = c[j + 1:]
        rest = strip_generics(rest).lstrip(':')
        name = rest.split('::')[-1] if rest else ''
        parts = split_top(inner, ' as ')
        if len(parts) == 2:
            selfty, trait = parts
            return ('trait', norm_ty(selfty), strip_generics(trait.strip()), name, selfty.strip())
        return ('inherent', norm_ty(inner), '', name, inner)
    segs = split_top(c, '::')
    segs = [s for s in segs if s and not (s.startswith('<') and not s.startswith('<impl'))]
    # strip generic suffixes on segments e.g. Vec<u8>
    clean = [strip_generics(s) if not s.startswith('<impl') else s for s in segs]
    name = clean[-1]
    for k, s in enumerate(clean[:-1]):
        if s.startswith('<impl '):
            ty = s[len('<impl '):-1]
            return ('inherent', norm_ty(ty), '', name, ty)
    if len(clean) >= 2 and re.match(r'^[A-Z]', clean[-2]):
        owner = '::'.join(clean[:-1])
        return ('inherent', owner, '', name, owner)
    return ('free', '::'.join(clean[:-1]), '', name, '')


def operand_type(op, decls):
    op = op.strip()
    op = re.sub(r'^(move|copy) ', '', op)
    m = re.match(r'^_(\d+)$', op)
    if m:
        return decls.get(int(m.group(1)), '?')
    m = re.match(r'^\(\*_(\d+)\)$', op)
    if m:
        return strip_refs(decls.get(int(m.group(1)), '?'))
    m = re.match(r'^\(.*: (.+)\)$', op)
    if m:
        return m.group(1)
    m = re.match(r'^const .*_(u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize|f32|f64)$', op)
    if m:
        return m.group(1)
    if op.startswith('const true') or op.startswith('const false'):
        return 'bool'
    if op.startswith("const '"):
        return 'char'
    return '?'


BINOPS = ('Add', 'Sub', 'Mul', 'Div', 'Rem', 'BitAnd', 'BitOr', 'BitXor', 'Shl', 'Shr',
          'Eq', 'Ne', 'Lt', 'Le', 'Gt', 'Ge', 'Cmp', 'AddWithOverflow',
          'SubWithOverflow', 'MulWithOverflow', 'Offset')
BIN_RE = re.compile(r'^\s+\S.* = (' + '|'.join(BINOPS) + r')\((.*)\);\s+//')
UN_RE = re.compile(r'^\s+\S.* = (Neg|Not)\((.*)\);\s+//')
CAST_RE = re.compile(r'^\s+\S.* = (.*) as (.+) \((\w+)(?:\([^)]*\))?\);\s+//')
FNREF_RE = re.compile(r'\+ const_: Const \{ ty: (.*), val: ')


def main():
    path, outdir = sys.argv[1], sys.argv[2]
    calls = collections.Counter()          # (kind, owner, trait, name) -> n
    call_ex = {}
    call_full = collections.defaultdict(collections.Counter)  # key -> full self types
    fnrefs = collections.Counter()
    fnref_ex = {}
    casts = collections.Counter()
    cast_ex = {}
    binops = collections.Counter()
    types = collections.Counter()
    type_ex = {}
    dyns = collections.Counter()
    dyn_ex = {}
    fnptr_calls = collections.Counter()
    fnptr_ex = {}
    closures = collections.Counter()
    decls = {}
    sites = []
    cur_fn = ''
    last_span = ''
    with open(path, errors='replace') as f:
        for line in f:
            if HDR_RE.match(line):
                cur_fn = line.strip()
                decls = {}
                # header args
                if line.startswith('fn '):
                    i = line.index('(')
                    j = match_close(line, i)
                    for a in split_top(line[i + 1:j], ', '):
                        m = re.match(r'_(\d+): (.*)', a.strip())
                        if m:
                            decls[int(m.group(1))] = m.group(2)
                    m = re.match(r'fn .*::\{closure#\d+\}\(_1: (&mut |&)?\{closure', line)
                    if m:
                        kind = {'&mut ': 'FnMut-style(&mut self)', '&': 'Fn-style(&self)', None: 'FnOnce-style(self)'}[m.group(1)]
                        closures[kind] += 1
                continue
            sm = SPAN_RE.search(line)
            if sm:
                span = sm.group(1) + ':' + sm.group(2)
            else:
                span = last_span
            m = DECL_RE.match(line)
            if m:
                t = m.group(2)
                decls[int(m.group(1))] = t
                for sp in set(re.findall(r'\b((?:std|core|alloc)::[A-Za-z0-9_:]+)', t)):
                    types[sp] += 1
                    type_ex.setdefault(sp, span)
                for d in set(re.findall(r'dyn ([A-Za-z0-9_:]+(?:<[^>]*>)?)', t)):
                    dyns[d] += 1
                    dyn_ex.setdefault(d, span)
                continue
            if '+ const_: Const' in line:
                m = FNREF_RE.search(line)
                if m:
                    ty = m.group(1)
                    # fn item type: '... {path}' at end
                    if ty.endswith('}') and ' {' in ty:
                        k = ty.rfind(' {')
                        # find the brace that matches the final }
                        depth = 0
                        start = -1
                        for idx in range(len(ty) - 1, -1, -1):
                            ch = ty[idx]
                            if ch == '}':
                                depth += 1
                            elif ch == '{':
                                depth -= 1
                                if depth == 0:
                                    start = idx
                                    break
                        p = ty[start + 1:-1]
                        if p.startswith('closure@'):
                            continue
                        kind, owner, trait, name, full = classify_callee(p)
                        key = (kind, owner, trait, name)
                        fnrefs[key] += 1
                        fnref_ex.setdefault(key, last_span)
                continue
            if not line.startswith('        ') or line[8] == ' ':
                continue
            last_span = span
            if ' -> [return' in line or ' -> unwind' in line or '-> [return' in line:
                m = re.match(r'^\s+(\S[^=]*?) = (.*)$', line)
                if m:
                    rhs = m.group(2)
                    i = find_call_paren(rhs)
                    if i > 0:
                        callee = rhs[:i]
                        if callee.startswith('move _') or callee.startswith('copy _') or re.match(r'^\(?\*?_\d', callee):
                            mm = re.search(r'_(\d+)', callee)
                            t = decls.get(int(mm.group(1)), '?') if mm else '?'
                            fnptr_calls[t] += 1
                            fnptr_ex.setdefault(t, span)
                        else:
                            kind, owner, trait, name, full = classify_callee(callee)
                            key = (kind, owner, trait, name)
                            calls[key] += 1
                            call_ex.setdefault(key, span)
                            j = match_close(rhs, i)
                            a0 = split_top(rhs[i + 1:j], ', ')[0] if j > i + 1 else ''
                            sites.append((kind, owner, trait, name, span, full[:200], operand_type(a0, decls)[:300] if a0 else ''))
                            if full:
                                call_full[key][full] += 1
                continue
            m = BIN_RE.match(line)
            if m:
                op = m.group(1)
                args = split_top(m.group(2), ', ')
                t = operand_type(args[0], decls)
                t2 = operand_type(args[1], decls) if len(args) > 1 else ''
                if t == '?':
                    t = t2
                binops[(op, strip_refs(t))] += 1
                continue
            m = UN_RE.match(line)
            if m:
                binops[(m.group(1), strip_refs(operand_type(m.group(2), decls)))] += 1
                continue
            m = CAST_RE.match(line)
            if m:
                src = operand_type(m.group(1), decls)
                key = (m.group(3), src, m.group(2))
                casts[key] += 1
                cast_ex.setdefault(key, span)
                continue

    def w(name, rows, header):
        with open(f'{outdir}/{name}', 'w') as o:
            o.write('\t'.join(header) + '\n')
            for r in rows:
                o.write('\t'.join(str(x) for x in r) + '\n')

    w('calls.tsv', [(n, *k, call_ex[k], ' | '.join(f'{t}' for t, _ in call_full[k].most_common(4)))
                    for k, n in calls.most_common()],
      ['count', 'kind', 'owner', 'trait', 'name', 'example', 'self_types'])
    w('fnrefs.tsv', [(n, *k, fnref_ex[k]) for k, n in fnrefs.most_common()],
      ['count', 'kind', 'owner', 'trait', 'name', 'example'])
    w('casts.tsv', [(n, *k, cast_ex[k]) for k, n in casts.most_common()],
      ['count', 'kind', 'from', 'to', 'example'])
    w('binops.tsv', [(n, *k) for k, n in binops.most_common()], ['count', 'op', 'type'])
    w('types.tsv', [(n, k, type_ex[k]) for k, n in types.most_common()], ['count', 'path', 'example'])
    w('dyn.tsv', [(n, k, dyn_ex[k]) for k, n in dyns.most_common()], ['count', 'dyn', 'example'])
    w('fnptr_calls.tsv', [(n, k, fnptr_ex[k]) for k, n in fnptr_calls.most_common()], ['count', 'callee_type', 'example'])
    w('sites.tsv', sites, ['kind', 'owner', 'trait', 'name', 'span', 'self', 'arg0_type'])
    w('closures.tsv', [(n, k) for k, n in closures.most_common()], ['count', 'kind'])


if __name__ == '__main__':
    main()
