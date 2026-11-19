# Evidence manifest — AJ-E3811

Task: AJ-E3811 (the recommendation: the best model for this Mac, D-131).
Status: **IN-PROGRESS**: the pure recommendation function and its
hardware matrix are built and tested. Reading the facts through the OS
seams, the reason's words in `feed-words.ts`, and feeding the router and
the Models screen are still to do.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R2 Apple's model first when Apple Intelligence is on and its entry passes a job | cargo `juggler-app` `features::ai::models::recommend_tests::every_mac_in_the_matrix_gets_its_one_answer` (8 GB and 16 GB with Apple Intelligence on; an empty or unread list still gives the built-in Apple entry; Apple narrowed to no pass falls through) | Apple, reason "built in" | pass | PASS | the AJ-E3811 recommend commit | 2026-11-18 | Claude Opus (self) |
| R3 otherwise the listed model passing the most jobs that fits the chip, the memory and the disk (size plus 10 GB); none fits: nothing, with why | the same test, `fixtures/recommend-matrix.json`: 16 GB off gives Qwen3 1.7B; exactly size plus 10 GB free fits, one byte under gives "not enough disk" with the bytes needed; 8 GB gives "not enough memory" (and says whether turning Apple Intelligence on would help); Intel gives nothing; a withdrawn Qwen3 gives Granite; an installed Granite wins the tie when the disk is short | as listed | pass | PASS | the AJ-E3811 recommend commit | 2026-11-18 | Claude Opus (self) |
| R5 pure and deterministic, the hardware matrix a fixture, no branch on a model id | `models/recommend.rs` (a table-driven function over the typed section); the test calls it twice per case | same answer twice; no id in the code | pass | PASS | the AJ-E3811 recommend commit | 2026-11-18 | Claude Opus (self) |
| R1 chip, memory, free disk and Apple Intelligence read through the OS seams (D-52) | the seam readers | — | not built | PENDING | — | — | — |
| R4 one plain reason line from `feed-words.ts` | the words for each `Why` and `WhyNot` | — | not built (the function returns the reason and its facts) | PENDING | — | — | — |
| R6 feeds the router's Automatic default and the Models screen; changes with Apple Intelligence or the list | AJ-E3802, AJ-E3812 | — | not built | PENDING | — | — | — |
| Performance: under 10 ms from cached facts | a timed unit test over the matrix | ≤ 10 ms | not measured | PENDING | — | — | — |
