//! jq — full filter language via jaq.

use super::exec::input;

pub(super) fn jq_cmd(args: &[String], stdin: &str) -> Result<String, String> {
    let mut raw = false;
    let mut filter: Option<String> = None;
    let mut files = Vec::new();
    for a in args {
        match a.as_str() {
            "-r" => raw = true,
            "-c" | "-C" | "-M" => {} // output is always compact/uncolored
            _ if filter.is_none() => filter = Some(a.clone()),
            _ => files.push(a.clone()),
        }
    }
    let filter = filter.ok_or("jq: missing filter")?;
    let text = input(&files, stdin)?;
    let mut out = String::new();
    for val in jq_eval(&filter, &text)? {
        match &val {
            jaq_json::Val::TStr(b) if raw => out.push_str(&String::from_utf8_lossy(b)),
            v => out.push_str(&v.to_string()),
        }
        out.push('\n');
    }
    Ok(out)
}

fn jq_eval(filter_src: &str, input_text: &str) -> Result<Vec<jaq_json::Val>, String> {
    use jaq_core::load::{Arena, File, Loader};
    use jaq_core::{data, unwrap_valr, Compiler, Ctx, Vars};
    use jaq_json::Val;

    let input = jaq_json::read::parse_single(input_text.trim().as_bytes())
        .map_err(|e| format!("jq: input is not JSON: {}", e))?;
    let program = File { code: filter_src, path: () };
    let loader = Loader::new(jaq_core::defs().chain(jaq_std::defs()).chain(jaq_json::defs()));
    let arena = Arena::default();
    let modules = loader.load(&arena, program).map_err(|errs| {
        let msgs: Vec<String> = errs.iter().map(|(_, e)| format!("{:?}", e)).collect();
        format!("jq: parse error in '{}': {}", filter_src, msgs.join("; "))
    })?;
    let filter = Compiler::default()
        .with_funs(jaq_core::funs().chain(jaq_std::funs()).chain(jaq_json::funs()))
        .compile(modules)
        .map_err(|errs| {
            let msgs: Vec<String> = errs.iter().map(|(_, e)| format!("{:?}", e)).collect();
            format!("jq: compile error: {}", msgs.join("; "))
        })?;
    let ctx = Ctx::<data::JustLut<Val>>::new(&filter.lut, Vars::new([]));
    let mut out = Vec::new();
    for v in filter.id.run((ctx, input)).map(unwrap_valr) {
        out.push(v.map_err(|e| format!("jq: {}", e))?);
    }
    Ok(out)
}
