//! Data-section encryption.
//!
//! Every active data segment with a constant offset is XOR-scrambled in the
//! binary (so `strings`/grep see noise), and a fresh function is injected and
//! made the module `start` so it XOR-decrypts those exact memory ranges back
//! at instantiation — before any export runs. Any pre-existing `start` is
//! chained after the decrypt so module init still happens, and on plaintext.

use anyhow::{anyhow, Result};
use walrus::ir::{BinaryOp, ExtendedLoad, InstrSeqType, LoadKind, MemArg, StoreKind, Value};
use walrus::{ConstExpr, DataKind, DataId, FunctionBuilder, Module, ValType};

pub struct Report {
    pub segments: usize,
    pub bytes: usize,
    pub key: u8,
}

pub fn encrypt_data(module: &mut Module) -> Result<Report> {
    let key: u8 = rand::random::<u8>() | 1; // nonzero so it actually scrambles

    // Targets: active segments with a literal i32 offset (the only ones whose
    // destination address we can reconstruct in the decryptor).
    let mut targets: Vec<(DataId, u32, u32)> = Vec::new(); // (id, offset, len)
    for data in module.data.iter() {
        if let DataKind::Active { offset: ConstExpr::Value(Value::I32(off)), .. } = &data.kind {
            if !data.value.is_empty() {
                targets.push((data.id(), *off as u32, data.value.len() as u32));
            }
        }
    }
    if targets.is_empty() {
        return Ok(Report { segments: 0, bytes: 0, key });
    }

    // Scramble the bytes that ship in the binary.
    let mut bytes = 0usize;
    for (id, _, _) in &targets {
        for b in module.data.get_mut(*id).value.iter_mut() {
            *b ^= key;
            bytes += 1;
        }
    }

    // Build the decryptor: for each segment, a byte loop that XORs memory
    // [off, off+len) with the same key.
    let mem = module
        .memories
        .iter()
        .next()
        .map(|m| m.id())
        .ok_or_else(|| anyhow!("module has no memory to decrypt"))?;
    let i = module.locals.add(ValType::I32);
    let addr = module.locals.add(ValType::I32);
    let orig_start = module.start;

    let mut fb = FunctionBuilder::new(&mut module.types, &[], &[]);
    {
        let mut body = fb.func_body();
        for (_, off, len) in &targets {
            let (off, len, k) = (*off as i32, *len as i32, key as i32);
            body.i32_const(0).local_set(i);
            body.block(InstrSeqType::Simple(None), |done| {
                let done_id = done.id();
                done.loop_(InstrSeqType::Simple(None), |lp| {
                    let lp_id = lp.id();
                    lp.local_get(i).i32_const(len).binop(BinaryOp::I32GeU).br_if(done_id);
                    // addr = off + i
                    lp.i32_const(off).local_get(i).binop(BinaryOp::I32Add).local_set(addr);
                    // mem[addr] = load8(addr) ^ key   (store wants [addr, val])
                    lp.local_get(addr);
                    lp.local_get(addr)
                        .load(mem, LoadKind::I32_8 { kind: ExtendedLoad::ZeroExtend }, MemArg { align: 0, offset: 0 });
                    lp.i32_const(k).binop(BinaryOp::I32Xor);
                    lp.store(mem, StoreKind::I32_8 { atomic: false }, MemArg { align: 0, offset: 0 });
                    // i += 1; continue
                    lp.local_get(i).i32_const(1).binop(BinaryOp::I32Add).local_set(i);
                    lp.br(lp_id);
                });
            });
        }
        if let Some(start) = orig_start {
            body.call(start); // run the original init on now-plaintext memory
        }
    }
    let decrypt = fb.finish(vec![], &mut module.funcs);
    module.start = Some(decrypt);

    Ok(Report { segments: targets.len(), bytes, key })
}
