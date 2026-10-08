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
LOCAL_EVIDENCE_FIELDS = {
    "domain_name",
    "domain_registered_at",
    "domain_registered_year_verified",
    "domain_registration_checked_at",
    "domain_registration_source",
    "domain_registration_source_url",
    "review_platform",
    "review_rating",
    "review_count",
    "review_snapshot_date",
    "review_checked_at",
    "review_source_url",
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
    # Keep locally refreshed Trustpilot and domain-registry facts when the canonical research
    # directory is imported again. Both evidence sets have independent refresh scripts/clocks.
    previous = {}
    if args.destination.exists():
        previous_payload = json.loads(args.destination.read_text(encoding="utf-8"))
        previous = {record["id"]: record for record in previous_payload.get("vendors", [])}
    for vendor in vendors:
        old = previous.get(vendor["id"], {})
        for field in LOCAL_EVIDENCE_FIELDS:
            if field in old:
                vendor[field] = old[field]
        vendor["review_count"] = vendor.get("review_count") or 0

    # Keep the site's sourced warning classification for the known ASICKings variants record; the
    # upstream research export currently labels its role generically even though its evidence and
    # availability fields still describe a warning record.
    for vendor in vendors:
        if vendor["id"] == "public-warning-records-global-asic-kings-asickings-variants":
            vendor["record_type"] = "public_warning_record"

    payload["default_order"] = (
        "Trustpilot review count descending; vendors without a sourced Trustpilot profile count as 0."
    )
    payload["sort_modes"] = [
        {
            "id": "review_count",
            "label": "Trustpilot reviews · highest first",
            "field": "review_count",
            "direction": "descending",
            "default": True,
            "missing_value": 0,
        },
        {
            "id": "domain_age",
            "label": "Domain age · oldest first",
            "field": "domain_registered_at",
            "direction": "ascending",
            "missing_values": "last",
        },
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
