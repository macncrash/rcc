#!/usr/bin/env python3
"""Turn mir_inventory.py output into markdown tables (std API by type).

Usage: report.py OUTDIR > api.md
"""
import sys
import collections
import re

out = sys.argv[1]


def read(name):
    rows = []
    with open(f'{out}/{name}') as f:
        hdr = f.readline().rstrip('\n').split('\t')
        for line in f:
            rows.append(dict(zip(hdr, line.rstrip('\n').split('\t'))))
    return rows


PRIM = {'u8', 'u16', 'u32', 'u64', 'u128', 'usize', 'i8', 'i16', 'i32', 'i64', 'i128',
        'isize', 'f32', 'f64', 'bool', 'char', 'str', '[T]', '[T; N]', '(tuple)', '()',
        'fn-ptr', '{closure}', 'T(param)', 'projection'}


def is_std(p):
    return p.startswith(('std::', 'core::', 'alloc::'))


calls = read('calls.tsv')
fnrefs = read('fnrefs.tsv')
callcount = {(r['kind'], r['owner'], r['trait'], r['name']): int(r['count']) for r in calls}

# group: owner -> {method: (refs, calls, example)}
by_owner = collections.defaultdict(dict)
local_trait_on_std = collections.defaultdict(dict)   # std trait -> local self types
std_trait_generic = collections.defaultdict(dict)    # std trait called on T / closures
value_uses = []
for r in fnrefs:
    kind, owner, trait, name = r['kind'], r['owner'], r['trait'], r['name']
    key = (kind, owner, trait, name)
    n = int(r['count'])
    c = callcount.get(key, 0)
    ex = r['example']
    if n > c:
        value_uses.append((key, n - c, ex))
    owner_std = is_std(owner) or owner in PRIM or owner.startswith('dyn std')
    if kind == 'trait':
        if is_std(trait):
            tshort = trait.split('::')[-1]
            if owner in ('T(param)', 'projection', '{closure}', 'fn-ptr', 'impl Trait') or owner.startswith('dyn '):
                std_trait_generic[trait].setdefault(name, [0, 0, ex, set()])
                e = std_trait_generic[trait][name]
                e[0] += n; e[1] += c; e[3].add(owner)
            elif owner_std:
                m = f'{tshort}::{name}'
                by_owner[owner].setdefault(m, [0, 0, ex])
                by_owner[owner][m][0] += n
                by_owner[owner][m][1] += c
            else:
                local_trait_on_std[trait].setdefault(name, [0, 0, ex, set()])
                e = local_trait_on_std[trait][name]
                e[0] += n; e[1] += c; e[3].add(owner)
        continue
    if kind == 'inherent' and owner_std:
        by_owner[owner].setdefault(name, [0, 0, ex])
        by_owner[owner][name][0] += n
        by_owner[owner][name][1] += c
    elif kind == 'free' and is_std(owner):
        by_owner[owner + ' (free fns)'].setdefault(name, [0, 0, ex])
        by_owner[owner + ' (free fns)'][name][0] += n
        by_owner[owner + ' (free fns)'][name][1] += c


def fmt_methods(d, show_trait=True):
    inh = sorted((k for k in d if '::' not in k), key=lambda k: -d[k][0])
    tr = sorted((k for k in d if '::' in k), key=lambda k: -d[k][0])
    s1 = ', '.join(f'{k} ({d[k][0]})' for k in inh)
    s2 = ', '.join(f'{k} ({d[k][0]})' for k in tr)
    return s1, s2


print('| type / module | inherent / free (refs) | via std traits (refs) |')
print('|---|---|---|')
for owner in sorted(by_owner, key=lambda o: -sum(v[0] for v in by_owner[o].values())):
    s1, s2 = fmt_methods(by_owner[owner])
    print(f'| `{owner}` | {s1} | {s2} |')

print('\n\n### std traits invoked on generic params / closures / dyn\n')
print('| trait | method (refs) [self kinds] |')
print('|---|---|')
for t in sorted(std_trait_generic):
    d = std_trait_generic[t]
    print(f'| `{t}` | ' + ', '.join(f'{k} ({d[k][0]}) [{"/".join(sorted(d[k][3]))}]' for k in sorted(d, key=lambda k: -d[k][0])) + ' |')

print('\n\n### std trait methods invoked on ccc-defined types (impls ccc must provide, derived or by hand)\n')
print('| trait | method (refs) | #distinct self types | sample self types |')
print('|---|---|---|---|')
for t in sorted(local_trait_on_std):
    d = local_trait_on_std[t]
    for k in sorted(d, key=lambda k: -d[k][0]):
        samp = ', '.join(sorted(d[k][3])[:6])
        print(f'| `{t}` | {k} ({d[k][0]}) | {len(d[k][3])} | {samp} |')

print('\n\n### fn items used as values (not called directly; e.g. `.map(String::from)`)\n')
print('| path | uses | example |')
print('|---|---|---|')
for key, n, ex in sorted(value_uses, key=lambda x: -x[1]):
    kind, owner, trait, name = key
    if not (is_std(owner) or is_std(trait) or owner in PRIM):
        continue
    p = f'<{owner} as {trait}>::{name}' if trait else f'{owner}::{name}'
    print(f'| `{p}` | {n} | {ex} |')
