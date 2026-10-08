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
| `domain_name`, `domain_registered_at`, `domain_registered_year_verified` | Registrable domain and registry creation date/year from RDAP or WHOIS; unavailable dates remain `null`. |
| `domain_registration_checked_at`, `domain_registration_source`, `domain_registration_source_url` | Independent evidence clock and registry lookup provenance. |
| `review_platform`, `review_rating`, `review_count`, `review_snapshot_date`, `review_checked_at`, `review_source_url` | Attributed Trustpilot snapshot. Vendors without a sourced Trustpilot profile use `review_count: 0`. These fields are never converted into a house score. |
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

1. **Trustpilot review count** — the default; current sourced snapshot count descending. A vendor without a sourced Trustpilot profile has a count of `0`.
2. **Domain age** — oldest verified registry creation date first; unavailable dates last.

Neither order is a quality judgment. Do not create a hidden composite, rating tier, or recommendation from these fields.

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
