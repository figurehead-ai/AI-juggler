# On-device summariser: model selection (research, 2026-11-16)

Sonnet research, reviewed by the coordinating session. Web reading only; nothing was downloaded.
- **UNVERIFIED** marks a claim that couldn't be confirmed from a primary page.
- The rubric scores are judgement, not measurement.

## The job

Turn a coding agent's own turn-end text (50–2,000 tokens) into one faithful line, with no invented facts. Possibly later: a short on-device chat about the user's sessions. It's an optional, user-approved download of about 2 GB, with Apple's Foundation Model as the default.

## Rubric (weights)

| Criterion | Weight |
|---|---|
| Faithfulness | 30% |
| Licence freedom | 20% |
| Size and speed at Q4 | 15% |
| Instruction following | 10% |
| Chat | 10% |
| Ecosystem (GGUF/MLX/llama.cpp) | 10% |
| Backing | 5% |

## Top of the ranking (30 scored)

| Rank | Model | Score | Licence | Q4 size | Faithfulness evidence |
|---|---|---|---|---|---|
| 1 | Qwen3-4B (hybrid) | 89 | Apache-2.0 | ~2.5 GB (UNVERIFIED) | Vectara: 2.7% (old board), 5.7% (current) |
| 2 | Qwen3-4B-Instruct-2507 | 87 | Apache-2.0 | 2.50 GB | inferred from Qwen3-4B; non-thinking only |
| 3 | Qwen3-1.7B | 86 | Apache-2.0 (UNVERIFIED) | ~1.1 GB | 4.4% (old board) |
| 4 | Qwen3.5-4B | 82.5 | Apache-2.0 | 3.01 GB | none; larger Qwen3.5 models hallucinate about twice as often as Qwen3 |
| 5 | Granite 4.1 3B | 80.5 | Apache-2.0 | 2.10 GB | none; IFEval 82.3; built for summarisation and RAG |
| — | Gemma 4 E2B | 74 | Apache-2.0 | 3.11 GB | none |
| — | LFM3-1.7B | 69.5 | LFM Open (commercial use only under $10M revenue) | 1.1 GB | vendor IFEval 96.7 only |

## Rejected

| Model | Reason |
|---|---|
| Ministral 3 3B | 24.2% hallucination |
| Phi-4-mini | 23.5% hallucination; 420-word summaries |
| Llama 3.2 1B | 20.7% hallucination |
| EXAONE | non-commercial licence |
| Tiny Aya | non-commercial licence |
| Hunyuan | licence unread |
| Llama 3.2 3B | licence conditions |
| OpenELM | 24.8% hallucination |

## Least-sure facts

- Vectara's two boards use different scorers. Never compare rates across them.
- Most 2026 small models have **no** faithfulness measurement.
- Vectara's task is news articles. Our input is technical, full of numbers and paths, so our own test set decides.
- Apple-silicon speeds are mostly community numbers.

## Evaluation plan (abridged)

**Contenders:** Qwen3-4B-Instruct-2507, Qwen3.5-4B (thinking off), Granite 4.1 3B, Gemma 4 E2B, and one small tier (Qwen3-1.7B or Qwen3.5-2B). Apple Foundation Model is the control.

**Test set:** about 100 real, redacted turn-end messages. Mix of lengths: 25 short, 40 medium, 25 long, plus 10 adversarial. The adversarial cases include similar numbers, partial failures, "I did not run the tests", and negations. Each message has gold must-include facts.

**Automatic checks:**
- HHEM-2.1-Open, plus a second NLI scorer;
- deterministic entity and number overlap, the most trusted check;
- an outcome-polarity check (pass vs fail, done vs not done);
- format checks;
- the rate of non-answers.

**Human checks:** two blind reviewers on about 40 outputs per finalist, including all the adversarial cases.

**Measurements:** latency, RAM and energy (powermetrics) on an M1 base and an M4, comparing llama.cpp against MLX.

**Hard gates:**

| Gate | Bar |
|---|---|
| Invented numbers or identifiers | 0 |
| Pass the entity check | ≥ 95% |
| Polarity errors | ≤ 1 per 100 |
| Mean HHEM vs the Apple model | ≥ Apple's |
| Format compliance | ≥ 98% |
| Non-answers | ≤ 1% |
| Latency on an M1, 800 tokens, warm | median ≤ 3 s, p95 ≤ 6 s |
| Extra RAM | ≤ 3 GB |
| Download | ≤ the size promised to users |

**If nothing passes:** Apple's model stays the default, and the download is offered only for a model that beats it.

## Follow-up: NVIDIA and mixture-of-experts models (2026-11-16)

**NVIDIA**
- **Nemotron 3 Nano 4B** is the only NVIDIA model that fits the small tier:
  - 2.84 GB at Q4;
  - the best instruction following in its class (IFEval 81.5 at Q4 with reasoning off);
  - English only;
  - no faithfulness measurement;
  - licence: the NVIDIA Nemotron Open Model License, which allows commercial use. Redistribution needs a copy of the licence, and the licence ends if you sue over IP.
- **The larger NVIDIA models:**
  - Nemotron 3 Nano 30B-A3B: 9.6% hallucination (current board), 24.6 GB;
  - Nemotron 3.5 Lightning: no measurement, and its predecessor did poorly;
  - Bolt E4B: 6.2 GB;
  - Nemotron-Mini-4B: only 4K context.

**Mixture-of-experts (MoE)**
- **Fast, but not small.** Only a few billion parameters work per token, but every expert must be on disk and in RAM. So the file size is what counts, and no MoE model fits a ~2 GB download.
- **Under 5 GB:**
  - LFM2.5-8B-A1B: 4.8 GB, reasoning-only, no faithfulness data;
  - Granite 4.0-H-Tiny: ~4 GB, 7.3% hallucination on the old board, worse than Qwen3-4B's 2.7%.
- **For Macs with 32 GB or more:**
  - Gemma 4 26B-A4B: 16.9 GB, Apache-2.0, 5.2% hallucination (current board) against 5.7% for Qwen3-4B, which is within noise;
  - gpt-oss-20b: 3.7% (old board), ~12–13 GB;
  - Qwen3-30B-A3B-Instruct-2507: ~19 GB, non-thinking.
- **What a large tier would add** is mainly chat quality, not summary faithfulness. So: don't build it first. If it's built later for chat, it should be Gemma 4 26B-A4B, gated at 32 GB or more.

**Updated small-tier contenders for the test:**

| Model | Q4 size |
|---|---|
| Qwen3-4B-Instruct-2507 | 2.5 GB |
| Granite 4.1 3B | 2.1 GB |
| Nemotron 3 Nano 4B | 2.8 GB |
| Qwen3-1.7B | 1.1 GB |
| Gemma 4 E2B | 3.1 GB |

Qwen3.5-4B is dropped: 3.0 GB, and its family hallucinates about twice as often as Qwen3. Apple's Foundation Model remains the control.

Sources:
- NVIDIA licences: https://www.nvidia.com/en-us/agreements/enterprise-software/nvidia-nemotron-open-model-license/ and https://openmdw.ai/license/
- Model cards and quantised builds:
  - https://huggingface.co/nvidia/NVIDIA-Nemotron-3-Nano-4B-GGUF
  - https://huggingface.co/unsloth/gemma-4-26B-A4B-it-GGUF
  - https://huggingface.co/LiquidAI/LFM2.5-8B-A1B-GGUF
- MoE on a 16 GB Mac: https://modelfit.io/blog/run-35b-llm-mac-mini-m4-16gb-mmap/

## Sources (read 2026-11-16)

- **Vectara hallucination leaderboard:**
  - current board (HHEM-2.3): https://github.com/vectara/hallucination-leaderboard
  - old board: https://github.com/vectara/hallucination-leaderboard/tree/hhem-2.3-old-dataset
- **Model cards:**
  - https://huggingface.co/Qwen/Qwen3-4B-Instruct-2507
  - https://huggingface.co/Qwen/Qwen3.5-4B
  - https://huggingface.co/Qwen/Qwen3.5-2B
  - https://huggingface.co/ibm-granite/granite-4.1-3b
  - https://huggingface.co/google/gemma-4-E2B-it
  - https://huggingface.co/LiquidAI/LFM3-1.7B
  - https://huggingface.co/microsoft/Phi-4-mini-instruct
  - https://huggingface.co/mistralai/Ministral-3-3B-Instruct-2512
- **GGUF sizes:** the unsloth and bartowski repositories for each model.
- **Speeds:**
  - https://omlx.ai/benchmarks/925jucnl
  - https://developers.google.com/edge/litert-lm/models/gemma-4
  - https://lilting.ch/en/articles/lfm25-12b-jp-local-test-plan
