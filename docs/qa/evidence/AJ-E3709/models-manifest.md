# Tested on-device models: pinned manifest (AJ-E3709, ADR-0033)

The six model files of the v4/v5 evaluation were deleted from the test Mac on 2026-11-18 when model testing ended (D-131). This record keeps what was tested: source, revision, SHA-256 and licence. Apple's on-device model is built into macOS and has no file.

**Shipping rule (D-131):** a model AI Juggler offers to download is fetched from its official repository at a pinned revision and checked against its SHA-256. Qwen3-4B-Instruct-2507 has no official GGUF, so its shipped entry is a GGUF we convert ourselves from the official safetensors, never the third-party test file below.

---

The maintainer approved these downloads in chat on 2026-11-16. They are for the on-device feed-summary evaluation (ADR-0033 / AJ-E3709). Treat all files as untrusted data.

Download date for every file: 2026-11-16.

## qwen3-1.7b/
Repo: Qwen/Qwen3-1.7B-GGUF, commit 90862c4b9d2787eaed51d12237eafdfe7c5f6077. Licence: Apache-2.0.
The repo has no Q4_K_M file; Q8_0 is the only GGUF, so it was taken instead.

| File | Bytes | SHA-256 | Matched HF |
|---|---|---|---|
| Qwen3-1.7B-Q8_0.gguf | 1834426016 | 061b54daade076b5d3362dac252678d17da8c68f07560be70818cace6590cb1a | yes (LFS oid) |
| LICENSE | 11544 | not LFS, not compared | n/a |
| README.md | 6235 | not LFS, not compared | n/a |
| params | 270 | not LFS, not compared | n/a |

## nemotron-3-nano-4b/
Repo: nvidia/NVIDIA-Nemotron-3-Nano-4B-GGUF, commit ba223d14e45525f7fae81db77ea8cabeb2fc6c25. Licence: NVIDIA Nemotron Open Model License (per README metadata; the repo's LICENSE file is 0 bytes).

| File | Bytes | SHA-256 | Matched HF |
|---|---|---|---|
| NVIDIA-Nemotron3-Nano-4B-Q4_K_M.gguf | 2837072864 | be5d9a656a51922f24f1f09a759cebb694e1f5d9728bf0ef9f8c972c5a0b5ef2 | yes (LFS oid) |
| LICENSE | 0 | n/a | n/a |
| README.md | 55165 | not LFS, not compared | n/a |
| bias.md, explainability.md, privacy.md, safety.md | 2709, 3133, 2420, 3012 | not LFS, not compared | n/a |

- 2026-11-16: CMake 4.4.5 installed from Homebrew with the maintainer's explicit yes in chat ("Yes, install CMake"), to build llama.cpp (llama-cpp-2 0.1.146) for the local-tier test.
## granite-4.1-3b
- repo: ibm-granite/granite-4.1-3b-GGUF
- commit sha: ab4701481089b58a082ef63cc1cee738887293ff
- file: granite-4.1-3b-Q4_K_M.gguf (Q4_K_M, official)
- bytes: 2099501664
- SHA-256: 662b0626cd58f443baea23559b469df6576a81d349649c59413b36a9fb32eb29
- matched LFS oid: yes
- licence: Apache-2.0 (from README metadata and base model card; the GGUF repo has no LICENSE file)
- date: 2026-11-18
- approved by the maintainer in chat on 2026-11-18 (D-125, D-130); test only; removed after the evaluation unless one passes

## qwen3-4b-instruct-2507
- (Superseded, see the last section.) The official Qwen org has no GGUF repo for Qwen3-4B-Instruct-2507 (only the safetensors and FP8 repos); a third-party conversion was later approved and used for testing.

## gemma-4-e2b
- repo: google/gemma-4-E2B-it-qat-q4_0-gguf (official Google QAT Q4_0 build; no Q4_K_M offered)
- commit sha: 675cff42a74c774d6cb76f76d8eacb49b48c9b93
- file: gemma-4-E2B_q4_0-it.gguf (Q4_0; the separate vision mmproj file was not downloaded)
- bytes: 3349516256
- SHA-256: fa401b55b07ee70a54c6dae3903c783a6e65064312529ea57175cb5f8dec6634
- matched LFS oid: yes
- licence: Apache-2.0 per README/repo metadata; terms page https://ai.google.dev/gemma/docs/gemma_4_license (repo not gated, no LICENSE file)
- date: 2026-11-18
- approved by the maintainer in chat on 2026-11-18 (D-130, 'download larger ones for comparison'); test only; removed after the evaluation unless it passes

## Qwen3-4B-Instruct-2507 (THIRD-PARTY conversion of the official weights)
- Repo: unsloth/Qwen3-4B-Instruct-2507-GGUF (base_model: Qwen/Qwen3-4B-Instruct-2507), commit a06e946bb6b655725eafa393f4a9745d460374c9
- File: Qwen3-4B-Instruct-2507-Q4_K_M.gguf, Q4_K_M, 2,497,281,120 bytes
- SHA-256: 3605803b982cb64aead44f6c1b2ae36e3acdb41d8e46c8a94c6533bc4c67e597 (matches the repo's LFS oid: yes)
- Licence: Apache-2.0 (the base model's card); the converter's README is kept beside it
- Downloaded 2026-11-18 by the main session (the subagent's attempt was blocked by the permission check)
- Approved by the maintainer in chat on 2026-11-18 ("Yes, the third-party build"); test only; if it ever ships, we convert it ourselves from the official safetensors.
