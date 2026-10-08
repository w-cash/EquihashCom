#!/usr/bin/env python3
"""Refresh vendor domain-registration evidence from RDAP with WHOIS fallback.

The script writes only registry facts into data/vendor-directory.json. It never
uses a website footer, vendor claim, search snippet, or company incorporation
date as a substitute for the domain registration date.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import subprocess
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_DATA = ROOT / "data" / "vendor-directory.json"
BOOTSTRAP_URL = "https://data.iana.org/rdap/dns.json"
CHECKED_AT = dt.datetime.now(dt.timezone.utc).date().isoformat()
MULTI_LABEL_SUFFIXES = {
    "co.uk",
    "com.au",
    "co.za",
    "com.br",
    "com.ar",
    "com.mx",
    "co.nz",
    "com.py",
}
DATE_PATTERNS = (
    re.compile(r"(?im)^Creation Date:\s*(\d{4}-\d{2}-\d{2})"),
    re.compile(r"(?im)^Created (?:On|Date):\s*(\d{4}-\d{2}-\d{2})"),
    re.compile(r"(?im)^Original Created:\s*(\d{4}-\d{2}-\d{2})"),
    re.compile(r"(?im)^Registered(?: on)?:\s*(\d{4}-\d{2}-\d{2})"),
    re.compile(r"(?im)^Domain Name Commencement Date:\s*(\d{4}-\d{2}-\d{2})"),
    re.compile(r"(?im)^Registration Time:\s*(\d{4}-\d{2}-\d{2})"),
    re.compile(r"(?im)^created-date:\s*(\d{4}-\d{2}-\d{2})"),
)


def load_json(url: str, timeout: int = 10) -> dict:
    request = urllib.request.Request(
        url,
        headers={
            "Accept": "application/rdap+json, application/json",
            "User-Agent": "Equihash.com vendor evidence refresh/1.0",
        },
    )
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return json.load(response)


def host_from_url(value: str | None) -> str | None:
    if not value:
        return None
    host = urllib.parse.urlparse(value).hostname
    if not host:
        return None
    host = host.lower().rstrip(".")
    return host[4:] if host.startswith("www.") else host


def registrable_domain(host: str) -> str:
    labels = host.split(".")
    if len(labels) <= 2:
        return host
    last_two = ".".join(labels[-2:])
    if last_two in MULTI_LABEL_SUFFIXES:
        return ".".join(labels[-3:])
    return last_two


def rdap_services() -> dict[str, str]:
    payload = load_json(BOOTSTRAP_URL)
    result: dict[str, str] = {}
    for tlds, urls in payload.get("services", []):
        if not urls:
            continue
        for tld in tlds:
            result[tld.lower()] = urls[0]
    return result


def normalize_date(value: str | None) -> str | None:
    if not value:
        return None
    match = re.match(r"^(\d{4}-\d{2}-\d{2})", value)
    return match.group(1) if match else None


def rdap_registration(domain: str, services: dict[str, str]) -> tuple[str | None, str | None, str]:
    tld = domain.rsplit(".", 1)[-1]
    base = services.get(tld)
    if not base:
        return None, None, "RDAP service not published by IANA"
    url = f"{base.rstrip('/')}/domain/{urllib.parse.quote(domain)}"
    try:
        payload = load_json(url)
    except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError, ValueError) as error:
        return None, url, f"RDAP lookup failed: {type(error).__name__}"
    dates = []
    for event in payload.get("events", []):
        action = str(event.get("eventAction", "")).lower()
        if action in {"registration", "registered"}:
            parsed = normalize_date(event.get("eventDate"))
            if parsed:
                dates.append(parsed)
    return (min(dates) if dates else None), url, "IANA-bootstrapped RDAP"


def whois_registration(domain: str) -> tuple[str | None, str]:
    try:
        process = subprocess.run(
            ["whois", domain],
            check=False,
            capture_output=True,
            text=True,
            timeout=20,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return None, f"WHOIS lookup failed: {type(error).__name__}"
    output = process.stdout + "\n" + process.stderr
    dates = []
    for pattern in DATE_PATTERNS:
        dates.extend(match.group(1) for match in pattern.finditer(output))
    return (min(dates) if dates else None), "Registry WHOIS fallback"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--data", type=Path, default=DEFAULT_DATA)
    parser.add_argument("--delay", type=float, default=0.12)
    args = parser.parse_args()

    payload = json.loads(args.data.read_text())
    services = rdap_services()
    cache: dict[str, tuple[str | None, str | None, str]] = {}

    for record in payload["vendors"]:
        if record.get("record_type") == "coverage_gap":
            continue
        host = host_from_url(record.get("website"))
        if not host or host == "n/a":
            record["domain_name"] = None
            record["domain_registered_at"] = None
            record["domain_registered_year_verified"] = None
            record["domain_registration_checked_at"] = CHECKED_AT
            record["domain_registration_source"] = "No vendor domain recorded"
            record["domain_registration_source_url"] = None
            continue
        domain = registrable_domain(host)
        if (
            record.get("domain_name") == domain
            and record.get("domain_registration_checked_at") == CHECKED_AT
            and record.get("domain_registration_source") == "IANA-bootstrapped RDAP"
            and record.get("domain_registered_at")
        ):
            cache.setdefault(
                domain,
                (
                    record["domain_registered_at"],
                    record.get("domain_registration_source_url"),
                    record["domain_registration_source"],
                ),
            )
            continue
        if domain not in cache:
            registered_at, source_url, source = rdap_registration(domain, services)
            if not registered_at:
                registered_at, fallback_source = whois_registration(domain)
                source = f"{source}; {fallback_source}"
            cache[domain] = (registered_at, source_url, source)
            print(f"{domain}\t{registered_at or 'unknown'}\t{source}")
            time.sleep(args.delay)
        registered_at, source_url, source = cache[domain]
        record["domain_name"] = domain
        record["domain_registered_at"] = registered_at
        record["domain_registered_year_verified"] = (
            int(registered_at[:4]) if registered_at else None
        )
        record["domain_registration_checked_at"] = CHECKED_AT
        record["domain_registration_source"] = source
        record["domain_registration_source_url"] = source_url

    args.data.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n")
    found = sum(
        1
        for record in payload["vendors"]
        if record.get("record_type") != "coverage_gap" and record.get("domain_registered_at")
    )
    print(f"stored registration dates for {found} public vendor records")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
