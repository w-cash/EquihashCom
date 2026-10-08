# Public vendor schema and sorting rules

## Public fields

| Field | Meaning |
|---|---|
| `id` | Stable slug derived from region, country, and vendor name. |
| `region`, `country`, `vendor`, `website` | Navigation and public identity fields. |
| `record_type` | Factual role: manufacturer direct, reseller/broker, marketplace, hardware/hosting, coverage gap, or sourced public-warning record. |
| `manufacturer_direct` | `true` only for a named manufacturer's official shop in the source research. |
| `legal_identity_and_location_summary` | Registry/location evidence summary; absence is not proof of wrongdoing. |
| `company_incorporated_year_verified` | Incorporation year only when explicitly stated in the cited evidence; otherwise `null`. |
| `public_operating_since_year` | Earliest explicitly stated founded/established/operating year; not necessarily the current legal entity's age. |
| `domain_registered_year_verified` | Registry year only when checked; otherwise `null`. |
| `review_platform`, `review_rating`, `review_count`, `review_snapshot_date`, `review_source_url` | Dated third-party review snapshot. Values from different platforms must not be mixed into one score. |
| `declared_availability` | Normalized state derived from the seller page, quote, or cited public record. It is not an inventory audit. |
| `availability_basis` | Required attribution and verification limitation. |
| `equihash_z15_claim` | Product, hashrate, price, stock, or batch language reported in the source research. |
| `payment_methods_and_protection` | Descriptive payment terms and chargeback/escrow limitations. |
| `shipping_customs`, `pickup`, `warranty_rma` | Descriptive transaction terms. |
| `source_urls` | Evidence links. JSON uses an array; CSV uses ` | ` separators. |
| `last_verified` | Evidence snapshot date. |

The public schema intentionally excludes house trust tiers, house scores, global ranks, delivery probabilities, and labels such as “recommended,” “safe,” or “scam.”

## Allowed sorting modes

The page heading must say `Sorted by: <label>` and show the direction and missing-data rule.

1. **Alphabetical** — default; Unicode-normalized vendor name ascending.
2. **Seller-declared availability** — `seller_declared_spot_or_near_term`, `preorder_or_future_batch`, `historical_or_used`, `unknown_or_quote_required`, `sold_out_or_no_current_listing`, coverage/warning records. The label must retain “seller-declared.”
3. **Trustpilot review count** — descending; null last; show platform and snapshot date.
4. **Trustpilot rating** — descending only when `review_count >= 20`; tie-break by review count, then alphabetical. Profiles below the minimum appear as “insufficient sample,” not as zero.
5. **Verified company incorporation year** — oldest first; null last. Explain that company age is not a quality guarantee.
6. **Verified domain registration year** — oldest first; null last. This sort remains disabled while coverage is zero. When enabled, explain that domain age is not a quality guarantee and can outlive a change of ownership.
7. **Manufacturer direct** — `true` first, then alphabetical. Explain that current stock and destination support still require checking.
8. **Last verified** — newest date first, then alphabetical.

Do not create a hidden composite of these fields. If Equihash.com later adds another sort, publish its exact formula and source.

## Availability definitions

| Value | Display label |
|---|---|
| `seller_declared_spot_or_near_term` | Seller declares spot or near-term dispatch |
| `preorder_or_future_batch` | Preorder or future batch |
| `historical_or_used` | Historical or used listing |
| `unknown_or_quote_required` | Unknown or quote required |
| `sold_out_or_no_current_listing` | Sold out or no current Equihash listing |
| `coverage_gap` | No researched local vendor record |
| `not_applicable_warning_record` | Public warning/clone record; availability not applicable |

## Publication validation

Run from this directory:

```bash
python3 build_vendor_directory.py
python3 - <<'PY'
import csv, json
from pathlib import Path
p = Path('.')
d = json.loads((p / 'global-asic-vendors-2026-10-08.json').read_text())
assert len(d['vendors']) == 87
assert [x['vendor'].casefold() for x in d['vendors']] == sorted(x['vendor'].casefold() for x in d['vendors'])
for forbidden in ('trust_tier', 'trust_score', 'global_trust_rank', 'delivery_probability'):
    assert forbidden not in json.dumps(d).lower()
assert sum(1 for _ in csv.DictReader((p / 'global-asic-vendors-2026-10-08.csv').open())) == 87
print('validation passed')
PY
```
