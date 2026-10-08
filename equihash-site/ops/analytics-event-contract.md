# Analytics and privacy contract

The current baseline is **unknown**. No traffic, retention, conversion, cohort, or market-size figure may be invented from server availability or search-console impressions.

The default implementation stores setup state in the browser. If analytics is later enabled, it requires a named controller, published notice, retention period, objection/deletion path, and deployment-specific consent decision before collection.

## Allowed event shapes

| Event | Allowed properties | Purpose |
|---|---|---|
| `setup_saved_local` | algorithm parameters, miner model ID, coarse country code, cost-field completeness booleans | Determine whether users can preserve a setup |
| `explanation_run` | model ID, pool ID, comparison window, missing-input flags, outcome band | Measure completion of the recurring explanation task |
| `source_opened` | internal source ID, page type, field name | Check whether evidence is reachable |
| `decision_outcome_sampled` | task type, useful yes/no, action category, cohort eligibility | Sample real decision usefulness |
| `correction_requested` | record type, urgency class, channel | Operate corrections without storing the allegation in analytics |

Never send raw search text, wallet or payout address, worker name, stratum credential, free-form correction text, email, IP-derived precise location, seller evidence, or full outbound URL. Query understanding stays request-local; aggregate known aliases/filters only after a consent and retention decision.

## Required deployment decisions

- analytics provider: `UNASSIGNED`
- lawful basis / consent behavior: `UNASSIGNED`
- retention period: `UNASSIGNED`
- deletion and objection owner: `UNASSIGNED`
- IP handling and log retention: `UNASSIGNED`

Until those fields are assigned, product analytics stays disabled and discovery evidence is recorded in the private, access-controlled pilot log.
