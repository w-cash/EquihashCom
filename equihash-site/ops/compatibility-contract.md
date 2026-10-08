# Equihash compatibility and consensus contract

Compatibility is an exact tuple, not the word “Equihash.” Every hardware, coin, pool endpoint and calculator path must carry enough information to compare:

```text
(n, k, personalization, network_or_chain_id, activation_epoch, lifecycle_state)
```

- `n` and `k` are required integers for a compatibility assertion.
- `personalization` is required when a chain changes the proof domain; unknown never matches known.
- `network_or_chain_id` prevents a testnet or fork endpoint from appearing interchangeable with mainnet.
- `activation_epoch` identifies the height/time range in which the rule applies.
- `lifecycle_state` is one of `active`, `announced`, `proposed`, `ended`, or `unknown`.

A machine’s manufacturer rating and a seller configuration are separate claims. A seller-advertised hashrate cannot overwrite the rated model specification. Variant identity, tuning, firmware, hashrate, watts, tolerance, source and observed time travel together.

Unknown values fail closed: the UI says “compatibility not established” and does not recommend a pool, coin or calculator preset. Announced or proposed rules are never shown as current. A source transition creates a new epoch; it does not rewrite history.

The initial contract fixture belongs in typed data and must include negative tests for same family/different `(n,k)`, unknown personalization, future activation, ended proof of work, testnet/mainnet mismatch, and seller-overclock data presented as manufacturer rating.
