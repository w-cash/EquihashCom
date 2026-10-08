# Deployment security checklist

The public application is read-only: there is no account system, checkout or public write endpoint. Submission pages provide copyable templates rather than accepting secrets. Publisher security therefore belongs to the host, repository and deployment system; repository code cannot prove those controls are active.

Before any public-input, publisher or pilot-data feature is enabled, record evidence for:

- MFA on repository, host and DNS/CDN accounts;
- named least-privilege deploy, data-refresh and editorial roles;
- an independent reviewer for affiliated or high-risk publication;
- active session/token revocation and a credential-loss drill;
- CSRF protection for every future state-changing browser route;
- outbound egress restricted to approved source hosts, with proxy/DNS enforcement in addition to application checks;
- response, decompression, redirect, retry and runtime limits;
- log redaction, retention, deletion and access control;
- CDN/cache purge authority for corrections and rights withdrawals;
- systemd sandbox settings and the production unit matching `deploy/`;
- restore and rollback evidence.

The collector library rejects credentials, non-HTTPS sources, non-allowlisted hosts, non-standard ports, private/link-local/metadata addresses, DNS rebinding to non-public addresses and unsafe redirects. Network-level egress rules are still required because application checks are defense in depth, not the only boundary.
