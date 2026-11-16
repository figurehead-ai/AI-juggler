# local_probe: the local tier's engine for the AJ-E3709 runs

A throwaway test harness, not product code. It is the local-model counterpart of `../apple_probe.swift`: the same protocol 3
over stdin and stdout, so `../runner_v3.py` runs a GGUF model through the same frozen harness (ADR-0033 §4.2 and §5; gate.py
frozen at 7acec4f50).

## Why the source is stored here as text, and built outside the repository

The Cargo project lives at `~/Dev/AIJuggler-models/local_probe/`, beside the models, and builds into the shared target dir
`~/Dev/AIJuggler-shared-target/r05-probe`. Only its source is kept here, as text:

- `cargo machete` (`just supply-chain`, CI) walks every `Cargo.toml` under the repository, so a nested project would be audited
  as if it were a product crate;
- CI's cache keys hash `**/Cargo.lock`, so a nested lock file would change every Rust cache key;
- GitHub's dependency graph would list llama-cpp-2 and its tree as the repository's dependencies, while the maintainer approved
  llama-cpp-2 for this test only (ADR-0033 §4.2: the product helper's dependency needs its own yes with Q1).

So `Cargo.toml.txt` and `Cargo.lock.txt` are the project's manifest and lock file renamed; `main.rs` is `src/main.rs`. The `.rs`
file sits outside `crates/` and `ui/src/`, so the repository's Rust lints and size budgets don't scan it, as intended for a harness.

## Build (offline, from the cargo cache only)

```sh
mkdir -p ~/Dev/AIJuggler-models/local_probe/src          # only if rebuilding from this copy
cp main.rs ~/Dev/AIJuggler-models/local_probe/src/main.rs
cp Cargo.toml.txt ~/Dev/AIJuggler-models/local_probe/Cargo.toml
cp Cargo.lock.txt ~/Dev/AIJuggler-models/local_probe/Cargo.lock
cd ~/Dev/AIJuggler-models/local_probe
CARGO_TARGET_DIR=~/Dev/AIJuggler-shared-target/r05-probe CMAKE_DISABLE_FIND_PACKAGE_OpenSSL=TRUE cargo build --offline --release
```

- llama-cpp-2 0.1.146 with `default-features = false, features = ["metal"]` (no OpenMP: Apple's clang has none).
- `CMAKE_DISABLE_FIND_PACKAGE_OpenSSL=TRUE`: llama.cpp's `common` library builds cpp-httplib for its model-download helpers. With
  Homebrew's OpenSSL found, that file doesn't compile; without it, no HTTPS code is built at all. The probe calls no network code.
- Needs CMake (installed 2026-11-16 with the maintainer's yes) and the Command Line Tools' clang/libclang. Nothing is fetched.

## What it does

- Loads exactly the file given by `--model`, all layers on Metal, a 4096-token context cleared before every call.
- Renders each call with the GGUF's own Jinja chat template (`tokenizer.chat_template`): `system` as the system message, `prompt`
  as the user message, `enable_thinking=false`. Both models' templates honour that flag by pre-filling an empty think block
  (Qwen3: `<think>\n\n</think>\n\n`; Nemotron 3 Nano: `<think></think>`), so the instructions reach the model byte for byte, with
  no `/no_think` added. `--show-prompt` prints the rendered form and the template, then exits.
- Greedy decoding (temperature 0). `write` (mode `line`): at most `max_tokens` (60) under ADR-0033 §1's `line_grammar()` as
  ruled on 2026-11-16: one line of at most 160 characters, no line break, no quotation mark, no whitespace but the plain space,
  and no word cap. The first build capped the grammar at 20 words, and the models then glued words together once the cap was
  reached ("MB_of_memory"). The gate's shape check enforces the 20 words, as it does for Apple's guided output, which has no word
  cap either. A request's own `grammar` field replaces it. `choose` (mode `pick`): the grammar `[1-count]`, at most 20 tokens.
- `finish` is `length` when the token limit is reached before the model ends its turn, or when the grammar forced the end at
  160 characters while the model wanted to go on (a cut line, rejected by step 4 like a token-limit cut). `refused` and `retried_permissive` are always
  false (llama.cpp has no guardrail layer; a refusal in words is the harness's non-answer check).
- The hello carries the model file's name, its SHA-256 read from `MANIFEST.md` (not re-hashed), its quantisation, the llama.cpp
  build, `line_max_chars` (runner_v3.py refuses a probe whose limit differs from gate.py's) and the rendered grammar.
- `--stats FILE` appends one JSON line per call (prompt tokens, prompt and decode time, decode tokens/s, whether a think tag was
  generated) and, at exit, the peak RSS and the peak physical footprint.

## The full runs (one model at a time; the Mac is shared)

From this folder's parent (`harness/`), with `P=~/Dev/AIJuggler-shared-target/r05-probe/release/local_probe` and
`M=~/Dev/AIJuggler-models`:

```sh
python3 runner_v3.py --cmd "$P --model $M/qwen3-1.7b/Qwen3-1.7B-Q8_0.gguf --stats <scratch>/stats-qwen.jsonl" \
  --corpus v2=../corpus-v2-8f7c7474.json --corpus turnends=../corpus-v3-turnends.json \
  --corpus heldout=../heldout/corpus-heldout.json --out <scratch>/results-qwen3-1.7b-q8-v3.json
python3 runner_v3.py --cmd "$P --model $M/nemotron-3-nano-4b/NVIDIA-Nemotron3-Nano-4B-Q4_K_M.gguf --stats <scratch>/stats-nemotron.jsonl" \
  --corpus v2=../corpus-v2-8f7c7474.json --corpus turnends=../corpus-v3-turnends.json \
  --corpus heldout=../heldout/corpus-heldout.json --out <scratch>/results-nemotron-3-nano-4b-q4km-v3.json
python3 report_v3.py <results.json> --corpus v2=... --corpus turnends=... --corpus heldout=... --md <report.md> > <report.json>
```

`report_v3.py` labels a Q8 file "Q8 upper bound" (never a shipping number) and applies the local tier's hard gate: any obeyed
injection fails the model.

## Harness v4 extensions (2026-11-16; still protocol 3, backwards compatible)

The hello lists `"extensions": ["fields", "sampling"]`; `runner_v3.py` requests are unchanged and behave byte for byte as before.

- `mode: "fields"` generates a job's structured fields under the request's own `grammar` (required; `../v4/jobs.py` builds it from
  the job's schema). The grammar closes the object, so an end it forces is a normal `stop`; `length` means the token limit only.
- `sampling: {temperature, top_p, top_k, min_p, presence_penalty, seed}`. Absent, or temperature 0, is greedy (v3's path).
  Otherwise the grammar masks the vocabulary and llama.cpp's penalties (last 64 tokens), top_k, top_p, min_p, temperature and
  dist samplers pick, in llama.cpp common's order. The response echoes the sampling used.
- `--print-meta`: every GGUF metadata key and value as JSON (token tables summarised by size), from a vocabulary-only load: no
  weights, no context, no generation. Neither file carries generation defaults (2026-11-16; `../v4/gguf-meta.json`).
- `--check-grammar <file>`: parses each grammar in a JSON-lines file `{"name", "grammar"}` with llama.cpp's own parser, from a
  vocabulary-only load. `../v4/runner_v4.py --check-grammars` writes that file.
