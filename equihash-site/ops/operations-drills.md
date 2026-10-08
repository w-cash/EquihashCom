# Operations drill cards

Each drill records date, operator, observer, candidate snapshot/commit, expected result, actual result, elapsed time, data lost, follow-up owner and evidence path. A written card is not a passed drill.

## Snapshot restore

Corrupt one candidate generated file in an isolated data directory. Confirm validation fails, the active manifest does not change, last-good pages remain complete, `/healthz` exposes the failure, and recovery succeeds after a valid candidate appears.

## Application rollback

Deploy the candidate to a staging-equivalent directory, switch the `current` symlink, verify routes and health, then restore the previous release. Confirm static assets, binary and data schema remain compatible in both directions.

## Stale and unavailable sources

Block a pool source, one approved seller page and NiceHash independently. Confirm unrelated fields keep their own timestamps, seller current availability expires to unknown/historical, the marketplace does not become zero, and pages state which dataset is stale without a global false alarm.

## Rights withdrawal

Select a disposable fixture source. Mark display and export rights revoked, regenerate pages and downloads, purge the staging cache, and search all public artifacts for the removed value. Confirm only the permitted audit trace remains.

## Unknown consensus upgrade

Introduce a future/proposed algorithm epoch fixture. Confirm it cannot become the active compatibility profile, cannot preselect a calculator route and displays as proposed with its source.

## Urgent malicious destination

Replace a fixture outbound source with a known-safe test URL marked malicious. The primary or backup operator must suppress it across pages, JSON and caches without waiting for an ordinary editorial cycle. Record who could perform the action; the drill fails if only an unavailable person has access.

## Privacy redaction

Insert synthetic wallet-like, credential-like and free-text values into a private staging event. Exercise deletion in application storage, logs, analytics/export fixtures and caches. Confirm allowed aggregates remain while the raw synthetic value is absent.
