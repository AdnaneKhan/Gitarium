//! Code-level passes that mutate function bodies. These bloat the binary and
//! MUST be the last build step — a subsequent `wasm-opt` would fold them away
//! (`a ^ (a^n) → n`, `i32.const k; call_indirect → call f`).

use std::collections::HashMap;

use rand::Rng;
use walrus::ir::{BinaryOp, Binop, CallIndirect, Const, Instr, InstrSeqId, Value};
use walrus::{
    ConstExpr, ElementItems, ElementKind, FunctionId, FunctionKind, LocalFunction, Module, RefType,
    TypeId,
};

/// Every `InstrSeqId` reachable in a local function (entry + nested
/// block/loop/if-else bodies).
fn seq_ids(lf: &LocalFunction) -> Vec<InstrSeqId> {
    let mut out = vec![lf.entry_block()];
    let mut i = 0;
    while i < out.len() {
        let id = out[i];
        i += 1;
        for (instr, _) in lf.block(id).instrs.iter() {
            match instr {
                Instr::Block(b) => out.push(b.seq),
                Instr::Loop(l) => out.push(l.seq),
                Instr::IfElse(ie) => {
                    out.push(ie.consequent);
                    out.push(ie.alternative);
                }
                _ => {}
            }
        }
    }
    out
}

/// Replace each `i32.const N` / `i64.const N` in every function body with an
/// equivalent `const A; const (A^N); xor` triple (stack-neutral), hiding all
/// literal magic numbers / offsets. Returns how many constants were encoded.
pub fn obfuscate_constants(module: &mut Module) -> usize {
    let mut rng = rand::thread_rng();
    let fids: Vec<_> = module.funcs.iter().map(|f| f.id()).collect();
    let mut count = 0usize;
    for fid in fids {
        let lf = match &mut module.funcs.get_mut(fid).kind {
            FunctionKind::Local(lf) => lf,
            _ => continue,
        };
        for sid in seq_ids(lf) {
            let instrs = &mut lf.block_mut(sid).instrs;
            let mut out = Vec::with_capacity(instrs.len());
            for (instr, loc) in instrs.drain(..) {
                match instr {
                    Instr::Const(Const { value: Value::I32(n) }) => {
                        let a: i32 = rng.gen();
                        out.push((Instr::Const(Const { value: Value::I32(a) }), loc));
                        out.push((Instr::Const(Const { value: Value::I32(a ^ n) }), loc));
                        out.push((Instr::Binop(Binop { op: BinaryOp::I32Xor }), loc));
                        count += 1;
                    }
                    Instr::Const(Const { value: Value::I64(n) }) => {
                        let a: i64 = rng.gen();
                        out.push((Instr::Const(Const { value: Value::I64(a) }), loc));
                        out.push((Instr::Const(Const { value: Value::I64(a ^ n) }), loc));
                        out.push((Instr::Binop(Binop { op: BinaryOp::I64Xor }), loc));
                        count += 1;
                    }
                    other => out.push((other, loc)),
                }
            }
            *instrs = out;
        }
    }
    count
}

/// Hide the call graph: route every direct call to a *local* function through
/// a fresh funcref table, rewriting `call F` into `i32.const slot;
/// call_indirect (F's type)`. The index lands after the already-pushed args, so
/// it's a stack-correct local replacement. (Run before `obfuscate_constants`
/// so the slot literals get encoded too.) Returns how many calls were aliased.
pub fn alias_calls(module: &mut Module) -> usize {
    let locals: Vec<FunctionId> = module
        .funcs
        .iter()
        .filter(|f| matches!(f.kind, FunctionKind::Local(_)))
        .map(|f| f.id())
        .collect();
    if locals.is_empty() {
        return 0;
    }
    let slot: HashMap<FunctionId, i32> =
        locals.iter().enumerate().map(|(i, &f)| (f, i as i32)).collect();
    let tyof: HashMap<FunctionId, TypeId> =
        locals.iter().map(|&f| (f, module.funcs.get(f).ty())).collect();

    // A dedicated table holding exactly these functions at their slot indices.
    let n = locals.len() as u64;
    let table = module.tables.add_local(false, n, Some(n), RefType::Funcref);
    module.elements.add(
        ElementKind::Active { table, offset: ConstExpr::Value(Value::I32(0)) },
        ElementItems::Functions(locals.clone()),
    );

    let fids: Vec<_> = module.funcs.iter().map(|f| f.id()).collect();
    let mut count = 0usize;
    for fid in fids {
        let lf = match &mut module.funcs.get_mut(fid).kind {
            FunctionKind::Local(lf) => lf,
            _ => continue,
        };
        for sid in seq_ids(lf) {
            let instrs = &mut lf.block_mut(sid).instrs;
            let mut out = Vec::with_capacity(instrs.len());
            for (instr, loc) in instrs.drain(..) {
                match instr {
                    Instr::Call(call) if slot.contains_key(&call.func) => {
                        out.push((Instr::Const(Const { value: Value::I32(slot[&call.func]) }), loc));
                        out.push((Instr::CallIndirect(CallIndirect { ty: tyof[&call.func], table }), loc));
                        count += 1;
                    }
                    other => out.push((other, loc)),
                }
            }
            *instrs = out;
        }
    }
    count
}
