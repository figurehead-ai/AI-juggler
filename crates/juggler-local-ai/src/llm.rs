//! The model, through llama-cpp-2's safe API: load one file, render each
//! request with the file's own chat template (thinking off, ADR-0035
//! §11.1), and generate under the request's guide and sampling, checking
//! the cancel flag and the deadline between tokens. Driven the way the
//! AJ-E3709 harness's local probe measured it (greedy fast path,
//! llama.cpp common's sampler order).

use std::num::NonZeroU32;
use std::path::Path;
use std::time::{Duration, Instant};

use juggler_local_ai_protocol::{
    Answer, Fault, Finish, Guide, MAX_TEXT_BYTES, Request, Sampling, is_model_file_name,
};
use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaChatTemplate, LlamaModel};
use llama_cpp_2::openai::OpenAIChatTemplateParams;
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::token::LlamaToken;
use llama_cpp_2::token::data::LlamaTokenData;
use llama_cpp_2::token::data_array::LlamaTokenDataArray;

use crate::{Shared, Wait};

/// Every layer on the GPU where there is one (Metal on Apple silicon).
const GPU_LAYERS: u32 = 999;

/// The prompt is decoded in batches of this many tokens.
const MICRO_BATCH: u32 = 512;

/// The seed of the sampled path: fixed, so a run can be repeated.
const SEED: u32 = 0;

/// The presence penalty looks back this many tokens (llama.cpp's default).
const PENALTY_WINDOW: i32 = 64;

/// llama.cpp's backend, its logs off (stderr is unused).
pub fn backend() -> Option<LlamaBackend> {
    let mut backend = LlamaBackend::init().ok()?;
    backend.void_logs();
    Some(backend)
}

/// Serves loads and generations until the process exits.
pub fn serve(backend: &LlamaBackend, dir: &Path, wait: &Wait, shared: &Shared) -> ! {
    let mut next = None;
    loop {
        let request = next.take().unwrap_or_else(|| wait.next());
        match request {
            Request::Load {
                id,
                file,
                size_bytes,
                context_tokens,
            } => {
                shared.set_running(0);
                let model = match open(backend, dir, &file, size_bytes) {
                    Ok(model) => model,
                    Err(error) => {
                        fail(shared, id, error);
                        continue;
                    }
                };
                let session = match Session::new(backend, &model, context_tokens) {
                    Ok(session) => session,
                    Err(error) => {
                        fail(shared, id, error);
                        continue;
                    }
                };
                shared.send(&Answer::Loaded {
                    id,
                    context_tokens: session.n_ctx,
                });
                shared.done();
                next = Some(session.serve(wait, shared));
            }
            Request::Generate { id, .. } => fail(shared, id, Fault::NotLoaded),
            Request::Cancel { .. } | Request::Quit {} => shared.done(),
        }
    }
}

fn fail(shared: &Shared, id: u64, error: Fault) {
    shared.send(&Answer::Error {
        id: Some(id),
        error,
    });
    shared.done();
}

/// Opens `file` in `dir`: a model file's name only, a plain file (never a
/// link), of exactly `size_bytes`; then llama.cpp loads it.
fn open(
    backend: &LlamaBackend,
    dir: &Path,
    file: &str,
    size_bytes: u64,
) -> Result<LlamaModel, Fault> {
    if !is_model_file_name(file) {
        return Err(Fault::BadRequest);
    }
    let path = dir.join(file);
    let meta = std::fs::symlink_metadata(&path).map_err(|_| Fault::NoFile)?;
    if !meta.file_type().is_file() {
        return Err(Fault::NoFile);
    }
    if meta.len() != size_bytes {
        return Err(Fault::WrongFile);
    }
    std::fs::File::open(&path).map_err(|_| Fault::NoFile)?;
    let params = LlamaModelParams::default().with_n_gpu_layers(GPU_LAYERS);
    LlamaModel::load_from_file(backend, &path, &params).map_err(|_| Fault::LoadFailed)
}

/// A loaded model's context and chat template.
struct Session<'m> {
    model: &'m LlamaModel,
    ctx: LlamaContext<'m>,
    template: LlamaChatTemplate,
    n_ctx: u32,
}

/// One generation's text and how it ended.
struct Output {
    text: String,
    finish: Finish,
    tokens: u32,
}

impl<'m> Session<'m> {
    fn new(
        backend: &LlamaBackend,
        model: &'m LlamaModel,
        context_tokens: u32,
    ) -> Result<Self, Fault> {
        let template = model.chat_template(None).map_err(|_| Fault::NoTemplate)?;
        let n_ctx = NonZeroU32::new(context_tokens).ok_or(Fault::BadRequest)?;
        let params = LlamaContextParams::default()
            .with_n_ctx(Some(n_ctx))
            .with_n_batch(context_tokens)
            .with_n_ubatch(MICRO_BATCH.min(context_tokens));
        let ctx = model
            .new_context(backend, params)
            .map_err(|_| Fault::LoadFailed)?;
        Ok(Self {
            model,
            ctx,
            template,
            n_ctx: context_tokens,
        })
    }

    /// Serves generations with this model; gives back the next load.
    fn serve(mut self, wait: &Wait, shared: &Shared) -> Request {
        loop {
            match wait.next() {
                load @ Request::Load { .. } => return load,
                Request::Generate {
                    id,
                    instructions,
                    prompt,
                    guide,
                    sampling,
                    max_tokens,
                    timeout_ms,
                } => {
                    shared.set_running(id);
                    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
                    let answer = match self.generate(
                        &instructions,
                        &prompt,
                        guide.as_ref(),
                        sampling,
                        max_tokens,
                        &|| shared.cancelled() || Instant::now() >= deadline,
                    ) {
                        Ok(output) => Answer::Text {
                            id,
                            text: output.text,
                            finish: output.finish,
                            tokens: output.tokens,
                        },
                        Err(Fault::Stopped) => Answer::Error {
                            id: Some(id),
                            error: if shared.cancelled() {
                                Fault::Stopped
                            } else {
                                Fault::TimedOut
                            },
                        },
                        Err(error) => Answer::Error {
                            id: Some(id),
                            error,
                        },
                    };
                    shared.send(&answer);
                    shared.done();
                }
                Request::Cancel { .. } | Request::Quit {} => shared.done(),
            }
        }
    }

    /// The file's own chat template, thinking off: `enable_thinking=false`
    /// as the flag and as a template argument (ADR-0035 §11.1); a template
    /// without that switch ignores it.
    fn render(&self, instructions: &str, prompt: &str) -> Result<String, Fault> {
        let messages = serde_json::json!([
            { "role": "system", "content": instructions },
            { "role": "user", "content": prompt },
        ])
        .to_string();
        let params = OpenAIChatTemplateParams {
            messages_json: &messages,
            tools_json: None,
            tool_choice: None,
            json_schema: None,
            grammar: None,
            reasoning_format: Some("none"),
            chat_template_kwargs: Some(r#"{"enable_thinking": false}"#),
            add_generation_prompt: true,
            use_jinja: true,
            parallel_tool_calls: false,
            enable_thinking: false,
            add_bos: false,
            add_eos: false,
            parse_tool_calls: false,
        };
        self.model
            .apply_chat_template_oaicompat(&self.template, &params)
            .map(|rendered| rendered.prompt)
            .map_err(|_| Fault::NoTemplate)
    }

    /// The guide as a grammar sampler, and whether an end it forces is a
    /// cut answer (a raw grammar's) or the natural end (a JSON schema's).
    fn guide(&self, guide: Option<&Guide>) -> Result<Option<(LlamaSampler, bool)>, Fault> {
        let (grammar, forced_end_is_cut) = match guide {
            None => return Ok(None),
            Some(Guide::Grammar(grammar)) => (grammar.clone(), true),
            Some(Guide::JsonSchema(schema)) => (
                llama_cpp_2::json_schema_to_grammar(schema).map_err(|_| Fault::Guide)?,
                false,
            ),
        };
        let sampler =
            LlamaSampler::grammar(self.model, &grammar, "root").map_err(|_| Fault::Guide)?;
        Ok(Some((sampler, forced_end_is_cut)))
    }

    fn generate(
        &mut self,
        instructions: &str,
        prompt: &str,
        guide: Option<&Guide>,
        sampling: Sampling,
        max_tokens: u32,
        stop: &dyn Fn() -> bool,
    ) -> Result<Output, Fault> {
        let rendered = self.render(instructions, prompt)?;
        let tokens = self
            .model
            .str_to_token(&rendered, AddBos::Always)
            .map_err(|_| Fault::Failed)?;
        let needed = u64::try_from(tokens.len()).unwrap_or(u64::MAX) + u64::from(max_tokens);
        if tokens.is_empty() || needed > u64::from(self.n_ctx) {
            return Err(Fault::TooLong);
        }
        let mut grammar = self.guide(guide)?;
        let mut chain = sampler_chain(sampling, self.model.n_vocab());
        if stop() {
            return Err(Fault::Stopped);
        }
        self.ctx.clear_kv_cache();
        let last = i32::try_from(tokens.len() - 1).map_err(|_| Fault::TooLong)?;
        let mut batch = LlamaBatch::new(tokens.len(), 1);
        for (position, token) in (0_i32..).zip(&tokens) {
            batch
                .add(*token, position, &[0], position == last)
                .map_err(|_| Fault::Failed)?;
        }
        self.ctx.decode(&mut batch).map_err(|_| Fault::Failed)?;
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let mut text = String::new();
        let mut count = 0_u32;
        let mut position = last + 1;
        let mut next_batch = LlamaBatch::new(1, 1);
        let mut logits_at = last;
        let finish = loop {
            if stop() {
                return Err(Fault::Stopped);
            }
            if count >= max_tokens {
                break Finish::Length;
            }
            let logits = self.ctx.get_logits_ith(logits_at);
            let held = grammar.as_mut().map(|(sampler, _)| sampler);
            let (token, forced) = match chain.as_mut() {
                Some(chain) => sample_under(logits, held, chain),
                None => greedy_under(logits, held),
            };
            if self.model.is_eog_token(token) {
                let cut = forced && grammar.as_ref().is_some_and(|(_, cut)| *cut);
                break if cut { Finish::Length } else { Finish::Stop };
            }
            let piece = self
                .model
                .token_to_piece(token, &mut decoder, false, None)
                .map_err(|_| Fault::Failed)?;
            if text.len() + piece.len() > MAX_TEXT_BYTES {
                break Finish::Length;
            }
            text.push_str(&piece);
            count += 1;
            next_batch.clear();
            next_batch
                .add(token, position, &[0], true)
                .map_err(|_| Fault::Failed)?;
            position += 1;
            self.ctx
                .decode(&mut next_batch)
                .map_err(|_| Fault::Failed)?;
            logits_at = 0;
        };
        Ok(Output {
            text,
            finish,
            tokens: count,
        })
    }
}

/// The sampled path's chain for `sampling` (llama.cpp common's order:
/// penalties, top-k, top-p, min-p, temperature, then the draw); `None`
/// for greedy decoding.
fn sampler_chain(sampling: Sampling, n_vocab: i32) -> Option<LlamaSampler> {
    if sampling.greedy() {
        return None;
    }
    let milli = |value: Option<u32>, default: f32| {
        value.map_or(default, |n| {
            f32::from(u16::try_from(n).unwrap_or(u16::MAX)) / 1000.0
        })
    };
    let presence = milli(sampling.presence_milli, 0.0);
    let top_k = sampling
        .top_k
        .and_then(|k| i32::try_from(k).ok())
        .filter(|k| *k > 0)
        .unwrap_or(n_vocab);
    Some(LlamaSampler::chain_simple([
        LlamaSampler::penalties(
            if presence > 0.0 { PENALTY_WINDOW } else { 0 },
            1.0,
            0.0,
            presence,
        ),
        LlamaSampler::top_k(top_k),
        LlamaSampler::top_p(milli(sampling.top_p_milli, 1.0), 1),
        LlamaSampler::min_p(milli(sampling.min_p_milli, 0.0), 1),
        LlamaSampler::temp(milli(sampling.temperature_milli, 0.0)),
        LlamaSampler::dist(SEED),
    ]))
}

/// Every token with its logit.
fn candidates(logits: &[f32]) -> LlamaTokenDataArray {
    LlamaTokenDataArray::from_iter(
        (0_i32..)
            .zip(logits)
            .map(|(id, logit)| LlamaTokenData::new(LlamaToken(id), *logit, 0.0)),
        false,
    )
}

/// The highest logit's token (the lowest id on a tie, as llama.cpp's
/// greedy sampler).
fn argmax(tokens: impl Iterator<Item = (i32, f32)>) -> i32 {
    tokens
        .fold((0, f32::NEG_INFINITY), |best, (id, logit)| {
            if logit > best.1 { (id, logit) } else { best }
        })
        .0
}

/// Greedy decoding under an optional grammar, with llama.cpp common's
/// fast path: the unconstrained best token when the grammar allows it,
/// otherwise the best of what the grammar leaves. The grammar accepts the
/// token. Gives the token and whether the grammar overrode the model.
fn greedy_under(logits: &[f32], grammar: Option<&mut LlamaSampler>) -> (LlamaToken, bool) {
    let best = argmax((0_i32..).zip(logits.iter().copied()));
    let Some(grammar) = grammar else {
        return (LlamaToken(best), false);
    };
    let best_logit = usize::try_from(best)
        .ok()
        .and_then(|i| logits.get(i))
        .copied()
        .unwrap_or(f32::NEG_INFINITY);
    let mut one = LlamaTokenDataArray::new(
        vec![LlamaTokenData::new(LlamaToken(best), best_logit, 0.0)],
        false,
    );
    grammar.apply(&mut one);
    let token = if one.data.first().is_some_and(|d| d.logit().is_finite()) {
        best
    } else {
        let mut all = candidates(logits);
        grammar.apply(&mut all);
        argmax(all.data.iter().map(|d| (d.id().0, d.logit())))
    };
    grammar.accept(LlamaToken(token));
    (LlamaToken(token), token != best)
}

/// Sampled decoding under an optional grammar: the grammar masks the
/// vocabulary, then the chain draws. Gives the token and whether the
/// grammar disallowed the model's own best token.
fn sample_under(
    logits: &[f32],
    mut grammar: Option<&mut LlamaSampler>,
    chain: &mut LlamaSampler,
) -> (LlamaToken, bool) {
    let best = argmax((0_i32..).zip(logits.iter().copied()));
    let mut all = candidates(logits);
    let mut best_allowed = true;
    if let Some(grammar) = grammar.as_mut() {
        grammar.apply(&mut all);
        best_allowed = all
            .data
            .iter()
            .any(|d| d.id().0 == best && d.logit().is_finite());
    }
    all.apply_sampler(chain);
    let token = all.selected_token().unwrap_or(LlamaToken(best));
    if let Some(grammar) = grammar.as_mut() {
        grammar.accept(token);
    }
    chain.accept(token);
    (token, !best_allowed)
}
