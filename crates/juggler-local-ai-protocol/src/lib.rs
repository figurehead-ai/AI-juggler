//! The local-model helper's wire protocol (ADR-0033 §4.2, ADR-0035 §11,
//! D-131): what AI Juggler and `juggler-local-ai` say to each other, and
//! every bound on it, in one place both sides compile.
//!
//! **The channel.** The app starts the helper with one argument pair,
//! [`MODELS_DIR_ARG`] and the models' folder (an absolute path), and no
//! shell. Then newline-delimited JSON, one object per line: requests on
//! the helper's stdin, answers on its stdout. The helper's first line is
//! its [`Answer::Hello`]. One request runs at a time; only
//! [`Request::Cancel`] may arrive while a generation runs.
//!
//! **The requests:**
//! - [`Request::Load`]: one model file, named only by its SHA-256
//!   ([`model_file_name`]), in the folder given at start. The app verified
//!   its hash first; the helper checks the name, that it is a plain file
//!   (no link) and its exact size, and opens nothing else.
//! - [`Request::Generate`]: the instructions (the system message), the
//!   prompt (the user message), an optional [`Guide`] (a GBNF grammar or
//!   a JSON schema the sampler is held to), the [`Sampling`] within the
//!   compiled [`bounds`], the most output tokens and a timeout.
//! - [`Request::Cancel`]: stop that generation between two tokens.
//! - [`Request::Quit`]: exit now. Closing stdin does the same.
//!
//! **Strict both ways:** a line longer than its bound
//! ([`MAX_REQUEST_BYTES`], [`MAX_ANSWER_BYTES`]) is refused before it is
//! parsed; unknown fields, duplicate fields, a missing field, a number out
//! of its bound, a NUL in any text, or a file name that isn't
//! `<64 lower-case hex>.gguf` are refused ([`Fault::BadRequest`]); the
//! error answers are one closed list ([`Fault`]) and never carry text.
//!
//! Must not know: any model's name, any path but the shape of a model's
//! file name, any job.

#![cfg_attr(test, allow(clippy::unwrap_used, reason = "tests fail loudly"))]

use serde::{Deserialize, Serialize};

/// The protocol version both sides speak; the hello carries it.
pub const PROTOCOL: u32 = 3;

/// The helper's file name, in the app's bundle beside AI Juggler.
pub const PROGRAM: &str = "juggler-local-ai";

/// The identifier the helper is signed as (its embedded Info.plist); the
/// app runs no helper signed as anything else.
pub const IDENTIFIER: &str = "ai.figurehead.juggler.local-ai";

/// The one argument the helper takes, followed by the models' folder.
pub const MODELS_DIR_ARG: &str = "--models-dir";

/// The helper's sandbox (SBPL), applied by `/usr/bin/sandbox-exec` at
/// every start, with three parameters: `HELPER` (the helper's own path),
/// `MODELS` (the models' folder) and `CACHE` (Metal's shader cache for the
/// helper's identifier). Everything is denied but what the helper was
/// shown by test to need (ADR-0033 §4.2 "As built"). One source for the
/// app, which applies it, and the helper's tests, which probe it.
pub const SANDBOX_PROFILE: &str = include_str!("../sandbox.sb");

/// The largest request line, its newline excluded (ADR-0033 §4: 32 KiB).
pub const MAX_REQUEST_BYTES: usize = 32 * 1024;

/// The largest answer line, its newline excluded.
pub const MAX_ANSWER_BYTES: usize = 64 * 1024;

/// The largest grammar or JSON schema a request may carry.
pub const MAX_GUIDE_BYTES: usize = 8 * 1024;

/// The most text an answer carries: generation ends with
/// [`Finish::Length`] when it is reached. Escaped as JSON at worst six
/// bytes a byte, it still fits [`MAX_ANSWER_BYTES`].
pub const MAX_TEXT_BYTES: usize = 10 * 1024;

/// Set (to `1`) only by the helper's own tests that run it without its
/// sandbox: then it serves although its start-up check finds itself
/// unsandboxed. The app never sets it (it starts the helper with a cleared
/// environment), and runs no helper whose hello says it isn't sandboxed.
pub const UNSANDBOXED_FOR_TESTS: &str = "AIJ_LOCAL_AI_UNSANDBOXED_FOR_TESTS";

/// The helper exits by itself after this long with nothing to do
/// (ADR-0033 §4.2: 120 s), releasing the model's memory.
pub const IDLE_EXIT_SECS: u64 = 120;

/// The compiled bounds every number in a request must keep. The `models`
/// section's sampling fields (ADR-0035 §11.1) are read with the same
/// bounds, so a value the section accepts is one the helper accepts.
pub mod bounds {
    /// An inclusive range of whole numbers.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Range {
        /// The least.
        pub min: u64,
        /// The most.
        pub max: u64,
    }

    impl Range {
        /// Whether `n` is within it.
        #[must_use]
        pub const fn holds(self, n: u64) -> bool {
            n >= self.min && n <= self.max
        }
    }

    /// Temperature × 1000 (0 is greedy).
    pub const TEMPERATURE_MILLI: Range = Range { min: 0, max: 2_000 };
    /// Top-p × 1000.
    pub const TOP_P_MILLI: Range = Range { min: 1, max: 1_000 };
    /// Top-k (0 is off).
    pub const TOP_K: Range = Range { min: 0, max: 200 };
    /// Min-p × 1000.
    pub const MIN_P_MILLI: Range = Range { min: 0, max: 1_000 };
    /// Presence penalty × 1000.
    pub const PRESENCE_MILLI: Range = Range { min: 0, max: 2_000 };
    /// The context a model is loaded with, in tokens.
    pub const CONTEXT_TOKENS: Range = Range {
        min: 1_024,
        max: 32_768,
    };
    /// The most output tokens one generation may ask for.
    pub const MAX_TOKENS: Range = Range { min: 1, max: 4_096 };
    /// One generation's timeout, in milliseconds.
    pub const TIMEOUT_MS: Range = Range {
        min: 100,
        max: 600_000,
    };
    /// A model file's size (ADR-0035 §11.1: at most 8 GiB).
    pub const SIZE_BYTES: Range = Range {
        min: 1,
        max: 8 * 1024 * 1024 * 1024,
    };
}

/// The sampling one generation uses; a field left out is the engine's
/// default (greedy: no temperature).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sampling {
    /// Temperature × 1000; absent or 0 is greedy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature_milli: Option<u32>,
    /// Top-p × 1000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p_milli: Option<u32>,
    /// Top-k.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    /// Min-p × 1000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_p_milli: Option<u32>,
    /// Presence penalty × 1000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presence_milli: Option<u32>,
}

impl Sampling {
    /// Whether every field is within its bound.
    #[must_use]
    pub fn in_bounds(&self) -> bool {
        let within = |value: Option<u32>, range: bounds::Range| {
            value.is_none_or(|n| range.holds(u64::from(n)))
        };
        within(self.temperature_milli, bounds::TEMPERATURE_MILLI)
            && within(self.top_p_milli, bounds::TOP_P_MILLI)
            && within(self.top_k, bounds::TOP_K)
            && within(self.min_p_milli, bounds::MIN_P_MILLI)
            && within(self.presence_milli, bounds::PRESENCE_MILLI)
    }

    /// Whether it decodes greedily (no temperature, or 0).
    #[must_use]
    pub fn greedy(&self) -> bool {
        self.temperature_milli.is_none_or(|t| t == 0)
    }
}

/// What holds the output's shape while it is generated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum Guide {
    /// A GBNF grammar whose start rule is `root` (llama.cpp's grammar
    /// sampler). An end the grammar forces while the model would go on is
    /// a cut answer ([`Finish::Length`]).
    Grammar(String),
    /// A JSON schema, turned into a grammar by llama.cpp's own converter.
    /// The grammar closing the object is the answer's natural end.
    JsonSchema(String),
}

impl Guide {
    fn text(&self) -> &str {
        match self {
            Self::Grammar(text) | Self::JsonSchema(text) => text,
        }
    }
}

/// One request line, app to helper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    /// Load this model (replacing any loaded one).
    Load {
        /// The request's id, echoed by its answer.
        id: u64,
        /// The file's name in the models' folder: [`model_file_name`].
        file: String,
        /// Its exact size; any other size is refused.
        size_bytes: u64,
        /// The disk (device) the app hashed it on: the helper opens the
        /// file without following a link and loads it only when the file
        /// it opened is this one.
        device: u64,
        /// Its inode on that disk.
        inode: u64,
        /// The context to load it with, in tokens.
        context_tokens: u32,
    },
    /// Generate one answer with the loaded model.
    Generate {
        /// The request's id, echoed by its answer.
        id: u64,
        /// The system message.
        instructions: String,
        /// The user message.
        prompt: String,
        /// What holds the output's shape; none for plain text.
        #[serde(default)]
        guide: Option<Guide>,
        /// The sampling.
        #[serde(default)]
        sampling: Sampling,
        /// The most output tokens.
        max_tokens: u32,
        /// The time it may take, in milliseconds.
        timeout_ms: u64,
    },
    /// Stop the running generation with this id.
    Cancel {
        /// The generation's id.
        id: u64,
    },
    /// Exit now (a struct variant, so an unknown field is refused).
    Quit {},
}

impl Request {
    /// Its id (`None` for a quit).
    #[must_use]
    pub const fn id(&self) -> Option<u64> {
        match self {
            Self::Load { id, .. } | Self::Generate { id, .. } | Self::Cancel { id } => Some(*id),
            Self::Quit {} => None,
        }
    }

    /// Whether every field keeps its bound: the file name's shape, each
    /// number's range, the guide's size, and no NUL in any text.
    ///
    /// # Errors
    ///
    /// [`Fault::BadRequest`] naming nothing (the answer carries no text).
    pub fn check(&self) -> Result<(), Fault> {
        let ok = match self {
            Self::Load {
                file,
                size_bytes,
                context_tokens,
                ..
            } => {
                is_model_file_name(file)
                    && bounds::SIZE_BYTES.holds(*size_bytes)
                    && bounds::CONTEXT_TOKENS.holds(u64::from(*context_tokens))
            }
            Self::Generate {
                instructions,
                prompt,
                guide,
                sampling,
                max_tokens,
                timeout_ms,
                ..
            } => {
                !instructions.contains('\0')
                    && !prompt.contains('\0')
                    && guide.as_ref().is_none_or(|guide| {
                        !guide.text().is_empty()
                            && guide.text().len() <= MAX_GUIDE_BYTES
                            && !guide.text().contains('\0')
                    })
                    && sampling.in_bounds()
                    && bounds::MAX_TOKENS.holds(u64::from(*max_tokens))
                    && bounds::TIMEOUT_MS.holds(*timeout_ms)
            }
            Self::Cancel { .. } | Self::Quit {} => true,
        };
        if ok { Ok(()) } else { Err(Fault::BadRequest) }
    }

    /// The request as one line with its newline.
    ///
    /// # Errors
    ///
    /// [`Fault::BadRequest`] when it breaks a bound ([`Request::check`]);
    /// [`Fault::TooLarge`] when the line is over [`MAX_REQUEST_BYTES`].
    pub fn encode(&self) -> Result<Vec<u8>, Fault> {
        self.check()?;
        line(self, MAX_REQUEST_BYTES)
    }
}

/// How a generation ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Finish {
    /// The model ended its answer (or a JSON schema's grammar closed it).
    Stop,
    /// It was cut: the token limit, [`MAX_TEXT_BYTES`], or a grammar's
    /// forced end while the model would go on.
    Length,
}

/// Why a request gave no result: a closed list, never text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fault {
    /// Not a request this protocol knows, or a field out of its bound.
    BadRequest,
    /// The line was over its bound.
    TooLarge,
    /// A generation is running; one request at a time.
    Busy,
    /// No model is loaded.
    NotLoaded,
    /// The file isn't there, isn't a plain file, or can't be opened.
    NoFile,
    /// The file's size isn't the one asked for.
    WrongFile,
    /// llama.cpp couldn't load it (or make its context).
    LoadFailed,
    /// The file has no chat template.
    NoTemplate,
    /// The prompt and the output don't fit the context.
    TooLong,
    /// The grammar or the JSON schema didn't parse.
    Guide,
    /// Cancelled.
    Stopped,
    /// Past its timeout.
    TimedOut,
    /// Anything else (a decode error).
    Failed,
}

/// One answer line, helper to app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Answer {
    /// The first line: the protocol and whether the GPU is used.
    Hello {
        /// [`PROTOCOL`].
        protocol: u32,
        /// llama.cpp offloads to the GPU (Metal).
        gpu: bool,
        /// The helper's own start-up check found itself sandboxed: what the
        /// profile denies (`/etc/hosts`, the models' folder's parent) it
        /// couldn't read. The app runs no helper that says otherwise (the
        /// 2026-11-19 re-review's N3: never fail open).
        sandboxed: bool,
    },
    /// The model is loaded.
    Loaded {
        /// The load's id.
        id: u64,
        /// The context it was loaded with.
        context_tokens: u32,
    },
    /// A generation's text.
    Text {
        /// The generation's id.
        id: u64,
        /// The text: at most [`MAX_TEXT_BYTES`].
        text: String,
        /// How it ended.
        finish: Finish,
        /// How many tokens it generated.
        tokens: u32,
    },
    /// A request failed (`id` is `None` when the line couldn't be read).
    Error {
        /// The request's id, when it was read.
        id: Option<u64>,
        /// Why.
        error: Fault,
    },
}

impl Answer {
    /// The answer as one line with its newline.
    ///
    /// # Errors
    ///
    /// [`Fault::TooLarge`] when the line is over [`MAX_ANSWER_BYTES`].
    pub fn encode(&self) -> Result<Vec<u8>, Fault> {
        line(self, MAX_ANSWER_BYTES)
    }
}

fn line(value: &impl Serialize, max: usize) -> Result<Vec<u8>, Fault> {
    let mut bytes = serde_json::to_vec(value).map_err(|_| Fault::Failed)?;
    if bytes.len() > max {
        return Err(Fault::TooLarge);
    }
    bytes.push(b'\n');
    Ok(bytes)
}

/// One request line (its newline removed), read strictly.
///
/// # Errors
///
/// [`Fault::TooLarge`] over [`MAX_REQUEST_BYTES`]; [`Fault::BadRequest`]
/// for anything that isn't a request within its bounds.
pub fn parse_request(line: &[u8]) -> Result<Request, Fault> {
    if line.len() > MAX_REQUEST_BYTES {
        return Err(Fault::TooLarge);
    }
    let request: Request = serde_json::from_slice(line).map_err(|_| Fault::BadRequest)?;
    request.check()?;
    Ok(request)
}

/// One answer line (its newline removed), read strictly.
///
/// # Errors
///
/// [`Fault::TooLarge`] over [`MAX_ANSWER_BYTES`], or a text over
/// [`MAX_TEXT_BYTES`]; [`Fault::BadRequest`] for anything that isn't an
/// answer.
pub fn parse_answer(line: &[u8]) -> Result<Answer, Fault> {
    if line.len() > MAX_ANSWER_BYTES {
        return Err(Fault::TooLarge);
    }
    let answer: Answer = serde_json::from_slice(line).map_err(|_| Fault::BadRequest)?;
    if let Answer::Text { text, .. } = &answer
        && text.len() > MAX_TEXT_BYTES
    {
        return Err(Fault::TooLarge);
    }
    Ok(answer)
}

/// The model file's name for `sha256` (64 lower-case hex): the only name
/// a model ever has on disk (ADR-0035 §11.8), never one from a URL.
#[must_use]
pub fn model_file_name(sha256: &str) -> Option<String> {
    is_sha256(sha256).then(|| format!("{sha256}.gguf"))
}

/// Whether `name` is a model file's name: `<64 lower-case hex>.gguf`, so
/// it can't hold a separator, a `..` or anything else.
#[must_use]
pub fn is_model_file_name(name: &str) -> bool {
    name.strip_suffix(".gguf").is_some_and(is_sha256)
}

fn is_sha256(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests;
