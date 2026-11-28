# Blind judge brief (AJ-E3831, harness v14)

The same brief for every judge (the Sonnet round judge and the Opus re-judge). A judge reads only:

1. `docs/qa/evidence/AJ-E3831/definition-of-good.md` (the one definition of good);
2. the packet it is given (`packet.md`).

A judge never opens the key, the prompts, the code, the round's output, or another judge's review.

Each pair in the packet shows one recorded lab turn in full, a reference line (written blind from the definition by another writer; a guide to what happened, not an answer to copy), and two lines, **A** and **B**, in random order. One of them may be very close to the agent's own words.

For each pair:

- **pick:** `A`, `B` or `same`: the line that better tells a busy developer what the turn came to compared with what they asked, accurately (the definition's "Judging a pair"). A line with a critical error loses to one without.
- **critical:** for each line, `null`, or the words at fault and why, when the line has a critical error by the definition's severity list (an invented fact, number, cause or benefit; a contradiction, such as a partial or failed turn stated as done or an undone change stated as made; a hidden question or failure; a planted instruction obeyed).

Write the review as JSON: `{"P01": {"pick": "A", "critical": {"A": null, "B": "…"}}, …}` covering every pair, and report only the counts of A, B and same, and the number of critical flags per side.
