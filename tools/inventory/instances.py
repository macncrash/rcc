#!/usr/bin/env python3
"""List distinct generic instantiations of selected std types seen in MIR local
declarations (what our mini-std must be monomorphised for).

Usage: instances.py mir.txt > instances.txt
"""
import re
import sys
import collections

sys.path.insert(0, __import__('os').path.dirname(__file__))
from mir_inventory import match_close, DECL_RE, SPAN_RE  # noqa: E402

WANT = ['std::collections::HashMap', 'std::collections::HashSet', 'std::collections::BTreeMap',
        'std::collections::BTreeSet', 'std::collections::VecDeque', 'std::collections::BinaryHeap',
        'std::rc::Rc', 'std::cell::RefCell', 'std::cell::Cell', 'std::borrow::Cow',
        'std::sync::LazyLock', 'std::thread::LocalKey', 'std::boxed::Box', 'std::cmp::Reverse',
        'std::mem::Discriminant', 'std::vec::Vec', 'std::option::Option', 'std::result::Result']

inst = collections.defaultdict(collections.Counter)
ex = {}
with open(sys.argv[1], errors='replace') as f:
    for line in f:
        m = DECL_RE.match(line)
        if not m:
            continue
        t = m.group(2)
        sm = SPAN_RE.search(line)
        span = sm.group(1) + ':' + sm.group(2) if sm else ''
        for w in WANT:
            start = 0
            while True:
                i = t.find(w + '<', start)
                if i < 0:
                    break
                j = match_close(t, i + len(w))
                s = t[i:j + 1]
                inst[w][s] += 1
                ex.setdefault(s, span)
                start = i + 1

for w in WANT:
    c = inst[w]
    print(f'## {w}: {len(c)} distinct instantiations')
    lim = 400 if w not in ('std::vec::Vec', 'std::option::Option', 'std::result::Result', 'std::boxed::Box') else 60
    for s, n in c.most_common(lim):
        print(f'{n}\t{s}\t{ex[s]}')
    print()
