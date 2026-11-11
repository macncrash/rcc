#![allow(dead_code)]
// t12: a C type system modelled on ccc src/common/types.rs: a CType enum with
// Box/Rc<str> payloads, Display as C type names, size/align, integer
// promotion, integer rank and usual arithmetic conversions (logic follows
// CType::integer_promoted / integer_rank / usual_arithmetic_conversion),
// align_up lifted verbatim, and a TypeContext computing struct/union layouts
// (including bitfields) stored in a map keyed by Rc<str>.
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

type RcStr = Rc<str>;

#[derive(Debug, Clone, PartialEq)]
enum CType {
    Void,
    Bool,
    Char,
    UChar,
    Short,
    UShort,
    Int,
    UInt,
    Long,
    ULong,
    LongLong,
    ULongLong,
    Int128,
    UInt128,
    Float,
    Double,
    Pointer(Box<CType>),
    Array(Box<CType>, Option<usize>),
    Function(Box<FunctionType>),
    Struct(RcStr),
    Union(RcStr),
    Enum(RcStr, Vec<(String, i64)>),
}

#[derive(Debug, Clone, PartialEq)]
struct FunctionType {
    ret: CType,
    params: Vec<CType>,
    variadic: bool,
}

// ---- lifted from ccc src/common/types.rs --------------------------------------
/// Align `offset` up to the next multiple of `align`.
pub fn align_up(offset: usize, align: usize) -> usize {
    if align == 0 { return offset; }
    let mask = align - 1;
    match offset.checked_add(mask) {
        Some(v) => v & !mask,
        None => offset, // overflow: return offset unchanged
    }
}

impl fmt::Display for CType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CType::Void => write!(f, "void"),
            CType::Bool => write!(f, "_Bool"),
            CType::Char => write!(f, "char"),
            CType::UChar => write!(f, "unsigned char"),
            CType::Short => write!(f, "short"),
            CType::UShort => write!(f, "unsigned short"),
            CType::Int => write!(f, "int"),
            CType::UInt => write!(f, "unsigned int"),
            CType::Long => write!(f, "long"),
            CType::ULong => write!(f, "unsigned long"),
            CType::LongLong => write!(f, "long long"),
            CType::ULongLong => write!(f, "unsigned long long"),
            CType::Int128 => write!(f, "__int128"),
            CType::UInt128 => write!(f, "unsigned __int128"),
            CType::Float => write!(f, "float"),
            CType::Double => write!(f, "double"),
            CType::Pointer(inner) => match &**inner {
                CType::Function(ft) => {
                    write!(f, "{} (*)(", ft.ret)?;
                    for (i, p) in ft.params.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", p)?;
                    }
                    if ft.variadic {
                        write!(f, ", ...")?;
                    }
                    write!(f, ")")
                }
                other => write!(f, "{} *", other),
            },
            CType::Array(inner, Some(n)) => write!(f, "{} [{}]", inner, n),
            CType::Array(inner, None) => write!(f, "{} []", inner),
            CType::Function(ft) => write!(f, "{} (fn/{})", ft.ret, ft.params.len()),
            CType::Struct(k) => write!(f, "struct {}", k.trim_start_matches("struct.")),
            CType::Union(k) => write!(f, "union {}", k.trim_start_matches("union.")),
            CType::Enum(name, _) => write!(f, "enum {}", name),
        }
    }
}

impl CType {
    fn is_unsigned(&self) -> bool {
        matches!(self, CType::Bool | CType::UChar | CType::UShort | CType::UInt | CType::ULong | CType::ULongLong | CType::UInt128)
    }
    fn is_integer(&self) -> bool {
        matches!(
            self,
            CType::Bool | CType::Char | CType::UChar | CType::Short | CType::UShort | CType::Int | CType::UInt | CType::Long | CType::ULong
                | CType::LongLong | CType::ULongLong | CType::Int128 | CType::UInt128 | CType::Enum(..)
        )
    }
    fn is_floating(&self) -> bool {
        matches!(self, CType::Float | CType::Double)
    }
    fn integer_promoted(&self) -> CType {
        match self {
            CType::Bool | CType::Char | CType::UChar | CType::Short | CType::UShort => CType::Int,
            CType::Enum(..) => CType::Int,
            other => other.clone(),
        }
    }
    fn integer_rank(&self) -> u32 {
        match self {
            CType::Bool => 0,
            CType::Char | CType::UChar => 1,
            CType::Short | CType::UShort => 2,
            CType::Int | CType::UInt => 3,
            CType::Long | CType::ULong => 4,
            CType::LongLong | CType::ULongLong => 5,
            CType::Int128 | CType::UInt128 => 6,
            _ => 3,
        }
    }
    fn to_unsigned(&self) -> CType {
        match self {
            CType::Int => CType::UInt,
            CType::Long => CType::ULong,
            CType::LongLong => CType::ULongLong,
            CType::Int128 => CType::UInt128,
            other => other.clone(),
        }
    }
    fn usual_arithmetic_conversion(lhs: &CType, rhs: &CType) -> CType {
        if matches!(lhs, CType::Double) || matches!(rhs, CType::Double) {
            return CType::Double;
        }
        if matches!(lhs, CType::Float) || matches!(rhs, CType::Float) {
            return CType::Float;
        }
        let l = lhs.integer_promoted();
        let r = rhs.integer_promoted();
        if l == r {
            return l;
        }
        let (lu, ru) = (l.is_unsigned(), r.is_unsigned());
        let (lr, rr) = (l.integer_rank(), r.integer_rank());
        if lu == ru {
            return if lr >= rr { l } else { r };
        }
        let (u, s) = if lu { (&l, &r) } else { (&r, &l) };
        if u.integer_rank() >= s.integer_rank() {
            return u.clone();
        }
        // signed type has greater rank: can it represent all unsigned values?
        if s.scalar_size() > u.scalar_size() {
            return s.clone();
        }
        s.to_unsigned()
    }
    fn scalar_size(&self) -> usize {
        match self {
            CType::Void | CType::Bool | CType::Char | CType::UChar => 1,
            CType::Short | CType::UShort => 2,
            CType::Int | CType::UInt | CType::Float | CType::Enum(..) => 4,
            CType::Long | CType::ULong | CType::LongLong | CType::ULongLong | CType::Double | CType::Pointer(_) => 8,
            CType::Int128 | CType::UInt128 => 16,
            _ => 0,
        }
    }
}

#[derive(Debug, Clone)]
struct Field {
    name: String,
    ty: CType,
    bit_width: Option<u32>,
}

#[derive(Debug, Clone)]
struct FieldLayout {
    name: String,
    offset: usize,
    bit_offset: u32,
    bit_width: Option<u32>,
}

#[derive(Debug, Clone)]
struct StructLayout {
    fields: Vec<FieldLayout>,
    size: usize,
    align: usize,
    is_union: bool,
}

struct TypeContext {
    layouts: HashMap<RcStr, StructLayout>,
    defs: HashMap<RcStr, (Vec<Field>, bool, Option<usize>)>, // fields, is_union, packed/aligned override
    typedefs: HashMap<String, CType>,
}

impl TypeContext {
    fn new() -> Self {
        TypeContext { layouts: HashMap::new(), defs: HashMap::new(), typedefs: HashMap::new() }
    }

    fn define(&mut self, key: &str, fields: Vec<Field>, is_union: bool, max_align: Option<usize>) -> RcStr {
        let k: RcStr = Rc::from(key);
        self.defs.insert(Rc::clone(&k), (fields, is_union, max_align));
        k
    }

    fn size_of(&mut self, t: &CType) -> usize {
        match t {
            CType::Array(inner, Some(n)) => self.size_of(inner) * n,
            CType::Array(_, None) => 0,
            CType::Struct(k) | CType::Union(k) => self.layout(k).size,
            CType::Function(_) => 1,
            other => other.scalar_size(),
        }
    }

    fn align_of(&mut self, t: &CType) -> usize {
        match t {
            CType::Array(inner, _) => self.align_of(inner),
            CType::Struct(k) | CType::Union(k) => self.layout(k).align,
            CType::Function(_) => 1,
            other => other.scalar_size().max(1),
        }
    }

    fn layout(&mut self, key: &RcStr) -> StructLayout {
        if let Some(l) = self.layouts.get(key) {
            return l.clone();
        }
        let (fields, is_union, max_align) = self.defs.get(key).cloned().expect("incomplete type");
        let mut out = Vec::new();
        let mut offset = 0usize;
        let mut size = 0usize;
        let mut align = 1usize;
        // bitfield packing state: (storage unit start, unit size, bits used)
        let mut unit: Option<(usize, usize, u32)> = None;
        for f in &fields {
            let fsize = self.size_of(&f.ty);
            let mut falign = self.align_of(&f.ty);
            if let Some(m) = max_align {
                falign = falign.min(m);
            }
            if is_union {
                out.push(FieldLayout { name: f.name.clone(), offset: 0, bit_offset: 0, bit_width: f.bit_width });
                size = size.max(fsize);
                align = align.max(falign);
                continue;
            }
            match f.bit_width {
                Some(0) => {
                    // zero-width bitfield: close the current unit
                    if let Some((start, usize_, _)) = unit.take() {
                        offset = start + usize_;
                    }
                    offset = align_up(offset, falign);
                }
                Some(w) => {
                    let fits = match unit {
                        Some((start, us, used)) if us == fsize => {
                            let _ = start;
                            used + w <= (us as u32) * 8
                        }
                        _ => false,
                    };
                    if !fits {
                        if let Some((start, us, _)) = unit {
                            offset = start + us;
                        }
                        offset = align_up(offset, falign);
                        unit = Some((offset, fsize, 0));
                    }
                    let (start, us, used) = unit.unwrap();
                    out.push(FieldLayout { name: f.name.clone(), offset: start, bit_offset: used, bit_width: Some(w) });
                    unit = Some((start, us, used + w));
                    align = align.max(falign);
                    size = size.max(start + us);
                }
                None => {
                    if let Some((start, us, _)) = unit.take() {
                        offset = start + us;
                    }
                    offset = align_up(offset, falign);
                    out.push(FieldLayout { name: f.name.clone(), offset, bit_offset: 0, bit_width: None });
                    offset += fsize;
                    size = size.max(offset);
                    align = align.max(falign);
                }
            }
        }
        if let Some((start, us, _)) = unit {
            size = size.max(start + us);
        }
        let size = align_up(size, align);
        let l = StructLayout { fields: out, size, align, is_union };
        self.layouts.insert(Rc::clone(key), l.clone());
        l
    }
}

fn field(name: &str, ty: CType) -> Field {
    Field { name: name.to_string(), ty, bit_width: None }
}

fn bits(name: &str, ty: CType, w: u32) -> Field {
    Field { name: name.to_string(), ty, bit_width: Some(w) }
}

fn ptr(t: CType) -> CType {
    CType::Pointer(Box::new(t))
}

fn arr(t: CType, n: usize) -> CType {
    CType::Array(Box::new(t), Some(n))
}

fn main() {
    let basic = [
        CType::Bool, CType::Char, CType::UChar, CType::Short, CType::UShort, CType::Int, CType::UInt, CType::Long, CType::ULong,
        CType::LongLong, CType::ULongLong, CType::Int128, CType::UInt128, CType::Float, CType::Double,
    ];
    let mut ctx = TypeContext::new();
    for t in &basic {
        println!("{:<20} size={:>2} align={:>2} unsigned={} promoted={} rank={}", t.to_string(), ctx.size_of(t), ctx.align_of(t), t.is_unsigned(), t.integer_promoted(), t.integer_rank());
    }
    println!("--- usual arithmetic conversions");
    let probe = [CType::Char, CType::UShort, CType::Int, CType::UInt, CType::Long, CType::ULong, CType::LongLong, CType::Float, CType::Double];
    let mut header = format!("{:>14}", "");
    for r in &probe {
        header.push_str(&format!(" {:>6}", short(r)));
    }
    println!("{}", header);
    for l in &probe {
        let mut row = format!("{:>14}", l.to_string());
        for r in &probe {
            row.push_str(&format!(" {:>6}", short(&CType::usual_arithmetic_conversion(l, r))));
        }
        println!("{}", row);
    }

    println!("--- struct layouts");
    let point = ctx.define("struct.point", vec![field("x", CType::Int), field("y", CType::Int)], false, None);
    let node = ctx.define(
        "struct.node",
        vec![field("tag", CType::Char), field("value", CType::Double), field("next", ptr(CType::Struct(Rc::from("struct.node")))), field("small", CType::Short)],
        false,
        None,
    );
    let flags = ctx.define(
        "struct.flags",
        vec![bits("a", CType::UInt, 3), bits("b", CType::UInt, 5), bits("c", CType::UInt, 30), field("ch", CType::Char), bits("d", CType::UChar, 4), bits("pad", CType::UChar, 0), bits("e", CType::UChar, 2)],
        false,
        None,
    );
    let value = ctx.define("union.value", vec![field("i", CType::Int), field("d", CType::Double), field("bytes", arr(CType::UChar, 12))], true, None);
    let outer = ctx.define(
        "struct.outer",
        vec![field("kind", CType::UChar), field("v", CType::Union(Rc::clone(&value))), field("pts", arr(CType::Struct(Rc::clone(&point)), 3)), field("big", CType::Int128), field("tail", CType::Char)],
        false,
        None,
    );
    let packed = ctx.define("struct.packed", vec![field("c", CType::Char), field("i", CType::Int), field("l", CType::Long)], false, Some(1));
    for k in [&point, &node, &flags, &value, &outer, &packed] {
        let l = ctx.layout(k);
        let t = if l.is_union { CType::Union(Rc::clone(k)) } else { CType::Struct(Rc::clone(k)) };
        println!("{}: size={} align={}", t, l.size, l.align);
        for f in &l.fields {
            match f.bit_width {
                Some(w) => println!("    {:<6} offset={:>2} bits {}..{}", f.name, f.offset, f.bit_offset, f.bit_offset + w),
                None => println!("    {:<6} offset={:>2}", f.name, f.offset),
            }
        }
    }
    println!("rc count for struct.point key: {}", Rc::strong_count(&point));

    println!("--- derived types");
    let fnty = CType::Function(Box::new(FunctionType { ret: CType::Int, params: vec![ptr(CType::Char), CType::Long], variadic: true }));
    let color = CType::Enum("color".into(), vec![("RED".into(), 0), ("GREEN".into(), 1), ("BLUE".into(), 4)]);
    ctx.typedefs.insert("size_t".into(), CType::ULong);
    ctx.typedefs.insert("callback".into(), ptr(fnty.clone()));
    let derived = vec![
        ptr(ptr(CType::Char)),
        arr(arr(CType::Int, 4), 3),
        arr(ptr(CType::Struct(Rc::clone(&node))), 2),
        ptr(fnty.clone()),
        CType::Array(Box::new(CType::Double), None),
        color.clone(),
        ctx.typedefs["size_t"].clone(),
        ctx.typedefs["callback"].clone(),
        CType::Struct(Rc::clone(&outer)),
    ];
    for t in &derived {
        println!("{:<34} size={:>3} align={:>2} integer={} float={}", t.to_string(), ctx.size_of(t), ctx.align_of(t), t.is_integer(), t.is_floating());
    }
    if let CType::Enum(name, variants) = &color {
        let max = variants.iter().map(|(_, v)| *v).max().unwrap();
        println!("enum {} has {} variants, max {} -> {}", name, variants.len(), max, CType::usual_arithmetic_conversion(&color, &CType::UInt));
    }
    for (o, a) in [(0usize, 8usize), (1, 8), (8, 8), (13, 4), (13, 0), (usize::MAX - 2, 8), (100, 64)] {
        println!("align_up({}, {}) = {}", o, a, align_up(o, a));
    }
    let mut keys: Vec<&RcStr> = ctx.layouts.keys().collect();
    keys.sort();
    println!("cached layouts: {:?}", keys);
}

fn short(t: &CType) -> &'static str {
    match t {
        CType::Char => "c",
        CType::UShort => "us",
        CType::Int => "i",
        CType::UInt => "u",
        CType::Long => "l",
        CType::ULong => "ul",
        CType::LongLong => "ll",
        CType::ULongLong => "ull",
        CType::Float => "f",
        CType::Double => "d",
        _ => "?",
    }
}
