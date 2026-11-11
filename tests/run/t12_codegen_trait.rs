#![allow(dead_code)]
// t12: a backend codegen trait in the style of ccc src/backend/traits.rs:
// a trait with required "primitive" methods and many default methods that
// express the shared algorithm, a shared CodegenState, two backends (x86-64
// and AArch64 flavoured), the delegate_to_impl! macro lifted verbatim from
// ccc, and drivers that call through both `&mut dyn ArchCodegen` and a
// generic `C: ArchCodegen`.
use std::collections::HashMap;
use std::fmt::Write;

// ---- lifted from ccc src/backend/traits.rs ------------------------------------
macro_rules! delegate_to_impl {
    // Entry point: parse one delegation at a time and recurse
    () => {};

    // &mut self method with return type
    (fn $name:ident(&mut self $(, $pname:ident : $pty:ty)*) -> $ret:ty => $impl_name:ident; $($rest:tt)*) => {
        fn $name(&mut self $(, $pname : $pty)*) -> $ret { self.$impl_name($($pname),*) }
        delegate_to_impl!{ $($rest)* }
    };

    // &mut self method without return type
    (fn $name:ident(&mut self $(, $pname:ident : $pty:ty)*) => $impl_name:ident; $($rest:tt)*) => {
        fn $name(&mut self $(, $pname : $pty)*) { self.$impl_name($($pname),*) }
        delegate_to_impl!{ $($rest)* }
    };

    // &self method with return type
    (fn $name:ident(&self $(, $pname:ident : $pty:ty)*) -> $ret:ty => $impl_name:ident; $($rest:tt)*) => {
        fn $name(&self $(, $pname : $pty)*) -> $ret { self.$impl_name($($pname),*) }
        delegate_to_impl!{ $($rest)* }
    };

    // &self method without return type
    (fn $name:ident(&self $(, $pname:ident : $pty:ty)*) => $impl_name:ident; $($rest:tt)*) => {
        fn $name(&self $(, $pname : $pty)*) { self.$impl_name($($pname),*) }
        delegate_to_impl!{ $($rest)* }
    };
}

// ---- IR ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Value(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IrType {
    I8,
    I32,
    I64,
    Ptr,
}

impl IrType {
    fn size(self) -> usize {
        match self {
            IrType::I8 => 1,
            IrType::I32 => 4,
            IrType::I64 | IrType::Ptr => 8,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Operand {
    Value(Value),
    Const(i64),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum IrBinOp {
    Add,
    Sub,
    Mul,
    And,
    Shl,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum IrCmpOp {
    Eq,
    Slt,
}

#[derive(Debug, Clone)]
enum Instruction {
    Alloca { dest: Value, size: usize },
    Store { val: Operand, ptr: Value, ty: IrType },
    Load { dest: Value, ptr: Value, ty: IrType },
    BinOp { dest: Value, op: IrBinOp, lhs: Operand, rhs: Operand, ty: IrType },
    Cmp { dest: Value, op: IrCmpOp, lhs: Operand, rhs: Operand },
    Copy { dest: Value, src: Operand },
    Call { dest: Option<Value>, func: String, args: Vec<Operand> },
    GlobalAddr { dest: Value, name: String },
}

#[derive(Debug, Clone)]
enum Terminator {
    Ret(Option<Operand>),
    Br(u32),
    CondBr(Operand, u32, u32),
}

struct Block {
    id: u32,
    insts: Vec<Instruction>,
    term: Terminator,
}

struct IrFunction {
    name: String,
    params: Vec<Value>,
    blocks: Vec<Block>,
}

// ---- shared state ---------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
struct StackSlot(i64);

#[derive(Default)]
struct CodegenState {
    out: String,
    slots: HashMap<u32, StackSlot>,
    alloca_values: HashMap<u32, StackSlot>,
    frame_size: i64,
    insts_emitted: usize,
    label_counter: u32,
}

impl CodegenState {
    fn emit(&mut self, s: &str) {
        self.out.push_str("    ");
        self.out.push_str(s);
        self.out.push('\n');
        self.insts_emitted += 1;
    }
    fn label(&mut self, s: &str) {
        self.out.push_str(s);
        self.out.push_str(":\n");
    }
    fn slot_of(&self, v: Value) -> StackSlot {
        *self.slots.get(&v.0).unwrap_or_else(|| panic!("no slot for %{}", v.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PtrDirective {
    Quad,
    Xword,
}

// ---- the trait ------------------------------------------------------------------

trait ArchCodegen {
    fn state(&mut self) -> &mut CodegenState;
    fn state_ref(&self) -> &CodegenState;
    fn arch_name(&self) -> &'static str;
    fn ptr_directive(&self) -> PtrDirective;

    // primitives (1-4 lines per backend)
    fn emit_prologue(&mut self, func: &IrFunction, frame_size: i64);
    fn emit_epilogue(&mut self, frame_size: i64);
    fn emit_load_const_to_acc(&mut self, c: i64);
    fn emit_load_slot_to_acc(&mut self, slot: StackSlot, ty: IrType);
    fn emit_store_acc_to_slot(&mut self, slot: StackSlot, ty: IrType);
    fn emit_acc_to_secondary(&mut self);
    fn emit_binop_secondary_acc(&mut self, op: IrBinOp);
    fn emit_cmp_secondary_acc(&mut self, op: IrCmpOp);
    fn emit_slot_addr_to_acc(&mut self, slot: StackSlot);
    fn emit_store_acc_through_secondary(&mut self, ty: IrType);
    fn emit_load_through_acc(&mut self, ty: IrType);
    fn emit_global_addr_to_acc(&mut self, name: &str);
    fn emit_arg_from_acc(&mut self, idx: usize);
    fn emit_param_to_acc(&mut self, idx: usize);
    fn emit_call_insn(&mut self, func: &str);
    fn emit_ret_val_to_acc(&mut self);
    fn emit_jump(&mut self, label: &str);
    fn emit_branch_if_acc_zero(&mut self, label: &str);
    fn max_reg_args(&self) -> usize;

    // ---- default methods: the shared algorithm ----

    fn block_label(&self, func: &IrFunction, id: u32) -> String {
        format!(".L{}_{}", func.name, id)
    }

    fn calculate_stack_space(&mut self, func: &IrFunction) -> i64 {
        let mut off = 0i64;
        let mut assign = |v: u32, size: usize, st: &mut CodegenState| {
            off += ((size as i64) + 7) & !7;
            st.slots.insert(v, StackSlot(-off));
        };
        for p in &func.params {
            assign(p.0, 8, self.state());
        }
        for b in &func.blocks {
            for inst in &b.insts {
                match inst {
                    Instruction::Alloca { dest, size } => {
                        assign(dest.0 + 10_000, *size, self.state());
                        let s = self.state().slots[&(dest.0 + 10_000)];
                        self.state().alloca_values.insert(dest.0, s);
                        assign(dest.0, 8, self.state());
                    }
                    other => {
                        if let Some(d) = inst_dest(other) {
                            assign(d.0, 8, self.state());
                        }
                    }
                }
            }
        }
        (off + 15) & !15
    }

    fn emit_load_operand(&mut self, op: &Operand) {
        match op {
            Operand::Const(c) => self.emit_load_const_to_acc(*c),
            Operand::Value(v) => {
                let s = self.state_ref().slot_of(*v);
                self.emit_load_slot_to_acc(s, IrType::I64);
            }
        }
    }

    fn emit_store_result(&mut self, dest: &Value) {
        let s = self.state_ref().slot_of(*dest);
        self.emit_store_acc_to_slot(s, IrType::I64);
    }

    fn emit_copy_value(&mut self, dest: &Value, src: &Operand) {
        self.emit_load_operand(src);
        self.emit_store_result(dest);
    }

    fn emit_binop(&mut self, dest: &Value, op: IrBinOp, lhs: &Operand, rhs: &Operand) {
        // strength-reduce multiply by power of two
        if let (IrBinOp::Mul, Operand::Const(c)) = (op, rhs) {
            if *c > 0 && (*c & (*c - 1)) == 0 {
                let shift = c.trailing_zeros() as i64;
                return self.emit_binop(dest, IrBinOp::Shl, lhs, &Operand::Const(shift));
            }
        }
        self.emit_load_operand(lhs);
        self.emit_acc_to_secondary();
        self.emit_load_operand(rhs);
        self.emit_binop_secondary_acc(op);
        self.emit_store_result(dest);
    }

    fn emit_cmp(&mut self, dest: &Value, op: IrCmpOp, lhs: &Operand, rhs: &Operand) {
        self.emit_load_operand(lhs);
        self.emit_acc_to_secondary();
        self.emit_load_operand(rhs);
        self.emit_cmp_secondary_acc(op);
        self.emit_store_result(dest);
    }

    fn emit_store(&mut self, val: &Operand, ptr: &Value, ty: IrType) {
        if let Some(slot) = self.state_ref().alloca_values.get(&ptr.0).copied() {
            // direct store into the alloca's frame slot
            self.emit_load_operand(val);
            self.emit_store_acc_to_slot(slot, ty);
            return;
        }
        let p = self.state_ref().slot_of(*ptr);
        self.emit_load_slot_to_acc(p, IrType::Ptr);
        self.emit_acc_to_secondary();
        self.emit_load_operand(val);
        self.emit_store_acc_through_secondary(ty);
    }

    fn emit_load(&mut self, dest: &Value, ptr: &Value, ty: IrType) {
        if let Some(slot) = self.state_ref().alloca_values.get(&ptr.0).copied() {
            self.emit_load_slot_to_acc(slot, ty);
        } else {
            let p = self.state_ref().slot_of(*ptr);
            self.emit_load_slot_to_acc(p, IrType::Ptr);
            self.emit_load_through_acc(ty);
        }
        self.emit_store_result(dest);
    }

    fn emit_call(&mut self, dest: &Option<Value>, func: &str, args: &[Operand]) {
        if args.len() > self.max_reg_args() {
            let msg = format!("; warning: {} args exceed {} register args", args.len(), self.max_reg_args());
            self.state().emit(&msg);
        }
        for (i, a) in args.iter().enumerate().take(self.max_reg_args()) {
            self.emit_load_operand(a);
            self.emit_arg_from_acc(i);
        }
        self.emit_call_insn(func);
        if let Some(d) = dest {
            self.emit_ret_val_to_acc();
            self.emit_store_result(d);
        }
    }

    fn emit_instruction(&mut self, inst: &Instruction) {
        match inst {
            Instruction::Alloca { dest, .. } => {
                let s = self.state_ref().alloca_values[&dest.0];
                self.emit_slot_addr_to_acc(s);
                self.emit_store_result(dest);
            }
            Instruction::Store { val, ptr, ty } => self.emit_store(val, ptr, *ty),
            Instruction::Load { dest, ptr, ty } => self.emit_load(dest, ptr, *ty),
            Instruction::BinOp { dest, op, lhs, rhs, .. } => self.emit_binop(dest, *op, lhs, rhs),
            Instruction::Cmp { dest, op, lhs, rhs } => self.emit_cmp(dest, *op, lhs, rhs),
            Instruction::Copy { dest, src } => self.emit_copy_value(dest, src),
            Instruction::Call { dest, func, args } => self.emit_call(dest, func, args),
            Instruction::GlobalAddr { dest, name } => {
                self.emit_global_addr_to_acc(name);
                self.emit_store_result(dest);
            }
        }
    }

    fn emit_terminator(&mut self, func: &IrFunction, term: &Terminator, frame: i64) {
        match term {
            Terminator::Ret(v) => {
                if let Some(v) = v {
                    self.emit_load_operand(v);
                }
                self.emit_epilogue(frame);
            }
            Terminator::Br(t) => {
                let l = self.block_label(func, *t);
                self.emit_jump(&l);
            }
            Terminator::CondBr(c, t, f) => {
                self.emit_load_operand(c);
                let lf = self.block_label(func, *f);
                let lt = self.block_label(func, *t);
                self.emit_branch_if_acc_zero(&lf);
                self.emit_jump(&lt);
            }
        }
    }

    fn emit_store_params(&mut self, func: &IrFunction) {
        for (i, p) in func.params.iter().enumerate() {
            self.emit_param_to_acc(i);
            self.emit_store_result(p);
        }
    }

    fn generate_function(&mut self, func: &IrFunction) {
        self.state().slots.clear();
        self.state().alloca_values.clear();
        let frame = self.calculate_stack_space(func);
        self.state().frame_size = frame;
        let header = format!("\n    .globl {0}\n    .type {0}, @function\n{0}:", func.name);
        self.state().out.push_str(&header[1..]);
        self.state().out.push('\n');
        self.emit_prologue(func, frame);
        self.emit_store_params(func);
        for b in &func.blocks {
            let l = self.block_label(func, b.id);
            self.state().label(&l);
            for inst in &b.insts {
                self.emit_instruction(inst);
            }
            self.emit_terminator(func, &b.term, frame);
        }
        let size = format!("    .size {0}, .-{0}\n", func.name);
        self.state().out.push_str(&size);
    }

    fn emit_global_ptr_table(&mut self, name: &str, targets: &[&str]) {
        let dir = match self.ptr_directive() {
            PtrDirective::Quad => ".quad",
            PtrDirective::Xword => ".xword",
        };
        let mut s = format!("{}:\n", name);
        for t in targets {
            writeln!(s, "    {} {}", dir, t).unwrap();
        }
        self.state().out.push_str(&s);
    }
}

fn inst_dest(i: &Instruction) -> Option<Value> {
    match i {
        Instruction::Alloca { dest, .. }
        | Instruction::Load { dest, .. }
        | Instruction::BinOp { dest, .. }
        | Instruction::Cmp { dest, .. }
        | Instruction::Copy { dest, .. }
        | Instruction::GlobalAddr { dest, .. } => Some(*dest),
        Instruction::Call { dest, .. } => *dest,
        Instruction::Store { .. } => None,
    }
}

// ---- x86-64 backend -----------------------------------------------------------------

struct X86Codegen {
    state: CodegenState,
}

const X86_ARG_REGS: [&str; 6] = ["rdi", "rsi", "rdx", "rcx", "r8", "r9"];

impl X86Codegen {
    fn suffix(ty: IrType) -> (&'static str, &'static str) {
        match ty {
            IrType::I8 => ("b", "al"),
            IrType::I32 => ("l", "eax"),
            IrType::I64 | IrType::Ptr => ("q", "rax"),
        }
    }
    fn emit_prologue_impl(&mut self, _func: &IrFunction, frame_size: i64) {
        self.state.emit("pushq %rbp");
        self.state.emit("movq %rsp, %rbp");
        if frame_size > 0 {
            self.state.emit(&format!("subq ${}, %rsp", frame_size));
        }
    }
    fn emit_epilogue_impl(&mut self, _frame_size: i64) {
        self.state.emit("movq %rbp, %rsp");
        self.state.emit("popq %rbp");
        self.state.emit("ret");
    }
    fn emit_load_const_to_acc_impl(&mut self, c: i64) {
        if c == 0 {
            self.state.emit("xorl %eax, %eax");
        } else if c >= i32::MIN as i64 && c <= i32::MAX as i64 {
            self.state.emit(&format!("movq ${}, %rax", c));
        } else {
            self.state.emit(&format!("movabsq ${}, %rax", c));
        }
    }
    fn emit_load_slot_to_acc_impl(&mut self, slot: StackSlot, ty: IrType) {
        let ins = match ty {
            IrType::I8 => format!("movsbq {}(%rbp), %rax", slot.0),
            IrType::I32 => format!("movslq {}(%rbp), %rax", slot.0),
            _ => format!("movq {}(%rbp), %rax", slot.0),
        };
        self.state.emit(&ins);
    }
    fn emit_store_acc_to_slot_impl(&mut self, slot: StackSlot, ty: IrType) {
        let (sfx, reg) = Self::suffix(ty);
        self.state.emit(&format!("mov{} %{}, {}(%rbp)", sfx, reg, slot.0));
    }
}

impl ArchCodegen for X86Codegen {
    fn state(&mut self) -> &mut CodegenState {
        &mut self.state
    }
    fn state_ref(&self) -> &CodegenState {
        &self.state
    }
    fn arch_name(&self) -> &'static str {
        "x86_64"
    }
    fn ptr_directive(&self) -> PtrDirective {
        PtrDirective::Quad
    }
    delegate_to_impl! {
        fn emit_prologue(&mut self, func: &IrFunction, frame_size: i64) => emit_prologue_impl;
        fn emit_epilogue(&mut self, frame_size: i64) => emit_epilogue_impl;
        fn emit_load_const_to_acc(&mut self, c: i64) => emit_load_const_to_acc_impl;
        fn emit_load_slot_to_acc(&mut self, slot: StackSlot, ty: IrType) => emit_load_slot_to_acc_impl;
        fn emit_store_acc_to_slot(&mut self, slot: StackSlot, ty: IrType) => emit_store_acc_to_slot_impl;
    }
    fn emit_acc_to_secondary(&mut self) {
        self.state.emit("movq %rax, %rcx");
    }
    fn emit_binop_secondary_acc(&mut self, op: IrBinOp) {
        // computes rcx op rax -> rax
        match op {
            IrBinOp::Add => self.state.emit("addq %rcx, %rax"),
            IrBinOp::Mul => self.state.emit("imulq %rcx, %rax"),
            IrBinOp::And => self.state.emit("andq %rcx, %rax"),
            IrBinOp::Sub => {
                self.state.emit("subq %rax, %rcx");
                self.state.emit("movq %rcx, %rax");
            }
            IrBinOp::Shl => {
                self.state.emit("xchgq %rax, %rcx");
                self.state.emit("shlq %cl, %rax");
            }
        }
    }
    fn emit_cmp_secondary_acc(&mut self, op: IrCmpOp) {
        self.state.emit("cmpq %rax, %rcx");
        let cc = match op {
            IrCmpOp::Eq => "e",
            IrCmpOp::Slt => "l",
        };
        self.state.emit(&format!("set{} %al", cc));
        self.state.emit("movzbq %al, %rax");
    }
    fn emit_slot_addr_to_acc(&mut self, slot: StackSlot) {
        self.state.emit(&format!("leaq {}(%rbp), %rax", slot.0));
    }
    fn emit_store_acc_through_secondary(&mut self, ty: IrType) {
        let (sfx, reg) = Self::suffix(ty);
        self.state.emit(&format!("mov{} %{}, (%rcx)", sfx, reg));
    }
    fn emit_load_through_acc(&mut self, ty: IrType) {
        let ins = match ty {
            IrType::I8 => "movsbq (%rax), %rax",
            IrType::I32 => "movslq (%rax), %rax",
            _ => "movq (%rax), %rax",
        };
        self.state.emit(ins);
    }
    fn emit_global_addr_to_acc(&mut self, name: &str) {
        self.state.emit(&format!("leaq {}(%rip), %rax", name));
    }
    fn emit_arg_from_acc(&mut self, idx: usize) {
        self.state.emit(&format!("movq %rax, %{}", X86_ARG_REGS[idx]));
    }
    fn emit_param_to_acc(&mut self, idx: usize) {
        self.state.emit(&format!("movq %{}, %rax", X86_ARG_REGS[idx]));
    }
    fn emit_call_insn(&mut self, func: &str) {
        self.state.emit(&format!("call {}@PLT", func));
    }
    fn emit_ret_val_to_acc(&mut self) {}
    fn emit_jump(&mut self, label: &str) {
        self.state.emit(&format!("jmp {}", label));
    }
    fn emit_branch_if_acc_zero(&mut self, label: &str) {
        self.state.emit("testq %rax, %rax");
        self.state.emit(&format!("je {}", label));
    }
    fn max_reg_args(&self) -> usize {
        X86_ARG_REGS.len()
    }
}

// ---- AArch64 backend ------------------------------------------------------------------

struct ArmCodegen {
    state: CodegenState,
    literal_pool: Vec<i64>,
}

impl ArmCodegen {
    fn width(ty: IrType) -> (&'static str, &'static str) {
        match ty {
            IrType::I8 => ("strb", "w0"),
            IrType::I32 => ("str", "w0"),
            _ => ("str", "x0"),
        }
    }
}

impl ArchCodegen for ArmCodegen {
    fn state(&mut self) -> &mut CodegenState {
        &mut self.state
    }
    fn state_ref(&self) -> &CodegenState {
        &self.state
    }
    fn arch_name(&self) -> &'static str {
        "aarch64"
    }
    fn ptr_directive(&self) -> PtrDirective {
        PtrDirective::Xword
    }
    // override a default method: AArch64 labels are local with .LBB
    fn block_label(&self, func: &IrFunction, id: u32) -> String {
        format!(".LBB_{}_{}", func.name, id)
    }
    fn emit_prologue(&mut self, _func: &IrFunction, frame_size: i64) {
        self.state.emit(&format!("stp x29, x30, [sp, #-{}]!", frame_size + 16));
        self.state.emit("mov x29, sp");
    }
    fn emit_epilogue(&mut self, frame_size: i64) {
        self.state.emit(&format!("ldp x29, x30, [sp], #{}", frame_size + 16));
        self.state.emit("ret");
    }
    fn emit_load_const_to_acc(&mut self, c: i64) {
        if (0..65536).contains(&c) {
            self.state.emit(&format!("mov x0, #{}", c));
        } else {
            // movz/movk sequence
            let u = c as u64;
            self.state.emit(&format!("movz x0, #{}", u & 0xffff));
            for shift in [16, 32, 48] {
                let part = (u >> shift) & 0xffff;
                if part != 0 {
                    self.state.emit(&format!("movk x0, #{}, lsl #{}", part, shift));
                }
            }
            self.literal_pool.push(c);
        }
    }
    fn emit_load_slot_to_acc(&mut self, slot: StackSlot, ty: IrType) {
        let ins = match ty {
            IrType::I8 => "ldrsb x0",
            IrType::I32 => "ldrsw x0",
            _ => "ldr x0",
        };
        self.state.emit(&format!("{}, [x29, #{}]", ins, slot.0 + self.state.frame_size + 16));
    }
    fn emit_store_acc_to_slot(&mut self, slot: StackSlot, ty: IrType) {
        let (ins, reg) = Self::width(ty);
        self.state.emit(&format!("{} {}, [x29, #{}]", ins, reg, slot.0 + self.state.frame_size + 16));
    }
    fn emit_acc_to_secondary(&mut self) {
        self.state.emit("mov x1, x0");
    }
    fn emit_binop_secondary_acc(&mut self, op: IrBinOp) {
        let m = match op {
            IrBinOp::Add => "add",
            IrBinOp::Sub => "sub",
            IrBinOp::Mul => "mul",
            IrBinOp::And => "and",
            IrBinOp::Shl => "lsl",
        };
        self.state.emit(&format!("{} x0, x1, x0", m));
    }
    fn emit_cmp_secondary_acc(&mut self, op: IrCmpOp) {
        self.state.emit("cmp x1, x0");
        let cc = match op {
            IrCmpOp::Eq => "eq",
            IrCmpOp::Slt => "lt",
        };
        self.state.emit(&format!("cset x0, {}", cc));
    }
    fn emit_slot_addr_to_acc(&mut self, slot: StackSlot) {
        self.state.emit(&format!("add x0, x29, #{}", slot.0 + self.state.frame_size + 16));
    }
    fn emit_store_acc_through_secondary(&mut self, ty: IrType) {
        let (ins, reg) = Self::width(ty);
        self.state.emit(&format!("{} {}, [x1]", ins, reg));
    }
    fn emit_load_through_acc(&mut self, ty: IrType) {
        let ins = match ty {
            IrType::I8 => "ldrsb x0, [x0]",
            IrType::I32 => "ldrsw x0, [x0]",
            _ => "ldr x0, [x0]",
        };
        self.state.emit(ins);
    }
    fn emit_global_addr_to_acc(&mut self, name: &str) {
        self.state.emit(&format!("adrp x0, {}", name));
        self.state.emit(&format!("add x0, x0, :lo12:{}", name));
    }
    fn emit_arg_from_acc(&mut self, idx: usize) {
        if idx != 0 {
            self.state.emit(&format!("mov x{}, x0", idx));
        }
    }
    fn emit_param_to_acc(&mut self, idx: usize) {
        if idx != 0 {
            self.state.emit(&format!("mov x0, x{}", idx));
        }
    }
    fn emit_call(&mut self, dest: &Option<Value>, func: &str, args: &[Operand]) {
        // override: evaluate args in reverse so x0 is written last
        for (i, a) in args.iter().enumerate().rev() {
            self.emit_load_operand(a);
            if i != 0 {
                self.state.emit(&format!("mov x{}, x0", i));
            }
        }
        self.emit_call_insn(func);
        if let Some(d) = dest {
            self.emit_store_result(d);
        }
    }
    fn emit_call_insn(&mut self, func: &str) {
        self.state.emit(&format!("bl {}", func));
    }
    fn emit_ret_val_to_acc(&mut self) {}
    fn emit_jump(&mut self, label: &str) {
        self.state.emit(&format!("b {}", label));
    }
    fn emit_branch_if_acc_zero(&mut self, label: &str) {
        self.state.emit(&format!("cbz x0, {}", label));
    }
    fn max_reg_args(&self) -> usize {
        8
    }
}

// ---- drivers -------------------------------------------------------------------------

fn generate_dyn(cg: &mut dyn ArchCodegen, funcs: &[IrFunction]) -> usize {
    for f in funcs {
        cg.generate_function(f);
    }
    cg.emit_global_ptr_table("fn_table", &["sum3", "store_load"]);
    cg.state_ref().insts_emitted
}

fn generate_generic<C: ArchCodegen>(cg: &mut C, f: &IrFunction) -> (String, i64) {
    cg.generate_function(f);
    let fs = cg.state_ref().frame_size;
    (std::mem::take(&mut cg.state().out), fs)
}

fn sample_functions() -> Vec<IrFunction> {
    let v = Value;
    let sum3 = IrFunction {
        name: "sum3".into(),
        params: vec![v(0), v(1), v(2)],
        blocks: vec![Block {
            id: 0,
            insts: vec![
                Instruction::BinOp { dest: v(3), op: IrBinOp::Add, lhs: Operand::Value(v(0)), rhs: Operand::Value(v(1)), ty: IrType::I64 },
                Instruction::BinOp { dest: v(4), op: IrBinOp::Add, lhs: Operand::Value(v(3)), rhs: Operand::Value(v(2)), ty: IrType::I64 },
                Instruction::BinOp { dest: v(5), op: IrBinOp::Mul, lhs: Operand::Value(v(4)), rhs: Operand::Const(8), ty: IrType::I64 },
                Instruction::BinOp { dest: v(6), op: IrBinOp::Mul, lhs: Operand::Value(v(5)), rhs: Operand::Const(3), ty: IrType::I64 },
            ],
            term: Terminator::Ret(Some(Operand::Value(v(6)))),
        }],
    };
    let store_load = IrFunction {
        name: "store_load".into(),
        params: vec![v(0)],
        blocks: vec![
            Block {
                id: 0,
                insts: vec![
                    Instruction::Alloca { dest: v(1), size: 4 },
                    Instruction::Store { val: Operand::Value(v(0)), ptr: v(1), ty: IrType::I32 },
                    Instruction::Load { dest: v(2), ptr: v(1), ty: IrType::I32 },
                    Instruction::Cmp { dest: v(3), op: IrCmpOp::Slt, lhs: Operand::Value(v(2)), rhs: Operand::Const(0) },
                ],
                term: Terminator::CondBr(Operand::Value(v(3)), 1, 2),
            },
            Block {
                id: 1,
                insts: vec![
                    Instruction::GlobalAddr { dest: v(4), name: "counter".into() },
                    Instruction::Store { val: Operand::Const(-1), ptr: v(4), ty: IrType::I8 },
                    Instruction::Call { dest: Some(v(5)), func: "abs".into(), args: vec![Operand::Value(v(2))] },
                    Instruction::Copy { dest: v(6), src: Operand::Value(v(5)) },
                ],
                term: Terminator::Br(2),
            },
            Block {
                id: 2,
                insts: vec![
                    Instruction::Load { dest: v(7), ptr: v(1), ty: IrType::I32 },
                    Instruction::Copy { dest: v(8), src: Operand::Const(0x1234_5678_9abc) },
                    Instruction::BinOp { dest: v(9), op: IrBinOp::Sub, lhs: Operand::Value(v(8)), rhs: Operand::Value(v(7)), ty: IrType::I64 },
                    Instruction::BinOp { dest: v(10), op: IrBinOp::And, lhs: Operand::Value(v(9)), rhs: Operand::Const(0xff), ty: IrType::I64 },
                    Instruction::Call {
                        dest: None,
                        func: "many".into(),
                        args: (0..9).map(Operand::Const).collect(),
                    },
                ],
                term: Terminator::Ret(Some(Operand::Value(v(10)))),
            },
        ],
    };
    vec![sum3, store_load]
}

fn main() {
    let funcs = sample_functions();
    let mut x86 = X86Codegen { state: CodegenState::default() };
    let mut arm = ArmCodegen { state: CodegenState::default(), literal_pool: Vec::new() };
    {
        let backends: Vec<&mut dyn ArchCodegen> = vec![&mut x86, &mut arm];
        for cg in backends {
            let n = generate_dyn(cg, &funcs);
            println!("##### {} ({} instructions)", cg.arch_name(), n);
            print!("{}", cg.state_ref().out);
            let mut slots: Vec<(u32, i64)> = cg.state_ref().slots.iter().map(|(k, s)| (*k, s.0)).collect();
            slots.sort();
            println!("##### last frame size {} slots {:?}", cg.state_ref().frame_size, slots);
        }
    }
    println!("arm literal pool: {:?}", arm.literal_pool);
    let mut x2 = X86Codegen { state: CodegenState::default() };
    let (text, fs) = generate_generic(&mut x2, &funcs[0]);
    println!("generic x86 sum3: {} lines, frame {}, out now empty: {}", text.lines().count(), fs, x2.state.out.is_empty());
    let mut a2 = ArmCodegen { state: CodegenState::default(), literal_pool: vec![] };
    let (text, fs) = generate_generic(&mut a2, &funcs[1]);
    let branches: Vec<&str> = text.lines().filter(|l| l.trim_start().starts_with('b') || l.contains("cbz")).collect();
    println!("generic arm store_load: frame {}, branches {:?}", fs, branches);
}
