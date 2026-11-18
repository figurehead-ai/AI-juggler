Synthetic held-out only; not checked on real messages (the maintainer declined a real held-out set).
Chosen variants: apple greedy + datamark on; qwen3-1.7b-q8 low + datamark off; nemotron-3-nano-4b low + datamark off.
Corpora: heldout (feed line job; review/attention/chat also ran on it) and heldout-jobs (review, attention, chat). Attention has no blind packet (report_v4 builds none). Packets are the tool's JSON, one per corpus and job; one key file per model, never opened.
