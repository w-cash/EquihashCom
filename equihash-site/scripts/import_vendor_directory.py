#!/usr/bin/env python3
"""Validate and import the global ASIC vendor research package."""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter
from pathlib import Path
from urllib.parse import urlparse


REQUIRED_FIELDS = {
    "region",
    "country",
    "vendor",
    "website",
    "vendor_type",
    "trust_tier",
    "editorial_trust_score_100",
    "status",
    "legal_identity_location",
    "history_and_reputation",
    "payment_protection",
    "shipping_customs",
    "pickup",
    "warranty_rma",
    "equihash_z15",
    "estimated_delivery_probability",
    "notes",
    "source_urls",
    "last_verified",
    "global_trust_rank",
}
TIERS = {"A", "B", "C", "D", "N/A"}


def slugify(value: str) -> str:
    value = value.lower().replace("&", " and ")
    return re.sub(r"[^a-z0-9]+", "-", value).strip("-") or "vendor"


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

    ids: set[str] = set()
    ranks: set[int] = set()
    for index, vendor in enumerate(vendors, start=1):
        missing = REQUIRED_FIELDS.difference(vendor)
        if missing:
            raise SystemExit(f"record {index} is missing: {', '.join(sorted(missing))}")
        if vendor["trust_tier"] not in TIERS:
            raise SystemExit(f"record {index} has an invalid trust tier")
        rank = vendor["global_trust_rank"]
        if rank in ranks:
            raise SystemExit(f"duplicate global trust rank: {rank}")
        ranks.add(rank)
        website = vendor.get("website", "").strip()
        if website and not is_http_url(website):
            raise SystemExit(f"record {index} has an invalid website URL")
        if not all(is_http_url(url) for url in vendor["source_urls"]):
            raise SystemExit(f"record {index} has an invalid evidence URL")

        record_id = slugify(vendor["vendor"])
        if record_id in ids:
            record_id = f"{record_id}-{slugify(vendor['country'])}"
        if record_id in ids:
            record_id = f"{record_id}-{rank}"
        ids.add(record_id)
        vendor["id"] = record_id
        vendor["website"] = website or None

    counts = Counter(vendor["trust_tier"] for vendor in vendors)
    if counts != Counter({"A": 16, "B": 22, "C": 31, "D": 16, "N/A": 2}):
        raise SystemExit(f"unexpected trust-tier totals: {dict(counts)}")
    if ranks != set(range(1, len(vendors) + 1)):
        raise SystemExit("global trust ranks must be consecutive")

    payload["vendors"] = sorted(vendors, key=lambda vendor: vendor["global_trust_rank"])
    args.destination.parent.mkdir(parents=True, exist_ok=True)
    args.destination.write_text(
        json.dumps(payload, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(f"Imported {len(vendors)} vendor research records into {args.destination}")


if __name__ == "__main__":
    main()
