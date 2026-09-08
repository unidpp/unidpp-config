# unidpp-config

The operator manifest: **a deployment is data.** One versioned,
validated file declares every knob of every UniDPP service — binds,
secrets (environment-substituted `${VAR}`, never inlined), feature
toggles, crypto-suite policy, branding, and the sovereignty profile.

```yaml
api_version: unidpp.org/v1
deployment: { name: acme-cn, profile: sovereign, base_url: https://dpp.acme.cn }
branding:   { organization: ACME Mobility, product_name: ACME DPP }
services:
  issuer: { bind: 127.0.0.1:9393, pack_suites: [sm2] }
sovereignty: { data_residency: CN, external_calls: none }
```

```
$ unidpp-config validate deployment.yaml
valid: acme-cn (profile sovereign, 1 service(s): ["issuer"])
$ unidpp-config render-env issuer deployment.yaml
UNIDPP_ISSUER_BIND=127.0.0.1:9393
UNIDPP_ISSUER_PACK_SUITE=sm2
```

Rules the library enforces:

- **Unknown knobs are loud.** A typo is a schema error, not a silent
  no-op (`deny_unknown_fields` throughout).
- **Secrets are references.** `${VAR}` substitutes from the
  environment at load; an unset reference is an error.
- **Profiles constrain.** `profile: sovereign` with any egress beyond
  `none` refuses to validate unless `egress_override_reason` records
  why; a sovereign log carrying a TSA under a `none` policy is a
  contradiction the validator rejects.
- **Parity.** `render-env` emits exactly the `UNIDPP_*` variables the
  services' own env-based configuration reads — the reference
  manifest renders the live stack's environment byte-for-byte, so
  manifests drive unmodified binaries.

Also a library: `unidpp_config::load(text)` → the typed
`OperatorManifest` (with branding/theme accessors for whitelabel
surfaces and `service_names()` for orchestration).
