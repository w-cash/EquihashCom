#!/usr/bin/env python3
"""Validate and import the neutral public ASIC vendor directory."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from urllib.parse import urlparse


REQUIRED_FIELDS = {
    "id",
    "region",
    "country",
    "vendor",
    "website",
    "record_type",
    "manufacturer_direct",
    "legal_identity_and_location_summary",
    "company_incorporated_year_verified",
    "public_operating_since_year",
    "domain_registered_year_verified",
    "review_platform",
    "review_rating",
    "review_count",
    "review_snapshot_date",
    "review_source_url",
    "declared_availability",
    "availability_basis",
    "equihash_z15_claim",
    "payment_methods_and_protection",
    "shipping_customs",
    "pickup",
    "warranty_rma",
    "source_urls",
    "last_verified",
}
FORBIDDEN_FIELDS = {
    "trust_tier",
    "editorial_trust_score_100",
    "global_trust_rank",
    "estimated_delivery_probability",
    "recommendation",
}
RECORD_TYPES = {
    "manufacturer_direct",
    "reseller_or_broker",
    "marketplace",
    "hardware_and_hosting",
    "coverage_gap",
    "public_warning_record",
}
AVAILABILITY = {
    "seller_declared_spot_or_near_term",
    "preorder_or_future_batch",
    "historical_or_used",
    "unknown_or_quote_required",
    "sold_out_or_no_current_listing",
    "coverage_gap",
    "not_applicable_warning_record",
}


def is_http_url(value: str) -> bool:
    parsed = urlparse(value)
    return parsed.scheme in {"http", "https"} and bool(parsed.hostname)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()

    payload = json.loads(args.source.read_text(encoding="utf-8"))
    vendors = payload.get("vendors")
    if not isinstance(vendors, list) or not vendors:
        raise SystemExit("source has no vendor records")
    if payload.get("vendor_count") != len(vendors):
        raise SystemExit("vendor_count does not match the record count")
    if len(vendors) != 87:
        raise SystemExit(f"expected 87 canonical records, found {len(vendors)}")

    ids: set[str] = set()
    for index, vendor in enumerate(vendors, start=1):
        missing = REQUIRED_FIELDS.difference(vendor)
        if missing:
            raise SystemExit(f"record {index} is missing: {', '.join(sorted(missing))}")
        forbidden = FORBIDDEN_FIELDS.intersection(vendor)
        if forbidden:
            raise SystemExit(f"record {index} contains house judgments: {', '.join(sorted(forbidden))}")
        if vendor["record_type"] not in RECORD_TYPES:
            raise SystemExit(f"record {index} has an invalid record_type")
        if vendor["declared_availability"] not in AVAILABILITY:
            raise SystemExit(f"record {index} has invalid declared_availability")
        if vendor["id"] in ids:
            raise SystemExit(f"duplicate id: {vendor['id']}")
        ids.add(vendor["id"])
        for field in ("website", "review_source_url"):
            value = vendor.get(field)
            if value and not is_http_url(value):
                raise SystemExit(f"record {index} has an invalid {field}")
        if not all(is_http_url(url) for url in vendor["source_urls"]):
            raise SystemExit(f"record {index} has an invalid evidence URL")
        count = vendor.get("review_count")
        rating = vendor.get("review_rating")
        if count is not None and (not isinstance(count, int) or count < 0):
            raise SystemExit(f"record {index} has an invalid review_count")
        if rating is not None and (not isinstance(rating, (int, float)) or not 0 <= rating <= 5):
            raise SystemExit(f"record {index} has an invalid review_rating")

    expected = sorted(vendors, key=lambda vendor: (vendor["vendor"].casefold(), vendor["id"]))
    if vendors != expected:
        raise SystemExit("canonical vendors must be alphabetical by public vendor name")
    if any(vendor["domain_registered_year_verified"] is not None for vendor in vendors):
        raise SystemExit("domain-age sorting must remain disabled until the canonical policy changes")

    # Publication gate: the source research may retain copied review statistics, but the public
    # site ships ordinary profile links only until rights and review-integrity operations are
    # evidenced. A neutral sort or disclaimer is not publication clearance.
    for vendor in vendors:
        vendor["review_rating"] = None
        vendor["review_count"] = None
    payload["methodology"]["review_rule"] = (
        "Only ordinary third-party review-profile links and their check dates are published. "
        "Copied scores, counts and related sorts remain disabled pending publication clearance."
    )
    payload["disclaimer"] = (
        "Informational directory only. A listing, position, seller claim, profile link, or availability "
        "label is not an endorsement or guarantee. Verify legal entity, stock, serials, invoice "
        "beneficiary, taxes, warranty and delivery terms before payment."
    )
    coverage = payload["methodology"].get("field_coverage", {})
    coverage.pop("trustpilot_snapshot_records", None)
    coverage["third_party_review_profile_links"] = sum(
        bool(vendor.get("review_source_url")) for vendor in vendors
    )
    payload["sort_modes"] = [
        mode
        for mode in payload.get("sort_modes", [])
        if mode.get("id") not in {"review_count", "review_rating_min_20"}
    ]

    args.destination.parent.mkdir(parents=True, exist_ok=True)
    args.destination.write_text(
        json.dumps(payload, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    profiles = sum(bool(vendor.get("review_source_url")) for vendor in vendors)
    print(f"Imported {len(vendors)} neutral vendor records ({profiles} review-profile links)")


if __name__ == "__main__":
    main()
