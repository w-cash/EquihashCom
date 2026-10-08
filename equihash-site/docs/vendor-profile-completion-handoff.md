# Complete internal vendor profiles

Checked against local commit `d6f11d0` on 8 October 2026.

## Confirmed inconsistency

- `data/vendor-directory.json` contains 87 research records.
- Two are `coverage_gap` placeholders, leaving 85 public directory rows.
- `data/vendors.json` contains 11 full internal profile records.
- The directory currently joins a research record to a profile by normalized website hostname.
- Only nine public directory rows currently match those 11 profiles. `Hashlabs` and `805 Mining` exist in `vendors.json` but have no matching research-directory row.
- Therefore **76 public rows currently have no internal profile destination**.

The machine-readable list is [`data/vendor-profile-gaps.csv`](../data/vendor-profile-gaps.csv). It contains every missing record, a suggested internal slug, profile kind, fields that can be prefilled and the remaining enrichment fields.

## Required behavior

Every public vendor name on `/vendors` must open an internal route first. The external website remains a separate `Visit website` action.

An ordinary profile with no tracked offers must still render:

1. Vendor name, profile kind and location.
2. `No Equihash listing currently recorded.`
3. Existing research facts from the linked `VendorResearch` record.
4. Evidence/source links and record-check date.
5. `Visit website` when a safe public website exists.
6. `Request a correction` and copy-profile-link actions.

Product listings, product images and seller feeds are optional enrichments. They must not be required to create a profile.

Warning/reference records also get internal profiles. Withhold their product/external purchase CTA where appropriate and show the sourced warning scope precisely. Do not turn an allegation into an Equihash.com finding.

## Data model change

Add an explicit relationship between the two records:

```rust
pub struct Vendor {
    // existing fields...
    pub research_id: Option<String>,
}
```

Resolve research data in this order:

1. Exact `research_id`.
2. Temporary hostname fallback for existing records while migrating.
3. No match.

Hostname is not a durable identity key. A business may change domains, multiple regional records may share a domain, and warning records may intentionally have no website. Once all rows have `research_id`, remove hostname-only identity matching.

The safest low-duplication implementation is either:

- generate a lightweight internal profile directly from every public `VendorResearch` row and overlay the richer `Vendor` record when present; or
- add the 76 minimal `Vendor` records from the CSV, with explicit `research_id` values.

The first option avoids copying the same company facts into two JSON files. The second requires validation preventing drift between the directory and profile record.

## Fields for a complete ordinary profile

These fields belong in `vendors.json` or in an equivalent normalized profile record:

| Field | Requirement |
|---|---|
| `id` | Stable internal profile ID. Do not derive identity from array position. |
| `research_id` | Exact foreign key to `VendorResearch.id`; required for migrated/new profiles. |
| `slug` | Unique, stable internal URL slug. CSV contains suggestions, not immutable decisions. |
| `name` | Public display name. |
| `url` | Public vendor website, or null for warning/reference records. |
| `base_region` | Seller/operator base as supported by the source. |
| `region_focus` | Directory region used for navigation. |
| `regions` | Verified or seller-declared shipping/service destinations. Empty means unknown. |
| `channel` | `manufacturer`, `independent_retailer`, `marketplace`, `broker_hosting`, or `warning_record`. |
| `legal_name` | Structured legal entity name if established; otherwise null. |
| `registration` | Registration fact and jurisdiction if established; otherwise null. |
| `registry_url` | Primary registry source where available. |
| `verification_label` | Exact scope of the check, such as company name/number match. It is not a trust badge. |
| `verification_url` | Source supporting the stated verification scope. |
| `notes` | Use `No Equihash listing currently recorded.` for ordinary empty profiles. |
| `source_url` | Main profile source. Additional sources continue to come from `VendorResearch.source_urls`. |
| `observed_at` | Date/time the structured profile facts were checked. Do not refresh this from an unrelated price update. |
| logo/monogram | Reviewed local logo or generated monogram; no remote unreviewed images. |

Do not invent missing identity or shipping values. The CSV deliberately leaves research-dependent values in `missing_required_enrichment`. Existing prose in `vendor-directory.json` is a lead; it is not automatically safe to parse into a registry fact.

## Route and UI change

Change `public_vendor_row` so the vendor name always links internally. A direct website link remains in the `Record` column and on the profile page.

Suggested migration route:

```text
/vendors/{profile-slug}
```

Use redirects if a generated slug later becomes a curated slug. Do not reuse retired slugs for a different business.

The existing `vendor_page` already handles zero listings and displays the desired empty state. Reuse it. Ensure research-only profiles can also build source links, evidence details and correction links without a `Listing` row.

## Validation and tests

Add these acceptance checks:

1. Every non-`coverage_gap` directory record produces exactly one internal profile URL.
2. Every internal URL returns 200 and contains the correct vendor name and directory record ID.
3. Vendor-name links never navigate directly to an external domain.
4. Every ordinary profile has a distinct `Visit website` button when `website` is present.
5. A profile with zero listings renders `No Equihash listing currently recorded.` and its research sources.
6. Warning profiles withhold purchase/product CTAs and clearly label their evidence type.
7. Duplicate website hosts do not merge distinct regional/public-warning records.
8. Changing a vendor website does not change its profile identity or slug.
9. `Hashlabs` and `805 Mining` are either added to the research directory or intentionally documented as detailed-only profiles; the mismatch must be explicit.
10. Data validation fails for duplicate `research_id`, duplicate slug, missing referenced research record or an external-only vendor-name link.

Expected post-migration invariant:

```text
public directory rows: 85
public rows with internal profile: 85
public rows linking externally from the vendor name: 0
```

## Publication order

1. Add `research_id` and loader validation.
2. Generate/render research-only profiles without requiring offers.
3. Change vendor-name links to internal routes.
4. Add or review slugs/monograms for the 76 rows in the CSV.
5. Enrich legal identity, registration, shipping and verification fields from cited sources.
6. Add tracked products only when exact product evidence is available.

This separates profile completeness from product coverage. Users get consistent navigation immediately while unverified structured facts remain honestly unknown.
