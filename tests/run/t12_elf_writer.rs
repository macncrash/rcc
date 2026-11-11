#![allow(dead_code)]
// t12: an ELF64/ELF32 relocatable-object writer and reader.
// The little-endian read/write helpers, section/symbol/relocation writers are
// lifted verbatim from ccc src/backend/elf/io.rs; constants from
// src/backend/elf/constants.rs; StringTable from src/backend/elf/string_table.rs.
use std::collections::HashMap;
use std::fmt::Write as _;

// ── lifted from ccc src/backend/elf/constants.rs ──────────────────────────────
pub const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
pub const ELFCLASS32: u8 = 1;
pub const ELFCLASS64: u8 = 2;
pub const ELFDATA2LSB: u8 = 1;
pub const EV_CURRENT: u8 = 1;
pub const ELFOSABI_NONE: u8 = 0;
pub const ET_REL: u16 = 1;
pub const ET_EXEC: u16 = 2;
pub const EM_386: u16 = 3;
pub const EM_X86_64: u16 = 62;
pub const EM_AARCH64: u16 = 183;
pub const EM_RISCV: u16 = 243;
pub const SHT_NULL: u32 = 0;
pub const SHT_PROGBITS: u32 = 1;
pub const SHT_SYMTAB: u32 = 2;
pub const SHT_STRTAB: u32 = 3;
pub const SHT_RELA: u32 = 4;
pub const SHT_NOBITS: u32 = 8;
pub const SHF_WRITE: u64 = 0x1;
pub const SHF_ALLOC: u64 = 0x2;
pub const SHF_EXECINSTR: u64 = 0x4;
pub const SHF_INFO_LINK: u64 = 0x40;
pub const STB_LOCAL: u8 = 0;
pub const STB_GLOBAL: u8 = 1;
pub const STB_WEAK: u8 = 2;
pub const STT_NOTYPE: u8 = 0;
pub const STT_OBJECT: u8 = 1;
pub const STT_FUNC: u8 = 2;
pub const STT_SECTION: u8 = 3;
pub const R_X86_64_64: u32 = 1;
pub const R_X86_64_PC32: u32 = 2;
pub const R_X86_64_PLT32: u32 = 4;
pub const PT_LOAD: u32 = 1;
pub const PF_X: u32 = 1;
pub const PF_W: u32 = 2;
pub const PF_R: u32 = 4;

// ── lifted from ccc src/backend/elf/io.rs ─────────────────────────────────────
#[inline]
pub fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}
#[inline]
pub fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
}
#[inline]
pub fn read_u64(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
        data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7],
    ])
}
#[inline]
pub fn read_i64(data: &[u8], offset: usize) -> i64 {
    i64::from_le_bytes([
        data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
        data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7],
    ])
}
pub fn read_cstr(data: &[u8], offset: usize) -> String {
    if offset >= data.len() {
        return String::new();
    }
    let end = data[offset..].iter().position(|&b| b == 0).unwrap_or(data.len() - offset);
    String::from_utf8_lossy(&data[offset..offset + end]).into_owned()
}
#[inline]
pub fn w16(buf: &mut [u8], off: usize, val: u16) {
    if off + 2 <= buf.len() {
        buf[off..off + 2].copy_from_slice(&val.to_le_bytes());
    }
}
#[inline]
pub fn w32(buf: &mut [u8], off: usize, val: u32) {
    if off + 4 <= buf.len() {
        buf[off..off + 4].copy_from_slice(&val.to_le_bytes());
    }
}
#[inline]
pub fn w64(buf: &mut [u8], off: usize, val: u64) {
    if off + 8 <= buf.len() {
        buf[off..off + 8].copy_from_slice(&val.to_le_bytes());
    }
}
#[inline]
pub fn write_bytes(buf: &mut [u8], off: usize, data: &[u8]) {
    let end = off + data.len();
    if end <= buf.len() {
        buf[off..end].copy_from_slice(data);
    }
}
pub fn write_shdr64(
    buf: &mut Vec<u8>,
    sh_name: u32, sh_type: u32, sh_flags: u64,
    sh_addr: u64, sh_offset: u64, sh_size: u64,
    sh_link: u32, sh_info: u32, sh_addralign: u64, sh_entsize: u64,
) {
    buf.extend_from_slice(&sh_name.to_le_bytes());
    buf.extend_from_slice(&sh_type.to_le_bytes());
    buf.extend_from_slice(&sh_flags.to_le_bytes());
    buf.extend_from_slice(&sh_addr.to_le_bytes());
    buf.extend_from_slice(&sh_offset.to_le_bytes());
    buf.extend_from_slice(&sh_size.to_le_bytes());
    buf.extend_from_slice(&sh_link.to_le_bytes());
    buf.extend_from_slice(&sh_info.to_le_bytes());
    buf.extend_from_slice(&sh_addralign.to_le_bytes());
    buf.extend_from_slice(&sh_entsize.to_le_bytes());
}
pub fn write_phdr64(
    buf: &mut [u8], off: usize,
    p_type: u32, p_flags: u32, p_offset: u64,
    p_vaddr: u64, p_paddr: u64, p_filesz: u64, p_memsz: u64, p_align: u64,
) {
    w32(buf, off, p_type);
    w32(buf, off + 4, p_flags);
    w64(buf, off + 8, p_offset);
    w64(buf, off + 16, p_vaddr);
    w64(buf, off + 24, p_paddr);
    w64(buf, off + 32, p_filesz);
    w64(buf, off + 40, p_memsz);
    w64(buf, off + 48, p_align);
}
#[inline]
pub fn wphdr(buf: &mut [u8], off: usize, pt: u32, flags: u32, foff: u64, va: u64, fsz: u64, msz: u64, align: u64) {
    write_phdr64(buf, off, pt, flags, foff, va, va, fsz, msz, align);
}
pub fn write_sym64(
    buf: &mut Vec<u8>,
    st_name: u32, st_info: u8, st_other: u8, st_shndx: u16,
    st_value: u64, st_size: u64,
) {
    buf.extend_from_slice(&st_name.to_le_bytes());
    buf.push(st_info);
    buf.push(st_other);
    buf.extend_from_slice(&st_shndx.to_le_bytes());
    buf.extend_from_slice(&st_value.to_le_bytes());
    buf.extend_from_slice(&st_size.to_le_bytes());
}
pub fn write_sym32(
    buf: &mut Vec<u8>,
    st_name: u32, st_value: u32, st_size: u32,
    st_info: u8, st_other: u8, st_shndx: u16,
) {
    buf.extend_from_slice(&st_name.to_le_bytes());
    buf.extend_from_slice(&st_value.to_le_bytes());
    buf.extend_from_slice(&st_size.to_le_bytes());
    buf.push(st_info);
    buf.push(st_other);
    buf.extend_from_slice(&st_shndx.to_le_bytes());
}
pub fn write_rela64(buf: &mut Vec<u8>, r_offset: u64, r_sym: u32, r_type: u32, r_addend: i64) {
    buf.extend_from_slice(&r_offset.to_le_bytes());
    let r_info: u64 = ((r_sym as u64) << 32) | (r_type as u64);
    buf.extend_from_slice(&r_info.to_le_bytes());
    buf.extend_from_slice(&r_addend.to_le_bytes());
}
pub fn write_rel32(buf: &mut Vec<u8>, r_offset: u32, r_sym: u32, r_type: u8) {
    buf.extend_from_slice(&r_offset.to_le_bytes());
    let r_info: u32 = (r_sym << 8) | (r_type as u32);
    buf.extend_from_slice(&r_info.to_le_bytes());
}

// ── lifted from ccc src/backend/elf/string_table.rs ───────────────────────────
pub struct StringTable {
    data: Vec<u8>,
    offsets: HashMap<String, u32>,
}
impl StringTable {
    pub fn new() -> Self {
        Self { data: vec![0], offsets: HashMap::new() }
    }
    pub fn add(&mut self, s: &str) -> u32 {
        if s.is_empty() {
            return 0;
        }
        if let Some(&offset) = self.offsets.get(s) {
            return offset;
        }
        let offset = self.data.len() as u32;
        self.data.extend_from_slice(s.as_bytes());
        self.data.push(0);
        self.offsets.insert(s.to_string(), offset);
        offset
    }
    pub fn offset_of(&self, s: &str) -> u32 {
        self.offsets.get(s).copied().unwrap_or(0)
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

// ── object model ─────────────────────────────────────────────────────────────

struct Section {
    name: &'static str,
    sh_type: u32,
    flags: u64,
    data: Vec<u8>,
    nobits_size: u64,
    align: u64,
    relocs: Vec<Reloc>,
}

struct Reloc {
    offset: u64,
    symbol: &'static str,
    r_type: u32,
    addend: i64,
}

struct Symbol {
    name: &'static str,
    section: Option<usize>, // index into sections (None = undefined)
    value: u64,
    size: u64,
    binding: u8,
    sym_type: u8,
}

fn align_up(v: usize, a: usize) -> usize {
    if a <= 1 { v } else { (v + a - 1) & !(a - 1) }
}

fn write_elf64(sections: &[Section], symbols: &[Symbol], machine: u16) -> Vec<u8> {
    let mut shstrtab = StringTable::new();
    let mut strtab = StringTable::new();
    // Section header index layout: 0 null, 1..=n user sections, then a
    // .rela.X for every section with relocs, then .symtab, .strtab, .shstrtab
    let n = sections.len();
    let rela_secs: Vec<usize> = (0..n).filter(|&i| !sections[i].relocs.is_empty()).collect();
    let symtab_idx = 1 + n + rela_secs.len();
    let strtab_idx = symtab_idx + 1;
    let shstrtab_idx = strtab_idx + 1;
    let shnum = shstrtab_idx + 1;

    // symbols: null, section symbols, locals, then globals (ELF requires locals first)
    let mut symbuf = Vec::new();
    write_sym64(&mut symbuf, 0, 0, 0, 0, 0, 0);
    let mut sym_index: HashMap<&str, u32> = HashMap::new();
    let mut count = 1u32;
    for i in 0..n {
        write_sym64(&mut symbuf, 0, (STB_LOCAL << 4) | STT_SECTION, 0, (i + 1) as u16, 0, 0);
        count += 1;
    }
    let mut ordered: Vec<&Symbol> = symbols.iter().filter(|s| s.binding == STB_LOCAL).collect();
    let first_global = count + ordered.len() as u32;
    ordered.extend(symbols.iter().filter(|s| s.binding != STB_LOCAL));
    for s in &ordered {
        let name = strtab.add(s.name);
        let shndx = s.section.map(|i| (i + 1) as u16).unwrap_or(0);
        write_sym64(&mut symbuf, name, (s.binding << 4) | s.sym_type, 0, shndx, s.value, s.size);
        sym_index.insert(s.name, count);
        count += 1;
    }

    let mut out = vec![0u8; 64];
    let mut offsets = vec![0u64; shnum];
    let mut sizes = vec![0u64; shnum];
    for (i, s) in sections.iter().enumerate() {
        let off = align_up(out.len(), s.align as usize);
        out.resize(off, 0);
        offsets[i + 1] = off as u64;
        if s.sh_type == SHT_NOBITS {
            sizes[i + 1] = s.nobits_size;
        } else {
            out.extend_from_slice(&s.data);
            sizes[i + 1] = s.data.len() as u64;
        }
    }
    for (k, &si) in rela_secs.iter().enumerate() {
        let off = align_up(out.len(), 8);
        out.resize(off, 0);
        let mut buf = Vec::new();
        for r in &sections[si].relocs {
            let sym = sym_index.get(r.symbol).copied().unwrap_or(0);
            write_rela64(&mut buf, r.offset, sym, r.r_type, r.addend);
        }
        offsets[1 + n + k] = off as u64;
        sizes[1 + n + k] = buf.len() as u64;
        out.extend_from_slice(&buf);
    }
    let off = align_up(out.len(), 8);
    out.resize(off, 0);
    offsets[symtab_idx] = off as u64;
    sizes[symtab_idx] = symbuf.len() as u64;
    out.extend_from_slice(&symbuf);
    offsets[strtab_idx] = out.len() as u64;
    sizes[strtab_idx] = strtab.len() as u64;
    out.extend_from_slice(strtab.as_bytes());

    // section names (must be added before writing .shstrtab)
    let mut names = vec![0u32; shnum];
    for (i, s) in sections.iter().enumerate() {
        names[i + 1] = shstrtab.add(s.name);
    }
    for (k, &si) in rela_secs.iter().enumerate() {
        names[1 + n + k] = shstrtab.add(&format!(".rela{}", sections[si].name));
    }
    names[symtab_idx] = shstrtab.add(".symtab");
    names[strtab_idx] = shstrtab.add(".strtab");
    names[shstrtab_idx] = shstrtab.add(".shstrtab");
    offsets[shstrtab_idx] = out.len() as u64;
    sizes[shstrtab_idx] = shstrtab.len() as u64;
    out.extend_from_slice(shstrtab.as_bytes());

    let shoff = align_up(out.len(), 8);
    out.resize(shoff, 0);
    let mut sh = Vec::new();
    write_shdr64(&mut sh, 0, SHT_NULL, 0, 0, 0, 0, 0, 0, 0, 0);
    for (i, s) in sections.iter().enumerate() {
        write_shdr64(&mut sh, names[i + 1], s.sh_type, s.flags, 0, offsets[i + 1], sizes[i + 1], 0, 0, s.align, 0);
    }
    for (k, &si) in rela_secs.iter().enumerate() {
        write_shdr64(&mut sh, names[1 + n + k], SHT_RELA, SHF_INFO_LINK, 0, offsets[1 + n + k], sizes[1 + n + k], symtab_idx as u32, (si + 1) as u32, 8, 24);
    }
    write_shdr64(&mut sh, names[symtab_idx], SHT_SYMTAB, 0, 0, offsets[symtab_idx], sizes[symtab_idx], strtab_idx as u32, first_global, 8, 24);
    write_shdr64(&mut sh, names[strtab_idx], SHT_STRTAB, 0, 0, offsets[strtab_idx], sizes[strtab_idx], 0, 0, 1, 0);
    write_shdr64(&mut sh, names[shstrtab_idx], SHT_STRTAB, 0, 0, offsets[shstrtab_idx], sizes[shstrtab_idx], 0, 0, 1, 0);
    out.extend_from_slice(&sh);

    // ELF header
    write_bytes(&mut out, 0, &ELF_MAGIC);
    out[4] = ELFCLASS64;
    out[5] = ELFDATA2LSB;
    out[6] = EV_CURRENT;
    out[7] = ELFOSABI_NONE;
    w16(&mut out, 16, ET_REL);
    w16(&mut out, 18, machine);
    w32(&mut out, 20, EV_CURRENT as u32);
    w64(&mut out, 24, 0); // e_entry
    w64(&mut out, 32, 0); // e_phoff
    w64(&mut out, 40, shoff as u64);
    w32(&mut out, 48, 0); // e_flags
    w16(&mut out, 52, 64); // e_ehsize
    w16(&mut out, 54, 0);
    w16(&mut out, 56, 0);
    w16(&mut out, 58, 64); // e_shentsize
    w16(&mut out, 60, shnum as u16);
    w16(&mut out, 62, shstrtab_idx as u16);
    out
}

fn hexdump(data: &[u8], start: usize, len: usize) -> String {
    let mut s = String::new();
    let end = (start + len).min(data.len());
    let mut off = start;
    while off < end {
        let row = &data[off..(off + 16).min(end)];
        write!(s, "{:08x}  ", off).unwrap();
        for i in 0..16 {
            if i < row.len() {
                write!(s, "{:02x} ", row[i]).unwrap();
            } else {
                s.push_str("   ");
            }
            if i == 7 {
                s.push(' ');
            }
        }
        s.push_str(" |");
        for &b in row {
            s.push(if (0x20..0x7f).contains(&b) { b as char } else { '.' });
        }
        s.push_str("|\n");
        off += 16;
    }
    s
}

fn sh_type_name(t: u32) -> &'static str {
    match t {
        SHT_NULL => "NULL",
        SHT_PROGBITS => "PROGBITS",
        SHT_SYMTAB => "SYMTAB",
        SHT_STRTAB => "STRTAB",
        SHT_RELA => "RELA",
        SHT_NOBITS => "NOBITS",
        _ => "?",
    }
}

fn flags_str(f: u64) -> String {
    let mut s = String::new();
    if f & SHF_WRITE != 0 { s.push('W'); }
    if f & SHF_ALLOC != 0 { s.push('A'); }
    if f & SHF_EXECINSTR != 0 { s.push('X'); }
    if f & SHF_INFO_LINK != 0 { s.push('I'); }
    s
}

/// Parse an ELF64 object back and print it readelf-style.
fn dump_elf64(data: &[u8]) {
    assert_eq!(&data[0..4], &ELF_MAGIC);
    println!("class={} data={} version={} type={} machine={}", data[4], data[5], data[6], read_u16(data, 16), read_u16(data, 18));
    let shoff = read_u64(data, 40) as usize;
    let shentsize = read_u16(data, 58) as usize;
    let shnum = read_u16(data, 60) as usize;
    let shstrndx = read_u16(data, 62) as usize;
    println!("shoff={:#x} shentsize={} shnum={} shstrndx={}", shoff, shentsize, shnum, shstrndx);
    let sh = |i: usize| shoff + i * shentsize;
    let shstr_off = read_u64(data, sh(shstrndx) + 24) as usize;
    let mut symtab = None;
    let mut strtab_off = 0;
    let mut names = Vec::new();
    for i in 0..shnum {
        let b = sh(i);
        let name = read_cstr(data, shstr_off + read_u32(data, b) as usize);
        let ty = read_u32(data, b + 4);
        let flags = read_u64(data, b + 8);
        let off = read_u64(data, b + 24);
        let size = read_u64(data, b + 32);
        let link = read_u32(data, b + 40);
        let info = read_u32(data, b + 44);
        let align = read_u64(data, b + 48);
        let entsize = read_u64(data, b + 56);
        println!("  [{:2}] {:<12} {:<8} {:>3} off={:06x} size={:06x} link={} info={} align={} es={}", i, name, sh_type_name(ty), flags_str(flags), off, size, link, info, align, entsize);
        if ty == SHT_SYMTAB {
            symtab = Some((off as usize, size as usize));
            strtab_off = read_u64(data, sh(link as usize) + 24) as usize;
        }
        names.push(name);
    }
    let (so, ss) = symtab.expect("symtab");
    let mut syms = Vec::new();
    for k in 0..ss / 24 {
        let b = so + k * 24;
        let name = read_cstr(data, strtab_off + read_u32(data, b) as usize);
        let info = data[b + 4];
        let shndx = read_u16(data, b + 6);
        let value = read_u64(data, b + 8);
        let size = read_u64(data, b + 16);
        let bind = match info >> 4 { STB_LOCAL => "LOCAL", STB_GLOBAL => "GLOBAL", STB_WEAK => "WEAK", _ => "?" };
        let ty = match info & 0xf { STT_NOTYPE => "NOTYPE", STT_OBJECT => "OBJECT", STT_FUNC => "FUNC", STT_SECTION => "SECTION", _ => "?" };
        let ndx = if shndx == 0 { "UND".to_string() } else { names[shndx as usize].clone() };
        println!("  sym {:2}: {:016x} {:4} {:<7} {:<6} {:<6} {}", k, value, size, ty, bind, ndx, name);
        syms.push(name);
    }
    for i in 0..shnum {
        let b = sh(i);
        if read_u32(data, b + 4) != SHT_RELA {
            continue;
        }
        let off = read_u64(data, b + 24) as usize;
        let size = read_u64(data, b + 32) as usize;
        println!("  relocations in {}:", names[i]);
        for k in 0..size / 24 {
            let r = off + k * 24;
            let info = read_u64(data, r + 8);
            let sym = (info >> 32) as usize;
            let ty = match info as u32 { R_X86_64_64 => "R_X86_64_64", R_X86_64_PC32 => "R_X86_64_PC32", R_X86_64_PLT32 => "R_X86_64_PLT32", _ => "?" };
            println!("    {:012x} {:<15} {} {:+}", read_u64(data, r), ty, if syms[sym].is_empty() { "<section>" } else { &syms[sym] }, read_i64(data, r + 16));
        }
    }
}

fn checksum(data: &[u8]) -> u32 {
    // simple FNV-1a so a single wrong byte anywhere shows up
    let mut h: u32 = 0x811c9dc5;
    for &b in data {
        h ^= b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    h
}

fn main() {
    // x86-64 code for:  main: push rbp; mov rbp,rsp; lea rdi,[rip+msg]; call puts; mov eax,[rip+counter]; pop rbp; ret
    let text: Vec<u8> = vec![
        0x55, 0x48, 0x89, 0xe5, 0x48, 0x8d, 0x3d, 0, 0, 0, 0, 0xe8, 0, 0, 0, 0, 0x8b, 0x05, 0, 0, 0, 0, 0x5d, 0xc3,
        // helper: xor eax,eax; ret
        0x31, 0xc0, 0xc3,
    ];
    let mut data_sec = b"Hello, ELF!\0".to_vec();
    data_sec.extend_from_slice(&0x1122334455667788u64.to_le_bytes());
    data_sec.extend_from_slice(&(-2i32).to_le_bytes());
    let sections = vec![
        Section {
            name: ".text",
            sh_type: SHT_PROGBITS,
            flags: SHF_ALLOC | SHF_EXECINSTR,
            data: text,
            nobits_size: 0,
            align: 16,
            relocs: vec![
                Reloc { offset: 7, symbol: "msg", r_type: R_X86_64_PC32, addend: -4 },
                Reloc { offset: 12, symbol: "puts", r_type: R_X86_64_PLT32, addend: -4 },
                Reloc { offset: 18, symbol: "counter", r_type: R_X86_64_PC32, addend: -4 },
            ],
        },
        Section {
            name: ".data",
            sh_type: SHT_PROGBITS,
            flags: SHF_ALLOC | SHF_WRITE,
            data: data_sec,
            nobits_size: 0,
            align: 8,
            relocs: vec![],
        },
        Section {
            name: ".bss",
            sh_type: SHT_NOBITS,
            flags: SHF_ALLOC | SHF_WRITE,
            data: vec![],
            nobits_size: 4096,
            align: 32,
            relocs: vec![],
        },
        Section {
            name: ".data.rel",
            sh_type: SHT_PROGBITS,
            flags: SHF_ALLOC | SHF_WRITE,
            data: vec![0; 16],
            nobits_size: 0,
            align: 8,
            relocs: vec![
                Reloc { offset: 0, symbol: "main", r_type: R_X86_64_64, addend: 0 },
                Reloc { offset: 8, symbol: "msg", r_type: R_X86_64_64, addend: 6 },
            ],
        },
    ];
    let symbols = vec![
        Symbol { name: "main", section: Some(0), value: 0, size: 24, binding: STB_GLOBAL, sym_type: STT_FUNC },
        Symbol { name: "helper", section: Some(0), value: 24, size: 3, binding: STB_LOCAL, sym_type: STT_FUNC },
        Symbol { name: "msg", section: Some(1), value: 0, size: 12, binding: STB_LOCAL, sym_type: STT_OBJECT },
        Symbol { name: "counter", section: Some(2), value: 0, size: 4, binding: STB_GLOBAL, sym_type: STT_OBJECT },
        Symbol { name: "puts", section: None, value: 0, size: 0, binding: STB_GLOBAL, sym_type: STT_NOTYPE },
        Symbol { name: "fallback", section: Some(0), value: 24, size: 3, binding: STB_WEAK, sym_type: STT_FUNC },
        Symbol { name: "table", section: Some(3), value: 0, size: 16, binding: STB_GLOBAL, sym_type: STT_OBJECT },
    ];
    let obj = write_elf64(&sections, &symbols, EM_X86_64);
    println!("object size {} checksum {:08x}", obj.len(), checksum(&obj));
    print!("{}", hexdump(&obj, 0, 64));
    print!("{}", hexdump(&obj, 64, 48));
    dump_elf64(&obj);

    // string table dedup behaviour
    let mut st = StringTable::new();
    let a = st.add("foo");
    let b = st.add("bar");
    let c = st.add("foo");
    let e = st.add("");
    println!("strtab foo={} bar={} foo again={} empty={} missing={} len={}", a, b, c, e, st.offset_of("nope"), st.len());
    println!("strtab bytes {:?}", st.as_bytes());

    // ELF32 symbols and REL entries
    let mut b32 = Vec::new();
    write_sym32(&mut b32, 1, 0x8048000, 42, (STB_GLOBAL << 4) | STT_FUNC, 0, 1);
    write_rel32(&mut b32, 0x10, 3, 2);
    print!("{}", hexdump(&b32, 0, b32.len()));

    // program headers for a tiny executable image
    let mut ph = vec![0u8; 56 * 2];
    wphdr(&mut ph, 0, PT_LOAD, PF_R | PF_X, 0, 0x400000, 0x1234, 0x1234, 0x1000);
    wphdr(&mut ph, 56, PT_LOAD, PF_R | PF_W, 0x2000, 0x402000, 0x100, 0x5000, 0x1000);
    for k in 0..2 {
        let o = k * 56;
        println!(
            "phdr{} type={} flags={} off={:#x} vaddr={:#x} paddr={:#x} filesz={:#x} memsz={:#x} align={:#x}",
            k, read_u32(&ph, o), read_u32(&ph, o + 4), read_u64(&ph, o + 8), read_u64(&ph, o + 16), read_u64(&ph, o + 24), read_u64(&ph, o + 32), read_u64(&ph, o + 40), read_u64(&ph, o + 48)
        );
    }
    // out-of-bounds writes are no-ops
    let mut small = [0xaau8; 6];
    w32(&mut small, 4, 0xdeadbeef);
    w16(&mut small, 4, 0xbeef);
    w64(&mut small, 0, 1);
    write_bytes(&mut small, 5, &[1, 2]);
    println!("small {:02x?}", small);
    println!("read_cstr past end {:?} / mid {:?}", read_cstr(&obj, obj.len() + 5), read_cstr(&obj, 1));
    for (m, name) in [(EM_386, "i386"), (EM_AARCH64, "aarch64"), (EM_RISCV, "riscv")] {
        let o = write_elf64(&sections[..2], &symbols[..1], m);
        println!("{} object {} bytes checksum {:08x} machine field {}", name, o.len(), checksum(&o), read_u16(&o, 18));
    }
}
