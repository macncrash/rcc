#!/usr/bin/env python3
"""Source-level (grep-style) feature counts over the test-stripped ccc tree
produced by strip_tests.py.  Comments are removed and string-literal contents
blanked before matching (format strings are analysed separately by
fmt_specs.py).

Usage: features.py STRIPPED_ROOT > features.txt
"""
import os
import re
import sys
import collections

root = sys.argv[1]
sys.path.insert(0, os.path.dirname(__file__))
from strip_tests import skip_literal  # noqa: E402


def blank(src):
    """Remove comments, blank string/char literal contents (keep quotes)."""
    out = []
    i, n = 0, len(src)
    while i < n:
        if src.startswith('//', i):
            j = src.find('\n', i)
            j = n if j < 0 else j
            out.append(' ' * (j - i))
            i = j
            continue
        if src.startswith('/*', i):
            depth, j = 0, i
            while j < n:
                if src.startswith('/*', j):
                    depth += 1; j += 2
                elif src.startswith('*/', j):
                    depth -= 1; j += 2
                    if depth == 0:
                        break
                else:
                    j += 1
            out.append(re.sub(r'[^\n]', ' ', src[i:j]))
            i = j
            continue
        if src[i] in '"\'rb':
            if i > 0 and (src[i - 1].isalnum() or src[i - 1] == '_') and src[i] in 'rb':
                out.append(src[i]); i += 1
                continue
            e = skip_literal(src, i)
            if e is not None:
                lit = src[i:e]
                q = lit[0] if lit[0] in '"\'' else ('"' if '"' in lit[:4] else "'")
                if q == "'":
                    out.append("'c'" + re.sub(r'[^\n]', ' ', lit[3:]) if len(lit) >= 3 else lit)
                else:
                    out.append('"' + re.sub(r'[^\n]', ' ', lit[1:-1]) + '"' if len(lit) >= 2 else lit)
                i = e
                continue
        out.append(src[i]); i += 1
    return ''.join(out)


files = {}
for d, _, fs in os.walk(os.path.join(root, 'src')):
    for f in fs:
        if f.endswith('.rs'):
            p = os.path.join(d, f)
            rel = os.path.relpath(p, root)
            files[rel] = blank(open(p).read())

C = collections.Counter()
EX = {}


def count(name, pattern, flags=re.M):
    rx = re.compile(pattern, flags)
    for rel, s in files.items():
        for m in rx.finditer(s):
            C[name] += 1
            if name not in EX:
                EX[name] = f'{rel}:{s.count(chr(10), 0, m.start()) + 1}'
    C.setdefault(name, 0)


def lister(name, pattern, group=1, flags=re.M):
    rx = re.compile(pattern, flags)
    res = collections.Counter()
    ex = {}
    for rel, s in files.items():
        for m in rx.finditer(s):
            k = m.group(group)
            res[k] += 1
            ex.setdefault(k, f'{rel}:{s.count(chr(10), 0, m.start()) + 1}')
    print(f'\n## {name} ({sum(res.values())} total, {len(res)} distinct)')
    for k, v in res.most_common():
        print(f'{v:6d}  {k}   e.g. {ex[k]}')
    return res


total_lines = sum(s.count('\n') for s in files.values())
nonblank = sum(1 for s in files.values() for l in s.split('\n') if l.strip())
print(f'files {len(files)}  lines {total_lines}  non-blank non-comment lines {nonblank}')

# --- items / generics
count('fn items', r'\bfn\s+[a-z_][a-z0-9_]*')
count('generic fns  fn x<', r'\bfn\s+\w+\s*<')
count('generic fns with lifetimes only  fn x<\'a>', r"\bfn\s+\w+\s*<\s*'[a-z_]+\s*(,\s*'[a-z_]+\s*)*>")
count('generic structs', r'\bstruct\s+\w+\s*<')
count('structs with lifetime param', r"\bstruct\s+\w+\s*<\s*'")
count('generic enums', r'\benum\s+\w+\s*<')
count('enums with lifetime param', r"\benum\s+\w+\s*<\s*'")
count('generic impls  impl<', r'\bimpl\s*<')
count('impl<\'a ...> (lifetime generic impls)', r"\bimpl\s*<\s*'")
count('const generics  <const N', r'[<,]\s*const\s+[A-Z_]+\s*:')
count('where clauses', r'\bwhere\b')
count('structs', r'\bstruct\s+\w+')
count('tuple structs', r'\bstruct\s+\w+\s*(<[^>]*>)?\s*\(')
count('unit structs', r'\bstruct\s+\w+\s*;')
count('enums', r'\benum\s+\w+')
count('traits', r'\btrait\s+\w+')
count('trait impls  impl ... for', r'\bimpl\b[^{;]*?\bfor\b[^{;]*\{')
count('inherent impls', r'^\s*impl\b(?![^{;]*\bfor\b)[^{;]*\{')
count('type aliases', r'^\s*(pub(\([^)]*\))?\s+)?type\s+\w+')
count('associated type decls/defs  type X', r'^\s{4,}type\s+\w+')
count('associated consts (indented const in impl/trait)', r'^\s{4}(pub(\([^)]*\))?\s+)?const\s+[A-Z_0-9]+\s*:')
count('const items (all)', r'\bconst\s+[A-Z_][A-Z_0-9]*\s*:')
count('const fn', r'\bconst\s+fn\b')
count('static items', r'\bstatic\s+(mut\s+)?[A-Z_][A-Z_0-9]*\s*:')
count('static mut', r'\bstatic\s+mut\b')
count('thread_local!', r'\bthread_local!')
count('unsafe blocks/fns/impls', r'\bunsafe\b')
count('raw pointer types', r'\*(const|mut)\s')
count('extern blocks/fns', r'\bextern\b')
count('dyn', r'\bdyn\b')
count('&dyn', r'&\s*(mut\s+)?dyn\b')
count('Box<dyn', r'Box<\s*dyn\b')
count('impl Trait in arg position', r'[:(,]\s*(&\s*(mut\s+)?)?impl\s+[A-Z]')
count('impl Trait in return position', r'->\s*impl\b')
count('fn pointer types fn(', r'(?<![\w])fn\s*\(')
count('Fn/FnMut/FnOnce bounds', r'\b(Fn|FnMut|FnOnce)\s*\(')
count('move closures', r'\bmove\s*\|')
count('closures |..| (rough)', r'(?<![|\w)\]])\|[^|\n]*\|\s*[^|=]')
count('? operator', r'[\w)\]]\?(?![\w])')
count('let-else', r'\blet\b[^;=]*=[^;]*\belse\s*\{')
count('if let', r'\bif\s+let\b')
count('while let', r'\bwhile\s+let\b')
count('let chains (if let .. && let)', r'\bif\s+let\b[^{]*&&\s*let\b')
count('matches!', r'\bmatches!')
count('labels \'x: loop/while/for/{', r"'[a-z_]+\s*:\s*(loop|while|for|\{)")
count('labelled break/continue', r"\b(break|continue)\s+'[a-z_]+")
count('break with value (break expr)', r"\bbreak\s+(?!'|;)[^;}\n]")
count('loop', r'\bloop\b')
count('while', r'\bwhile\b')
count('for ... in', r'\bfor\b\s+[^{]*?\bin\b')
count('match', r'\bmatch\b')
count('slice patterns  [a, .., b]', r'\[[^\]\n]*\.\.[^\]\n=]*\]\s*(=>|=|\|)|let\s*\[')
count('rest pattern .. inside slice/tuple pattern', r'[\[(,]\s*\.\.\s*[\]),]')
count('@ bindings', r'\b[a-z_]\w*\s*@\s*')
count('range patterns ..=', r"(\d|')\s*\.\.=\s*[\d'-]")
count('exclusive range patterns a..b =>', r'\b\d+\.\.\d+\s*(=>|\|)')
count('ref patterns', r'\bref\s+(?!mut)\w')
count('ref mut patterns', r'\bref\s+mut\b')
count('box patterns', r'\bbox\s+\w')
count('as casts', r'\bas\s+(u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize|f32|f64|char|bool)\b')
count('as casts to pointers/other', r'\bas\s+(\*|&|[A-Z])')
count('u128 mentions', r'\bu128\b')
count('i128 mentions', r'\bi128\b')
count('f32 mentions', r'\bf32\b')
count('f64 mentions', r'\bf64\b')
count('pub(crate)', r'\bpub\s*\(\s*crate\s*\)')
count('pub(super)', r'\bpub\s*\(\s*super\s*\)')
count('pub(in ..)', r'\bpub\s*\(\s*in\b')
count('glob imports  use ..::*', r'\buse\b[^;]*::\*\s*;')
count('glob imports in braces {..*..}', r'\buse\b[^;]*\{[^;]*\*[^;]*\}\s*;')
count('pub use re-exports', r'\bpub(\([^)]*\))?\s+use\b')
count('use inside fn bodies (indented use)', r'^\s{4,}use\s')
count('use self::/super::', r'\buse\s+(self|super)::')
count('use crate::', r'\buse\s+crate::')
count('extern crate', r'\bextern\s+crate\b')
count('#[derive', r'#\[derive')
count('#[repr', r'#\[repr')
count('#[default]', r'#\[default\]')
count('#[inline', r'#\[inline')
count('#[cfg_attr', r'#\[cfg_attr')
count('#[allow', r'#\[allow')
count('#[must_use', r'#\[must_use')
count('impl Drop', r'\bimpl\b[^{]*\bDrop\s+for\b')
count('turbofish ::<', r'::<')
count('tuple index .0/.1', r'\.\d+\b')
count('nested closures returning closures (-> impl Fn)', r'->\s*impl\s+Fn')
count('Box<dyn Fn', r'Box<\s*dyn\s+Fn')
count('trait objects in struct fields (field: Box<dyn / &dyn)', r'^\s+\w+\s*:\s*(Box<\s*dyn|&\'?\w*\s*(mut\s+)?dyn)')
count('struct update syntax ..base', r'\.\.\s*(Default::default\(\)|self\b|[a-z_]\w*)\s*\}')
count('Self in exprs/types', r'\bSelf\b')
count('loop break-value  let x = loop', r'=\s*loop\b')
count('unreachable!', r'\bunreachable!')
count('todo!/unimplemented!', r'\b(todo|unimplemented)!')
count('async/await', r'\b(async|await)\b')
count('impl blocks with generic bound <T: ', r'\bimpl\s*<\s*[A-Z]\w*\s*:')
count('integer literal suffixes', r'\b\d[\d_]*(u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize)\b')
count('hex literals', r'\b0x[0-9a-fA-F_]+')
count('binary literals', r'\b0b[01_]+')
count('octal literals', r'\b0o[0-7_]+')
count('float literals with suffix', r'\b\d[\d_]*(\.\d+)?(e[+-]?\d+)?f(32|64)\b')
count('char literals (blanked)', r"'c'")
count('byte string literals b"', r'\bb"')
count('raw strings r"/r#"', r'\br#*"')
count('Default::default()', r'\bDefault::default\(\)')
count('.into()', r'\.into\(\)')
count('From::from / X::from(', r'\b\w+::from\(')
count('mem::take/replace/swap', r'\bmem::(take|replace|swap)\b')
count('std::mem::discriminant', r'\bdiscriminant\(')
count('assert! family', r'\b(debug_)?assert(_eq|_ne)?!')
count('debug_assert family (type-checked but dead in release)', r'\bdebug_assert(_eq|_ne)?!')
count('cfg!(...)', r'\bcfg!')
count('nested macro_rules', r'\bmacro_rules!')
count('#[macro_export]', r'#\[macro_export\]')
count('#[macro_use]', r'#\[macro_use\]')
count('impl Trait for &T / for &\'a T', r"\bimpl\b[^{]*\bfor\s+&")
count('trait with generic params trait X<', r'\btrait\s+\w+\s*<')
count('supertraits trait X: Y', r'\btrait\s+\w+\s*(<[^>]*>)?\s*:')
count('default type params <T = ', r'<[^>]*\b[A-Z]\w*\s*=\s*[A-Z]')
count('array types [T; N]', r'\[[^\[\];]+;\s*[^\]]+\]')
count('array repeat exprs [x; n]', r'=\s*\[[^\[\];]+;\s*[^\]]+\]')
count('closures stored: field: fn(/Box<dyn Fn', r'^\s+\w+\s*:\s*(fn\(|Box<dyn Fn|&\'?\w*\s*dyn Fn)')
count('char ranges in match', r"'c'\s*\.\.=\s*'c'")
count('negative literal patterns', r'(=>|\|)\s*-\d|^\s*-\d+\s*(=>|\|)')
count('return statements', r'\breturn\b')
count('#[cfg(not(feature = ...))] blocks kept', r'#\[cfg\(not\(feature')

for k in C:
    print(f'{C[k]:7d}  {k}' + (f'   e.g. {EX[k]}' if k in EX else ''))

lister('macro invocations', r'\b([a-z_][a-z0-9_]*)!\s*[\(\[\{]')
lister('derives (per trait)', r'#\[derive\(([^)]*)\)\]')
lister('attributes (outer)', r'#\[\s*([a-z_]+)')
lister('inner attributes #![..]', r'#!\[\s*([a-z_]+[^\]]*)')
lister('trait impls  impl X for Y (trait names)', r'\bimpl\b\s*(?:<[^{;]*?>)?\s*([A-Za-z_:]+(?:<[^{;]*?>)?)\s+for\b')
lister('traits defined', r'\btrait\s+(\w+[^{]*)\{')
lister('dyn Trait forms', r'\bdyn\s+([A-Za-z_:]+(?:<[^>]*>)?(?:\([^)]*\)(?:\s*->\s*[\w:<>&\' ]+)?)?)')
lister('impl Trait forms', r'\bimpl\s+((?:Fn|FnMut|FnOnce|Iterator|IntoIterator|Into|AsRef|Display|Debug|Write|Clone|Copy|PartialEq|Eq|Hash|Ord|PartialOrd|Default|ToString|Sized|DoubleEndedIterator|ExactSizeIterator)\b[^{;,)=]*)')
lister('where-clause / bound forms  T: Bound', r'[<,]\s*([A-Z]\w*\s*:\s*[^,>{]+)')
lister('repr attributes', r'#\[repr\(([^)]*)\)\]')
lister('inline attributes', r'#\[(inline[^\]]*)\]')
lister('allow attributes', r'#\[allow\(([^)]*)\)\]')
lister('cfg_attr', r'#\[cfg_attr\(([^\]]*)\)\]')
lister('std paths in use statements', r'\buse\s+(std::[^;]+);')
lister('fully qualified std:: paths in code (not use)', r'(?<!use )\b(std::[a-z_]+(?:::[A-Za-z_]+)*)')
lister('core::/alloc:: paths', r'\b((?:core|alloc)::[A-Za-z_:]+)')
lister('associated consts of primitives  u32::MAX etc', r'\b((?:u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize|f32|f64|char)::[A-Z][A-Z_0-9]*)\b')
lister('std::f64/f32 consts', r'\b((?:std::)?f(?:32|64)::consts::\w+)')
lister('as-cast targets', r'\bas\s+([a-z0-9]+|\*\s*(?:const|mut)\s+\w+|[A-Z]\w*)\b')
lister('labels', r"('[a-z_]+)\s*:\s*(?:loop|while|for|\{)")
lister('macro_rules names', r'\bmacro_rules!\s*(\w+)')
lister('local type names that shadow std/prelude names',
       r'\b(?:struct|enum|type|trait)\s+(Option|Result|Vec|String|Box|Rc|Cell|RefCell|Ordering|Iter|IntoIter|Entry|Error|Path|File|Cow|Range|Default|Display|Debug|Write|Read|Hash|Hasher|Instant|Duration|Arguments|Formatter|Map|Set|SmallVec|Value|Target|Operand|Symbol)\b')
lister('local fns that shadow common std names', r'\bfn\s+(min|max|swap|replace|take|drop|format|write|print|panic|from|into|default|clone|hash|eq|cmp|len|is_empty)\b')
lister('enum variants named like prelude (Some/None/Ok/Err) in local enums', r'^\s+(Some|None|Ok|Err)\s*[,({]')
