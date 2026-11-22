//! A tiny, synthetic GGUF model written by the tests themselves (nothing
//! is downloaded): a one-block llama-architecture network with
//! deterministic weights, a byte-fallback SentencePiece vocabulary with
//! ChatML's control tokens, and a ChatML chat template. It writes garbage,
//! but it exercises everything a real model does in the helper: llama.cpp's
//! loader, the Metal backend (every layer on the GPU), the chat template,
//! the grammar sampler, generation, cancel and timeout.

#![allow(
    dead_code,
    clippy::pedantic,
    clippy::unwrap_used,
    reason = "test code, pragmatic by policy (QUALITY.md §2); each test binary uses a part of it"
)]

use std::path::Path;

/// ChatML's start marker: a control token, id 3.
pub const IM_START: &str = "<|im_start|>";
/// Its id in the vocabulary.
pub const IM_START_ID: i32 = 3;
/// ChatML's end marker: a control token, id 4.
pub const IM_END: &str = "<|im_end|>";

const EMBD: u64 = 64;
const FF: u64 = 128;
const HEADS: u32 = 4;
const ALIGN: usize = 32;

const TEMPLATE: &str = "{% for message in messages %}<|im_start|>{{ message['role'] }}\n{{ message['content'] }}<|im_end|>\n{% endfor %}{% if add_generation_prompt %}<|im_start|>assistant\n{% endif %}";

fn string(out: &mut Vec<u8>, text: &str) {
    out.extend_from_slice(&(text.len() as u64).to_le_bytes());
    out.extend_from_slice(text.as_bytes());
}

enum Kv<'a> {
    Bool(bool),
    U32(u32),
    F32(f32),
    Str(&'a str),
    Strs(Vec<String>),
    F32s(Vec<f32>),
    I32s(Vec<i32>),
}

fn kv(out: &mut Vec<u8>, key: &str, value: &Kv<'_>) {
    string(out, key);
    match value {
        Kv::Bool(b) => {
            out.extend_from_slice(&7_u32.to_le_bytes());
            out.push(u8::from(*b));
        }
        Kv::U32(n) => {
            out.extend_from_slice(&4_u32.to_le_bytes());
            out.extend_from_slice(&n.to_le_bytes());
        }
        Kv::F32(n) => {
            out.extend_from_slice(&6_u32.to_le_bytes());
            out.extend_from_slice(&n.to_le_bytes());
        }
        Kv::Str(text) => {
            out.extend_from_slice(&8_u32.to_le_bytes());
            string(out, text);
        }
        Kv::Strs(items) => {
            out.extend_from_slice(&9_u32.to_le_bytes());
            out.extend_from_slice(&8_u32.to_le_bytes());
            out.extend_from_slice(&(items.len() as u64).to_le_bytes());
            for item in items {
                string(out, item);
            }
        }
        Kv::F32s(items) => {
            out.extend_from_slice(&9_u32.to_le_bytes());
            out.extend_from_slice(&6_u32.to_le_bytes());
            out.extend_from_slice(&(items.len() as u64).to_le_bytes());
            for item in items {
                out.extend_from_slice(&item.to_le_bytes());
            }
        }
        Kv::I32s(items) => {
            out.extend_from_slice(&9_u32.to_le_bytes());
            out.extend_from_slice(&5_u32.to_le_bytes());
            out.extend_from_slice(&(items.len() as u64).to_le_bytes());
            for item in items {
                out.extend_from_slice(&item.to_le_bytes());
            }
        }
    }
}

/// The model's bytes.
pub fn bytes() -> Vec<u8> {
    let mut tokens = vec![
        "<unk>".to_owned(),
        "<s>".to_owned(),
        "</s>".to_owned(),
        IM_START.to_owned(),
        IM_END.to_owned(),
    ];
    let mut types = vec![2, 3, 3, 3, 3];
    for b in 0..=255_u8 {
        tokens.push(format!("<0x{b:02X}>"));
        types.push(6);
    }
    let vocab = tokens.len() as u64;
    let scores = vec![0.0_f32; tokens.len()];

    let tensors: Vec<(String, Vec<u64>)> = vec![
        ("token_embd.weight".into(), vec![EMBD, vocab]),
        ("output_norm.weight".into(), vec![EMBD]),
        ("output.weight".into(), vec![EMBD, vocab]),
        ("blk.0.attn_norm.weight".into(), vec![EMBD]),
        ("blk.0.attn_q.weight".into(), vec![EMBD, EMBD]),
        ("blk.0.attn_k.weight".into(), vec![EMBD, EMBD]),
        ("blk.0.attn_v.weight".into(), vec![EMBD, EMBD]),
        ("blk.0.attn_output.weight".into(), vec![EMBD, EMBD]),
        ("blk.0.ffn_norm.weight".into(), vec![EMBD]),
        ("blk.0.ffn_gate.weight".into(), vec![EMBD, FF]),
        ("blk.0.ffn_up.weight".into(), vec![EMBD, FF]),
        ("blk.0.ffn_down.weight".into(), vec![FF, EMBD]),
    ];

    let mut out = Vec::new();
    out.extend_from_slice(b"GGUF");
    out.extend_from_slice(&3_u32.to_le_bytes());
    out.extend_from_slice(&(tensors.len() as u64).to_le_bytes());
    let kvs: Vec<(&str, Kv<'_>)> = vec![
        ("general.architecture", Kv::Str("llama")),
        ("general.name", Kv::Str("aij-test-fixture")),
        ("llama.context_length", Kv::U32(4_096)),
        ("llama.embedding_length", Kv::U32(EMBD as u32)),
        ("llama.block_count", Kv::U32(1)),
        ("llama.feed_forward_length", Kv::U32(FF as u32)),
        ("llama.attention.head_count", Kv::U32(HEADS)),
        ("llama.attention.head_count_kv", Kv::U32(HEADS)),
        ("llama.attention.layer_norm_rms_epsilon", Kv::F32(1e-5)),
        ("llama.rope.dimension_count", Kv::U32(EMBD as u32 / HEADS)),
        ("tokenizer.ggml.model", Kv::Str("llama")),
        ("tokenizer.ggml.tokens", Kv::Strs(tokens)),
        ("tokenizer.ggml.scores", Kv::F32s(scores)),
        ("tokenizer.ggml.token_type", Kv::I32s(types)),
        ("tokenizer.ggml.unknown_token_id", Kv::U32(0)),
        ("tokenizer.ggml.bos_token_id", Kv::U32(1)),
        ("tokenizer.ggml.eos_token_id", Kv::U32(2)),
        ("tokenizer.ggml.add_bos_token", Kv::Bool(false)),
        ("tokenizer.chat_template", Kv::Str(TEMPLATE)),
    ];
    out.extend_from_slice(&(kvs.len() as u64).to_le_bytes());
    for (key, value) in &kvs {
        kv(&mut out, key, value);
    }
    let mut offset = 0_u64;
    let mut sizes = Vec::new();
    for (name, dims) in &tensors {
        string(&mut out, name);
        out.extend_from_slice(&(dims.len() as u32).to_le_bytes());
        for d in dims {
            out.extend_from_slice(&d.to_le_bytes());
        }
        out.extend_from_slice(&0_u32.to_le_bytes()); // F32
        out.extend_from_slice(&offset.to_le_bytes());
        let size = dims.iter().product::<u64>() * 4;
        sizes.push(size);
        offset += size.div_ceil(ALIGN as u64) * ALIGN as u64;
    }
    while out.len() % ALIGN != 0 {
        out.push(0);
    }
    let mut seed = 0x9e37_79b9_u32;
    for ((name, _), size) in tensors.iter().zip(&sizes) {
        let count = (*size / 4) as usize;
        for _ in 0..count {
            let value = if name.ends_with("norm.weight") {
                1.0_f32
            } else {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                (f32::from(u16::try_from(seed % 2_000).unwrap()) / 1_000.0 - 1.0) * 0.08
            };
            out.extend_from_slice(&value.to_le_bytes());
        }
        while out.len() % ALIGN != 0 {
            out.push(0);
        }
    }
    out
}

/// Writes the model into `dir` under its SHA-256 name; gives the name and
/// size.
pub fn write(dir: &Path) -> (String, u64) {
    let bytes = bytes();
    let name = format!("{}.gguf", sha256_hex(&bytes));
    std::fs::write(dir.join(&name), &bytes).unwrap();
    (name, bytes.len() as u64)
}

/// A SHA-256 through the system's `shasum`, without a new dependency.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use std::io::Write as _;
    let mut child = std::process::Command::new("/usr/bin/shasum")
        .args(["-a", "256"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    let out = child.wait_with_output().unwrap();
    String::from_utf8(out.stdout).unwrap()[..64].to_owned()
}
