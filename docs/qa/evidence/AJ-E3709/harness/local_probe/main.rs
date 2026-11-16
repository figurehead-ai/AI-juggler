//! local_probe: AJ-E3709's local-tier engine (ADR-0033 §4.2 and §5), a drop-in for apple_probe.swift under runner_v3.py.
//! Throwaway test harness, not product code. llama.cpp through llama-cpp-2 0.1.146 with Metal; no network code at all.
//!
//!   local_probe --model <file.gguf> [--stats <file.jsonl>] [--manifest <MANIFEST.md>] [--ctx 4096]
//!   local_probe --model <file.gguf> --show-prompt      (prints the rendered chat prompt of a sample request, then exits)
//!
//! Protocol 3, as apple_probe.swift: one JSON object per stdin line, one per stdout line.
//!   request  {id, system, prompt, max_tokens, temperature, mode?, field?, field_description?, pattern?, count?, grammar?}
//!   response {id, text, finish: "stop"|"length", out_tokens, refused, retried_permissive, error, gen_ms}
//! mode "line": one line under LINE_GRAMMAR (ADR-0033 §1 `line_grammar()`: 1 to 20 words, no line break, no quotation mark;
//!   the same shape as gate.LINE_PATTERN). A request's own `grammar` replaces it (the ADR's contract: the client sends it).
//! mode "pick": a digit-only grammar, 1..=count. no mode: plain text, no grammar (v2's form).
//! Every call: `system` is the system message, `prompt` the user message, rendered with the GGUF's own (Jinja) chat template with
//! thinking off; greedy decoding (temperature 0); a 4096-token context cleared before each call; at most `max_tokens` new tokens.
//! finish is "length" when the limit was reached before the model ended its turn. refused and retried_permissive are always false:
//! llama.cpp has no guardrail layer (a refusal in words is the harness's non-answer check, as for Apple).
//! The hello carries the model file's name, its SHA-256 as recorded in MANIFEST.md (not re-hashed here), and the llama.cpp build.
//! --stats appends one JSON line per call (prompt tokens, prompt and decode time, tokens/s) and, at exit, the peak memory.
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::openai::OpenAIChatTemplateParams;
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::token::data::LlamaTokenData;
use llama_cpp_2::token::data_array::LlamaTokenDataArray;
use llama_cpp_2::token::LlamaToken;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::time::Instant;

const PROTOCOL: u32 = 3;
const LLAMA_CPP: &str = "llama-cpp-2 0.1.146 / llama-cpp-sys-2 0.1.146 (llama-cpp-rs 4afdaf0782ef, vendored llama.cpp), Metal";
/// gate.LINE_MAX_WORDS; runner_v3.py refuses a probe whose hello says otherwise.
const LINE_MAX_WORDS: u32 = 20;
/// ADR-0033 §1 `line_grammar()`, the GBNF form of gate.LINE_PATTERN `[^\s"“”„«»「」]+(?: [^\s"“”„«»「」]+){0,19}`:
/// one or more words of non-space, non-quotation-mark characters, joined by single spaces, at most LINE_MAX_WORDS words.
fn line_grammar() -> String {
    let word = r#"[^ \t\n\r\x0B\x0C\u0085   -     　"“”„«»「」]+"#;
    format!("root ::= word (\" \" word){{0,{}}}\nword ::= {word}\n", LINE_MAX_WORDS - 1)
}
/// choose: the number only, 1..=count (ADR-0033 §4.2: "a grammar that allows only a digit"; at most 8 candidates).
fn pick_grammar(count: u32) -> String {
    let n = count.clamp(1, 9);
    if n == 1 { "root ::= \"1\"\n".into() } else { format!("root ::= [1-{n}]\n") }
}

struct Args { model: PathBuf, stats: Option<PathBuf>, manifest: Option<PathBuf>, ctx: u32, show_prompt: bool }
fn args() -> Result<Args, String> {
    let mut a = Args { model: PathBuf::new(), stats: None, manifest: None, ctx: 4096, show_prompt: false };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut v = || it.next().ok_or(format!("{k} wants a value"));
        match k.as_str() {
            "--model" => a.model = PathBuf::from(v()?),
            "--stats" => a.stats = Some(PathBuf::from(v()?)),
            "--manifest" => a.manifest = Some(PathBuf::from(v()?)),
            "--ctx" => a.ctx = v()?.parse().map_err(|_| "--ctx wants a number".to_string())?,
            "--show-prompt" => a.show_prompt = true,
            _ => return Err(format!("unknown argument {k}")),
        }
    }
    if a.model.as_os_str().is_empty() { return Err("--model is required".into()); }
    Ok(a)
}

fn emit(v: &Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{v}");
    let _ = out.flush();
}

/// The file's SHA-256 from MANIFEST.md's table row `| <file name> | <bytes> | <sha256> | ... |` (default: the manifest two folders up).
fn manifest_sha(model: &Path, manifest: Option<&Path>) -> (Value, Value) {
    let name = model.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let path = manifest.map(Path::to_path_buf).or_else(|| model.parent()?.parent().map(|p| p.join("MANIFEST.md")));
    let Some(path) = path else { return (Value::Null, Value::Null) };
    let Ok(text) = std::fs::read_to_string(&path) else { return (Value::Null, json!(format!("{} not readable", path.display()))) };
    for line in text.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() > 3 && cells[1] == name && cells[3].len() == 64 && cells[3].bytes().all(|b| b.is_ascii_hexdigit()) {
            return (json!(cells[3]), json!(path.file_name().and_then(|n| n.to_str()).unwrap_or("MANIFEST.md")));
        }
    }
    (Value::Null, json!(format!("{name} not listed in {}", path.display())))
}
fn quant_of(name: &str) -> &'static str {
    let up = name.to_ascii_uppercase();
    ["Q8_0", "Q6_K", "Q5_K_M", "Q4_K_M", "Q4_0", "F16", "BF16"].into_iter().find(|q| up.contains(q)).unwrap_or("unknown")
}
/// (peak resident set, lifetime peak physical footprint) in MB. The footprint counts Metal buffers that RSS may not.
fn peak_memory_mb() -> (f64, f64) {
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    let rss = if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut ru) } == 0 { ru.ru_maxrss as f64 / 1048576.0 } else { -1.0 };
    let mut info: libc::rusage_info_v4 = unsafe { std::mem::zeroed() };
    let rc = unsafe {
        libc::proc_pid_rusage(libc::getpid(), libc::RUSAGE_INFO_V4, (&mut info as *mut libc::rusage_info_v4).cast())
    };
    let fp = if rc == 0 { info.ri_lifetime_max_phys_footprint as f64 / 1048576.0 } else { -1.0 };
    (rss, fp)
}

/// Greedy decoding (temperature 0) under an optional grammar, with llama.cpp common's fast path: the unconstrained argmax is taken
/// when the grammar allows it (then it is also the constrained argmax); otherwise the grammar masks the whole vocabulary and the
/// argmax of what is left is taken. Ties go to the lowest token id, as llama.cpp's greedy sampler does. The grammar accepts the token.
fn greedy_under(logits: &[f32], grammar: Option<&mut LlamaSampler>) -> LlamaToken {
    let argmax = |it: &mut dyn Iterator<Item = (i32, f32)>| {
        it.fold((0, f32::NEG_INFINITY), |b, (i, l)| if l > b.1 { (i, l) } else { b }).0
    };
    let best = argmax(&mut logits.iter().enumerate().map(|(i, l)| (i as i32, *l)));
    let Some(g) = grammar else { return LlamaToken(best) };
    let mut one = LlamaTokenDataArray::new(vec![LlamaTokenData::new(LlamaToken(best), logits[best as usize], 0.0)], false);
    g.apply(&mut one);
    let tok = if one.data[0].logit().is_finite() {
        best
    } else {
        let mut all = LlamaTokenDataArray::from_iter(
            logits.iter().enumerate().map(|(i, l)| LlamaTokenData::new(LlamaToken(i as i32), *l, 0.0)), false);
        g.apply(&mut all);
        argmax(&mut all.data.iter().map(|d| (d.id().0, d.logit())))
    };
    g.accept(LlamaToken(tok));
    LlamaToken(tok)
}

struct Engine<'m> {
    model: &'m LlamaModel,
    ctx: llama_cpp_2::context::LlamaContext<'m>,
    template: llama_cpp_2::model::LlamaChatTemplate,
    n_ctx: u32,
}
struct Gen { text: String, finish: &'static str, out_tokens: u32, prompt_tokens: usize, prompt_ms: f64, decode_ms: f64, think_tag: bool }

impl Engine<'_> {
    /// The GGUF's own chat template, rendered by llama.cpp's Jinja engine, thinking off (Qwen3's and Nemotron's templates both
    /// read `enable_thinking`; it is sent as the flag and as a template kwarg).
    fn render(&self, system: &str, prompt: &str) -> Result<String, String> {
        let messages = json!([{"role": "system", "content": system}, {"role": "user", "content": prompt}]).to_string();
        let p = OpenAIChatTemplateParams {
            messages_json: &messages, tools_json: None, tool_choice: None, json_schema: None, grammar: None,
            reasoning_format: Some("none"), chat_template_kwargs: Some(r#"{"enable_thinking": false}"#),
            add_generation_prompt: true, use_jinja: true, parallel_tool_calls: false, enable_thinking: false,
            add_bos: false, add_eos: false, parse_tool_calls: false,
        };
        self.model.apply_chat_template_oaicompat(&self.template, &p).map(|r| r.prompt).map_err(|e| format!("chat template: {e}"))
    }
    fn generate(&mut self, rendered: &str, grammar: Option<&str>, max_tokens: u32) -> Result<Gen, String> {
        let toks = self.model.str_to_token(rendered, AddBos::Always).map_err(|e| format!("tokenize: {e}"))?;
        if toks.len() + max_tokens as usize > self.n_ctx as usize {
            return Err(format!("TooLong: {} prompt tokens + {max_tokens} > {} context", toks.len(), self.n_ctx));
        }
        self.ctx.clear_kv_cache();
        let mut grammar = match grammar {
            Some(g) => Some(LlamaSampler::grammar(self.model, g, "root").map_err(|e| format!("grammar: {e}"))?),
            None => None,
        };
        let t0 = Instant::now();
        let mut batch = LlamaBatch::new(toks.len().max(1), 1);
        let last = toks.len() as i32 - 1;
        for (i, t) in toks.iter().enumerate() {
            batch.add(*t, i as i32, &[0], i as i32 == last).map_err(|e| format!("batch: {e}"))?;
        }
        self.ctx.decode(&mut batch).map_err(|e| format!("decode prompt: {e}"))?;
        let _ = self.ctx.get_logits_ith(last); // waits for the GPU, so prompt_ms is the prompt's real cost
        let prompt_ms = t0.elapsed().as_secs_f64() * 1000.0;
        let t1 = Instant::now();
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let (mut text, mut n, mut pos, mut finish, mut think_tag) = (String::new(), 0u32, toks.len() as i32, "length", false);
        let mut step = LlamaBatch::new(1, 1);
        while n < max_tokens {
            let tok = greedy_under(self.ctx.get_logits_ith(if n == 0 { last } else { 0 }), grammar.as_mut());
            if self.model.is_eog_token(tok) { finish = "stop"; break; }
            n += 1;
            let special = self.model.token_to_piece(tok, &mut encoding_rs::UTF_8.new_decoder(), true, None).unwrap_or_default();
            if special.contains("<think>") || special.contains("</think>") { think_tag = true; }
            text.push_str(&self.model.token_to_piece(tok, &mut decoder, false, None).map_err(|e| format!("detokenize: {e}"))?);
            if n == max_tokens { break; }
            step.clear();
            step.add(tok, pos, &[0], true).map_err(|e| format!("batch: {e}"))?;
            pos += 1;
            self.ctx.decode(&mut step).map_err(|e| format!("decode: {e}"))?;
        }
        Ok(Gen { text, finish, out_tokens: n, prompt_tokens: toks.len(), prompt_ms, decode_ms: t1.elapsed().as_secs_f64() * 1000.0, think_tag })
    }
}

fn main() {
    let a = match args() { Ok(a) => a, Err(e) => { eprintln!("local_probe: {e}"); std::process::exit(2) } };
    let name = a.model.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
    let (sha, sha_src) = manifest_sha(&a.model, a.manifest.as_deref());
    let mut hello = json!({"hello": true, "protocol": PROTOCOL, "guided": true, "engine": "local model (llama.cpp, Metal) via local_probe",
        "model_file": name, "model_sha256": sha, "sha256_source": sha_src, "quant": quant_of(&name), "llama_cpp": LLAMA_CPP,
        "n_ctx": a.ctx, "temperature": 0, "sampler": "greedy", "line_max_words": LINE_MAX_WORDS, "line_grammar": line_grammar(),
        "chat_template": "the GGUF's own (tokenizer.chat_template), Jinja", "thinking": "off: enable_thinking=false (flag and template kwarg)",
        "os": std::process::Command::new("sw_vers").arg("-productVersion").output().ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default()});
    let mut backend = match LlamaBackend::init() { Ok(b) => b, Err(e) => { hello["available"] = json!(format!("unavailable: backend {e}")); emit(&hello); return } };
    backend.void_logs();
    let mparams = LlamaModelParams::default().with_n_gpu_layers(999);
    let model = match LlamaModel::load_from_file(&backend, &a.model, &mparams) {
        Ok(m) => m,
        Err(e) => { hello["available"] = json!(format!("unavailable: model load {e}")); emit(&hello); return }
    };
    let template = match model.chat_template(None) {
        Ok(t) => t,
        Err(e) => { hello["available"] = json!(format!("unavailable: no chat template in the file ({e})")); emit(&hello); return }
    };
    let cparams = LlamaContextParams::default().with_n_ctx(NonZeroU32::new(a.ctx)).with_n_batch(a.ctx).with_n_ubatch(512);
    let ctx = match model.new_context(&backend, cparams) {
        Ok(c) => c,
        Err(e) => { hello["available"] = json!(format!("unavailable: context {e}")); emit(&hello); return }
    };
    hello["arch"] = json!(model.meta_val_str("general.architecture").unwrap_or_default());
    hello["gpu_offload"] = json!(backend.supports_gpu_offload());
    hello["system_info"] = json!(unsafe { std::ffi::CStr::from_ptr(llama_cpp_sys_2::llama_print_system_info()) }.to_string_lossy());
    let mut eng = Engine { model: &model, ctx, template, n_ctx: a.ctx };
    if a.show_prompt {
        match eng.render("SYSTEM TEXT", "USER TEXT") { Ok(p) => println!("{p}"), Err(e) => println!("ERROR {e}") }
        if let Ok(t) = eng.template.to_str() { eprintln!("---- template ----\n{t}"); }
        return;
    }
    hello["available"] = json!("available");
    emit(&hello);
    let mut stats = a.stats.as_ref().and_then(|p| std::fs::OpenOptions::new().create(true).append(true).open(p).ok());
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        let Ok(j) = serde_json::from_str::<Value>(&line) else { continue };
        let s = |k: &str| j.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let mode = j.get("mode").and_then(Value::as_str).unwrap_or("text").to_string();
        let max_tokens = j.get("max_tokens").and_then(Value::as_u64).unwrap_or(60) as u32;
        let count = j.get("count").and_then(Value::as_u64).unwrap_or(1) as u32;
        let id = j.get("id").cloned().unwrap_or(json!(""));
        let t0 = Instant::now();
        let grammar = match (mode.as_str(), j.get("grammar").and_then(Value::as_str)) {
            (_, Some(g)) => Some(g.to_string()),
            ("line", None) => Some(line_grammar()),
            ("pick", None) => Some(pick_grammar(count)),
            _ => None,
        };
        let r = eng.render(&s("system"), &s("prompt")).and_then(|p| eng.generate(&p, grammar.as_deref(), max_tokens));
        let gen_ms = t0.elapsed().as_secs_f64() * 1000.0;
        let o = match &r {
            Ok(g) => json!({"id": id, "text": g.text, "finish": g.finish, "out_tokens": g.out_tokens, "refused": false,
                            "retried_permissive": false, "error": "", "gen_ms": gen_ms}),
            Err(e) => json!({"id": id, "text": "", "finish": "stop", "out_tokens": Value::Null, "refused": false,
                             "retried_permissive": false, "error": e, "gen_ms": gen_ms}),
        };
        emit(&o);
        if let (Some(f), Ok(g)) = (stats.as_mut(), &r) {
            let tps = if g.decode_ms > 0.0 { g.out_tokens as f64 * 1000.0 / g.decode_ms } else { 0.0 };
            let _ = writeln!(f, "{}", json!({"mode": mode, "prompt_tokens": g.prompt_tokens, "out_tokens": g.out_tokens, "finish": g.finish,
                "prompt_ms": g.prompt_ms, "decode_ms": g.decode_ms, "decode_tokens_per_s": tps,
                "prompt_tokens_per_s": g.prompt_tokens as f64 * 1000.0 / g.prompt_ms.max(0.001), "think_tag_generated": g.think_tag}));
        }
    }
    if let Some(f) = stats.as_mut() {
        let (rss, fp) = peak_memory_mb();
        let _ = writeln!(f, "{}", json!({"exit": true, "model_file": name, "peak_rss_mb": rss, "peak_phys_footprint_mb": fp}));
    }
}
