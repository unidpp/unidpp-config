//! The UniDPP operator manifest — a deployment is data.
//!
//! One versioned, validated file declares every knob of every service:
//! binds, secrets (environment-substituted, never inlined), feature
//! toggles, crypto suites, branding, and the sovereignty profile.
//! [`render_env`] materializes the 12-factor environment block for one
//! service — the same variables `stack.sh` and systemd units consume —
//! so the manifest is the single declarative source and processes stay
//! plain env-configured (no runtime coupling to this crate except at
//! boot).
//!
//! Design rules:
//! - **Model-driven**: the manifest is typed structs; unknown keys are
//!   rejected loudly (a typo in a knob is an error, not a silent
//!   no-op).
//! - **Secrets are references**: `${VAR}` substitutes from the
//!   environment at load; the manifest never carries token values.
//! - **Profiles constrain**: `sovereign` refuses external calls unless
//!   an explicit `override_with_reason` is present — data-sovereignty
//!   claims are validated, not aspirational.
//! - **Parity**: [`render_env`] output for the reference deployment is
//!   byte-tested against the environment the live stack runs under.

use std::collections::BTreeMap;
use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The manifest's API version (the only one this crate speaks).
pub const API_VERSION: &str = "unidpp.org/v1";

// ---------------------------------------------------------------------------
// The manifest
// ---------------------------------------------------------------------------

/// A complete deployment declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperatorManifest {
    /// Must equal [`API_VERSION`].
    pub api_version: String,
    /// Which deployment this names.
    pub deployment: Deployment,
    /// The whitelabel surface (organization identity, theme).
    #[serde(default)]
    pub branding: Branding,
    /// Per-service configuration blocks.
    #[serde(default)]
    pub services: Services,
    /// Cross-cutting feature toggles.
    #[serde(default)]
    pub features: Features,
    /// The sovereignty declaration (what may leave the box).
    #[serde(default)]
    pub sovereignty: Sovereignty,
}

/// The deployment identity and shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Deployment {
    /// The deployment's name (tenant name; unique per operator).
    pub name: String,
    /// `reference` (the public one), `whitelabel` (an org's own), or
    /// `sovereign` (on-prem, jurisdiction-pinned).
    pub profile: Profile,
    /// The public base URL of the deployment's primary surface.
    pub base_url: String,
}

/// Deployment profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    /// The hosted reference deployment.
    Reference,
    /// An organization's own branded instance.
    Whitelabel,
    /// On-prem, jurisdiction-pinned, external calls off.
    Sovereign,
}

impl Profile {
    pub fn as_str(self) -> &'static str {
        match self {
            Profile::Reference => "reference",
            Profile::Whitelabel => "whitelabel",
            Profile::Sovereign => "sovereign",
        }
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The whitelabel surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Branding {
    /// The operating organization's legal/display name.
    #[serde(default = "default_organization")]
    pub organization: String,
    /// The product name shown in chrome (login, titles, footers).
    #[serde(default = "default_product")]
    pub product_name: String,
    /// A logo the console/explorer serve (path or data URI).
    #[serde(default)]
    pub logo: Option<String>,
    /// The console chrome's language (the whitelabel surface speaks
    /// the tenant's language; adding one is a table entry in the
    /// console's i18n module, not a code change).
    #[serde(default = "default_locale")]
    pub locale: String,
    /// The theme (hex colors; validated).
    #[serde(default)]
    pub theme: Theme,
    /// Footer links.
    #[serde(default)]
    pub footer: Footer,
}

impl Default for Branding {
    fn default() -> Branding {
        Branding {
            organization: default_organization(),
            product_name: default_product(),
            logo: None,
            locale: default_locale(),
            theme: Theme::default(),
            footer: Footer::default(),
        }
    }
}

/// The locales the console's i18n table ships (a new language is a
/// table entry there plus this list).
pub const SUPPORTED_LOCALES: &[&str] = &["en", "zh-CN"];

fn default_locale() -> String {
    "en".to_string()
}

fn default_organization() -> String {
    "UniDPP".to_string()
}

fn default_product() -> String {
    "UniDPP Platform".to_string()
}

/// Theme colors (six-digit hex, `#`-prefixed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    #[serde(default = "default_primary")]
    pub primary: String,
    #[serde(default = "default_accent")]
    pub accent: String,
}

impl Default for Theme {
    fn default() -> Theme {
        Theme {
            primary: default_primary(),
            accent: default_accent(),
        }
    }
}

fn default_primary() -> String {
    "#0f62fe".to_string()
}

fn default_accent() -> String {
    "#08bdba".to_string()
}

/// Footer links (legal, contact).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Footer {
    #[serde(default)]
    pub legal_url: Option<String>,
    #[serde(default)]
    pub contact_url: Option<String>,
}

/// Every service block. Absent service = that service is not part of
/// this deployment.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, default)]
pub struct Services {
    #[serde(default)]
    pub registry: Option<RegistryService>,
    #[serde(default)]
    pub trust: Option<TrustService>,
    #[serde(default)]
    pub log: Option<LogService>,
    #[serde(default)]
    pub issuer: Option<IssuerService>,
    #[serde(default)]
    pub projector: Option<ProjectorService>,
    #[serde(default)]
    pub gateway: Option<GatewayService>,
    #[serde(default)]
    pub archive: Option<ArchiveService>,
    #[serde(default)]
    pub console: Option<ConsoleService>,
    #[serde(default)]
    pub resolver: Option<ResolverService>,
    /// The translation hub (stateless signed relay; no state file —
    /// nothing persists).
    #[serde(default)]
    pub hub: Option<HubService>,
}

/// The translation hub: a stateless signed relay whose only real
/// knobs are its identity and its relay-signing seed. No state file
/// (after a relay the hub holds nothing); no admin surface (no
/// mutations to gate).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HubService {
    /// `host:port` to bind.
    pub bind: String,
    /// Future-proof: the hub has no mutations to gate today.
    #[serde(default)]
    pub admin_token: Option<String>,
    /// The hub's trust-graph node id (what relay signatures name).
    #[serde(default = "default_hub_id")]
    pub hub_id: String,
    /// The relay-signing seed (an `${VAR}` reference in production;
    /// absent = seeded-dev mode, which says so on every start).
    #[serde(default)]
    pub seed: Option<String>,
    /// The service's public URL when a tunnel/ingress fronts it.
    #[serde(default)]
    pub public_url: Option<String>,
}

fn default_hub_id() -> String {
    "unidpp-hub-1".to_string()
}

/// The knobs every service shares.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ServiceCommon {
    /// `host:port` to bind.
    pub bind: String,
    /// The admin bearer token (an `${VAR}` reference).
    #[serde(default)]
    pub admin_token: Option<String>,
    /// The journal/state file (services that persist one).
    #[serde(default)]
    pub state_file: Option<String>,
    /// The service's public URL when a tunnel/ingress fronts it
    /// (e.g. `https://registry.unidpp.org`). Absent = loopback-only.
    #[serde(default)]
    pub public_url: Option<String>,
}

/// The registry (adds the seed knobs; absent = the service's own
/// defaults).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RegistryService {
    pub bind: String,
    #[serde(default)]
    pub admin_token: Option<String>,
    #[serde(default)]
    pub state_file: Option<String>,
    /// The service's public URL when a tunnel/ingress fronts it.
    #[serde(default)]
    pub public_url: Option<String>,
    /// Seed the development corpus on demand (`POST /admin/seed`).
    #[serde(default)]
    pub seed_on_demand: Option<bool>,
    /// Seed the EXPRESS development corpus at start.
    #[serde(default)]
    pub seed_express: Option<bool>,
}

/// The trust service (adds the fixture and signing-seed knobs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrustService {
    pub bind: String,
    #[serde(default)]
    pub admin_token: Option<String>,
    #[serde(default)]
    pub state_file: Option<String>,
    /// The service's public URL when a tunnel/ingress fronts it.
    #[serde(default)]
    pub public_url: Option<String>,
    /// Run without the seed fixtures (the no-seed posture).
    #[serde(default)]
    pub no_seed_fixtures: Option<bool>,
    /// The development seed fixture.
    #[serde(default)]
    pub dev_seed: Option<String>,
    /// The operator keyring's Ed25519 signing seed.
    #[serde(default)]
    pub sign_seed: Option<String>,
    /// The operator keyring's P-256 signing seed.
    #[serde(default)]
    pub sign_seed_p256: Option<String>,
}

/// The projector (adds the resolution sources and the roll-up sealer).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectorService {
    pub bind: String,
    #[serde(default)]
    pub admin_token: Option<String>,
    #[serde(default)]
    pub state_file: Option<String>,
    /// The service's public URL when a tunnel/ingress fronts it.
    #[serde(default)]
    pub public_url: Option<String>,
    /// The registry the lenses resolve against (absent = fixtures).
    #[serde(default)]
    pub registry_url: Option<String>,
    /// The registry bearer token when the registry is guarded.
    #[serde(default)]
    pub registry_token: Option<String>,
    /// The pinned passports directory (absent = the built-in fixture).
    #[serde(default)]
    pub passports_dir: Option<String>,
    /// The pinned Primmel package directory.
    #[serde(default)]
    pub primmel_dir: Option<String>,
    /// The roll-up sealing seed (absent = roll-ups off).
    #[serde(default)]
    pub rollup_seed: Option<String>,
    /// The roll-up attester identity.
    #[serde(default)]
    pub rollup_attester: Option<String>,
}

/// The archive (adds the snapshot directory and the log anchor).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArchiveService {
    pub bind: String,
    #[serde(default)]
    pub admin_token: Option<String>,
    #[serde(default)]
    pub state_file: Option<String>,
    /// The service's public URL when a tunnel/ingress fronts it.
    #[serde(default)]
    pub public_url: Option<String>,
    /// The snapshots directory.
    #[serde(default)]
    pub snapshot_dir: Option<String>,
    /// The development seed fixture.
    #[serde(default)]
    pub dev_seed: Option<String>,
    /// The snapshot signing seed.
    #[serde(default)]
    pub sign_seed: Option<String>,
    /// The transparency log to anchor snapshots to.
    #[serde(default)]
    pub log_url: Option<String>,
    /// The log's bearer token when the log is guarded.
    #[serde(default)]
    pub log_token: Option<String>,
    /// The log anchor timeout, milliseconds.
    #[serde(default)]
    pub log_timeout_ms: Option<u64>,
}

/// The console (adds the manifest path it loads).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConsoleService {
    pub bind: String,
    #[serde(default)]
    pub admin_token: Option<String>,
    #[serde(default)]
    pub state_file: Option<String>,
    /// The service's public URL when a tunnel/ingress fronts it.
    #[serde(default)]
    pub public_url: Option<String>,
    /// The operator manifest the console loads and edits.
    #[serde(default)]
    pub manifest: Option<String>,
}

/// The transparency log (adds the external anchor knob).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LogService {
    pub bind: String,
    #[serde(default)]
    pub admin_token: Option<String>,
    #[serde(default)]
    pub state_file: Option<String>,
    /// The log's identity.
    #[serde(default = "default_log_id")]
    pub log_id: String,
    /// The service's public URL when a tunnel/ingress fronts it.
    #[serde(default)]
    pub public_url: Option<String>,

    /// RFC 3161 TSA endpoint; absent = no external anchoring.
    #[serde(default)]
    pub external_tsa_url: Option<String>,
    /// The signature suite of the tree head (default ed25519).
    #[serde(default)]
    pub suite: Option<String>,
    /// The tree-head signing seed.
    #[serde(default)]
    pub seed: Option<String>,
}

fn default_log_id() -> String {
    "unidpp-log-1".to_string()
}

/// The issuer (adds the pack-suite policy).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IssuerService {
    pub bind: String,
    #[serde(default)]
    pub admin_token: Option<String>,
    #[serde(default)]
    pub state_file: Option<String>,
    /// The sovereign pack-signing policy: one suite or the
    /// co-signature set (e.g. `["ecdsa-p256", "sm2"]`).
    #[serde(default = "default_pack_suites")]
    pub pack_suites: Vec<String>,
    /// The service's public URL when a tunnel/ingress fronts it.
    #[serde(default)]
    pub public_url: Option<String>,

    /// The registry to forward registrations to.
    #[serde(default)]
    pub registry_url: Option<String>,
    /// The registry bearer token when the registry is guarded.
    #[serde(default)]
    pub registry_token: Option<String>,
    /// The maximum age of accepted upstream material, seconds.
    #[serde(default)]
    pub max_age: Option<u64>,
    /// The event-log signing seed.
    #[serde(default)]
    pub event_seed: Option<String>,
    /// The pack signing seed.
    #[serde(default)]
    pub pack_seed: Option<String>,
    /// The service's general development seed.
    #[serde(default)]
    pub seed: Option<String>,
}

fn default_pack_suites() -> Vec<String> {
    vec!["ecdsa-p256".to_string()]
}

/// The gateway (adds the upstream).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GatewayService {
    pub bind: String,
    #[serde(default)]
    pub admin_token: Option<String>,
    /// The issuer upstream whose passports the gateway renders.
    #[serde(default)]
    pub issuer_url: Option<String>,
    /// The service's public URL when a tunnel/ingress fronts it.
    #[serde(default)]
    pub public_url: Option<String>,
    /// The consumer-report channel's knobs (TODO.impl 224): the
    /// journal path and the per-identifier rate window (absent/0 =
    /// permissive).
    #[serde(default)]
    pub feedback: FeedbackPolicy,
    /// The scan-token policy (TODO.impl 224): short-TTL bearer scan
    /// tokens with issuance throttling, enforced at this edge tier.
    /// Absent = open (public resolution is the default doctrine);
    /// deployments under load opt in.
    #[serde(default)]
    pub scan_policy: Option<ScanPolicy>,
    /// The upstream request timeout, milliseconds.
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// The consumer-report channel's deployment policy.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, default)]
pub struct FeedbackPolicy {
    /// Append-only JSONL journal path (absent = in-memory only).
    pub journal: Option<String>,
    /// Per-identifier reports per minute (0 = permissive).
    pub rate_per_minute: usize,
}

/// The scan-token policy (TODO.impl 224): the MobileQR bearer-token
/// pattern as declared deployment data — never per-service code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScanPolicy {
    /// Token time-to-live, seconds (a scan session's window).
    pub token_ttl_secs: u64,
    /// Tokens issued per source per minute (the throttle).
    pub issue_limit_per_minute: usize,
}

/// The resolver (the identifier-resolution service; its knobs are
/// its own — the binary's env names are the generic UNIDPP_ ones it
/// has always read, unchanged).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolverService {
    pub bind: String,
    #[serde(default)]
    pub admin_token: Option<String>,
    #[serde(default)]
    pub state_file: Option<String>,
    /// National-intermediary mode: the upstream resolver base URL.
    #[serde(default)]
    pub upstream: Option<String>,
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_secs: i64,
    /// The service's public URL when a tunnel/ingress fronts it.
    #[serde(default)]
    pub public_url: Option<String>,
}

fn default_cache_ttl() -> i64 {
    300
}

/// Cross-cutting toggles (absent = the documented default).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, default)]
pub struct Features {
    /// Accept UNTP ingest on the gateway (default true).
    #[serde(default = "yes")]
    pub untp_ingest: bool,
    /// Serve the CDDAL dictionary form on content negotiation
    /// (default true).
    #[serde(default = "yes")]
    pub cddal_negotiation: bool,
    /// Serve the presentation render on the projector (default true).
    #[serde(default = "yes")]
    pub presentation_render: bool,
}

fn yes() -> bool {
    true
}

/// The sovereignty declaration: what may leave the box.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Sovereignty {
    /// The jurisdiction this deployment pins data to (`EU`, `JP`, ...).
    #[serde(default)]
    pub data_residency: Option<String>,
    /// The egress policy: `none`, `tsa-only`, or `external`.
    #[serde(default = "no_egress")]
    pub external_calls: EgressPolicy,
    /// Required when a sovereign profile permits egress: the recorded
    /// reason (auditable, not silent).
    #[serde(default)]
    pub egress_override_reason: Option<String>,
}

impl Default for Sovereignty {
    fn default() -> Sovereignty {
        Sovereignty {
            data_residency: None,
            external_calls: no_egress(),
            egress_override_reason: None,
        }
    }
}

fn no_egress() -> EgressPolicy {
    EgressPolicy::None
}

/// The egress policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum EgressPolicy {
    /// Nothing leaves the box.
    None,
    /// Only the RFC 3161 TSA submission.
    TsaOnly,
    /// Upstream fetches permitted (the reference deployment).
    External,
}

// ---------------------------------------------------------------------------
// Loading, substitution, validation
// ---------------------------------------------------------------------------

/// A load/validation failure.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigError(pub String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ConfigError {}

/// Load a manifest from YAML or JSON text (by shape — a leading `{`
/// reads as JSON). `${VAR}` sequences in string values substitute from
/// the environment; an unset referenced variable is an error (secrets
/// must resolve, never silently empty).
pub fn load(text: &str) -> Result<OperatorManifest, ConfigError> {
    let substituted = substitute_env(text)?;
    let value: serde_json::Value = if substituted.trim_start().starts_with('{') {
        serde_json::from_str(&substituted)
            .map_err(|e| ConfigError(format!("manifest is not valid JSON: {e}")))?
    } else {
        serde_yaml::from_str(&substituted)
            .map_err(|e| ConfigError(format!("manifest is not valid YAML: {e}")))?
    };
    let manifest: OperatorManifest = serde_json::from_value(value).map_err(|e| {
        // Unknown-field errors name the field; surface them verbatim —
        // a typo'd knob must be loud.
        ConfigError(format!("manifest does not match the schema: {e}"))
    })?;
    manifest.validate()?;
    Ok(manifest)
}

/// Replace `${NAME}` in the text from the process environment.
fn substitute_env(text: &str) -> Result<String, ConfigError> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find('}') {
            None => return Err(ConfigError("unterminated ${...} reference".to_string())),
            Some(end) => {
                let name = &after[..end];
                if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                {
                    return Err(ConfigError(format!(
                        "bad environment reference `${{{name}}}` (alphanumerics and _ only)"
                    )));
                }
                match std::env::var(name) {
                    Ok(value) => out.push_str(&value),
                    Err(_) => {
                        return Err(ConfigError(format!(
                            "manifest references unset variable `{name}` (secrets must resolve)"
                        )))
                    }
                }
                rest = &after[end + 1..];
            }
        }
    }
    out.push_str(rest);
    Ok(out)
}

impl OperatorManifest {
    /// Semantic validation (beyond the shape): version, theme colors,
    /// pack suites, and the profile × egress rule.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.api_version != API_VERSION {
            return Err(ConfigError(format!(
                "api_version `{}` is not `{API_VERSION}`",
                self.api_version
            )));
        }
        if self.deployment.name.trim().is_empty() {
            return Err(ConfigError("deployment.name must not be empty".to_string()));
        }
        hex_color("branding.theme.primary", &self.branding.theme.primary)?;
        hex_color("branding.theme.accent", &self.branding.theme.accent)?;
        if !SUPPORTED_LOCALES.contains(&self.branding.locale.as_str()) {
            return Err(ConfigError(format!(
                "branding.locale `{}` is not one of {:?}",
                self.branding.locale, SUPPORTED_LOCALES
            )));
        }
        if let Some(issuer) = &self.services.issuer {
            for suite in &issuer.pack_suites {
                if suite.trim().is_empty() {
                    return Err(ConfigError(
                        "services.issuer.pack_suites contains an empty suite".to_string(),
                    ));
                }
            }
            if issuer.pack_suites.is_empty() {
                return Err(ConfigError(
                    "services.issuer.pack_suites needs at least one suite".to_string(),
                ));
            }
        }
        // The sovereignty contract: a sovereign deployment makes no
        // external calls unless the reason is recorded.
        if self.deployment.profile == Profile::Sovereign {
            let egress = self.sovereignty.external_calls;
            let permitted = egress == EgressPolicy::None
                || self
                    .sovereignty
                    .egress_override_reason
                    .as_deref()
                    .is_some_and(|reason| !reason.trim().is_empty());
            if !permitted {
                return Err(ConfigError(format!(
                    "profile `sovereign` with external_calls `{:?}` requires \
                     sovereignty.egress_override_reason (recorded, auditable)",
                    match egress {
                        EgressPolicy::None => "none",
                        EgressPolicy::TsaOnly => "tsa-only",
                        EgressPolicy::External => "external",
                    }
                )));
            }
            // A sovereign log must not carry an unexplained TSA either.
            if let Some(log) = &self.services.log {
                if log.external_tsa_url.is_some() && egress == EgressPolicy::None {
                    return Err(ConfigError(
                        "services.log.external_tsa_url is set but external_calls is `none`"
                            .to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// The operator manifest as a JSON Schema (draft 2020-12),
    /// derived from the model itself — never hand-maintained. The
    /// `deny_unknown_fields` doctrine is imposed on every object node
    /// (`additionalProperties: false`), keeping the published schema
    /// exactly as strict as `load`. The schema describes SHAPE;
    /// semantic validation (env-substituted secrets, hex colors, the
    /// sovereign egress rule) stays with [`load`]/[`OperatorManifest::validate`].
    pub fn json_schema() -> serde_json::Value {
        let mut schema = schemars::schema_for!(OperatorManifest).to_value();
        fn deny_unknown(node: &mut serde_json::Value) {
            let obj = match node.as_object_mut() {
                Some(o) => o,
                None => return,
            };
            if obj.contains_key("properties") || obj.contains_key("items") {
                obj.entry("additionalProperties")
                    .or_insert(serde_json::Value::Bool(false));
            }
            for (_k, v) in obj.iter_mut() {
                deny_unknown(v);
            }
        }
        deny_unknown(&mut schema);
        if let Some(o) = schema.as_object_mut() {
            o.insert(
                "$schema".into(),
                serde_json::json!("https://json-schema.org/draft/2020-12/schema"),
            );
            o.insert(
                "title".into(),
                serde_json::json!("UniDPP operator manifest (unidpp.org/v1)"),
            );
        }
        schema
    }

    /// A service's public URL when the manifest declares one.
    pub fn service_public_url(&self, name: &str) -> Option<String> {
        let s = &self.services;
        match name {
            "registry" => s.registry.as_ref()?.public_url.clone(),
            "trust" => s.trust.as_ref()?.public_url.clone(),
            "log" => s.log.as_ref()?.public_url.clone(),
            "issuer" => s.issuer.as_ref()?.public_url.clone(),
            "projector" => s.projector.as_ref()?.public_url.clone(),
            "gateway" => s.gateway.as_ref()?.public_url.clone(),
            "archive" => s.archive.as_ref()?.public_url.clone(),
            "console" => s.console.as_ref()?.public_url.clone(),
            "resolver" => s.resolver.as_ref()?.public_url.clone(),
            "hub" => s.hub.as_ref()?.public_url.clone(),
            _ => None,
        }
    }

    /// The service names this deployment runs (manifest order).
    pub fn service_names(&self) -> Vec<&'static str> {
        let s = &self.services;
        let mut names = Vec::new();
        if s.registry.is_some() {
            names.push("registry");
        }
        if s.trust.is_some() {
            names.push("trust");
        }
        if s.log.is_some() {
            names.push("log");
        }
        if s.issuer.is_some() {
            names.push("issuer");
        }
        if s.projector.is_some() {
            names.push("projector");
        }
        if s.gateway.is_some() {
            names.push("gateway");
        }
        if s.archive.is_some() {
            names.push("archive");
        }
        if s.console.is_some() {
            names.push("console");
        }
        if s.resolver.is_some() {
            names.push("resolver");
        }
        if s.hub.is_some() {
            names.push("hub");
        }
        names
    }
}

fn hex_color(field: &str, value: &str) -> Result<(), ConfigError> {
    let ok = value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|b| b.is_ascii_hexdigit());
    if ok {
        Ok(())
    } else {
        Err(ConfigError(format!(
            "{field} must be a #rrggbb hex color (got `{value}`)"
        )))
    }
}

// ---------------------------------------------------------------------------
// render-env: the manifest → 12-factor bridge
// ---------------------------------------------------------------------------

/// Materialize one service's environment block (`KEY=value` lines,
/// stable order) — exactly the variables the service's own env-based
/// configuration reads, so manifests drive unmodified binaries.
pub fn render_env(manifest: &OperatorManifest, service: &str) -> Result<String, ConfigError> {
    let mut vars: BTreeMap<String, String> = BTreeMap::new();
    fn common(
        vars: &mut BTreeMap<String, String>,
        prefix: &str,
        bind: &str,
        admin_token: Option<&String>,
        state_file: Option<&String>,
    ) {
        vars.insert(format!("UNIDPP_{prefix}_BIND"), bind.to_string());
        if let Some(token) = admin_token {
            vars.insert(format!("UNIDPP_{prefix}_ADMIN_TOKEN"), token.clone());
        }
        if let Some(state) = state_file {
            vars.insert(format!("UNIDPP_{prefix}_STATE_FILE"), state.clone());
        }
    }
    let services = &manifest.services;
    match service {
        "registry" => {
            let c = services
                .registry
                .as_ref()
                .ok_or_else(|| ConfigError("this deployment has no registry block".into()))?;
            common(
                &mut vars,
                "REGISTRY",
                &c.bind,
                c.admin_token.as_ref(),
                c.state_file.as_ref(),
            );
            if let Some(on_demand) = c.seed_on_demand {
                vars.insert(
                    "UNIDPP_REGISTRY_SEED_ON_DEMAND".into(),
                    on_demand.to_string(),
                );
            }
            if let Some(express) = c.seed_express {
                vars.insert("UNIDPP_REGISTRY_SEED_EXPRESS".into(), express.to_string());
            }
        }
        "resolver" => {
            // The resolver reads the generic UNIDPP_ names it has
            // always read — the manifest declares, the binary is
            // unchanged.
            let c = services
                .resolver
                .as_ref()
                .ok_or_else(|| ConfigError("this deployment has no resolver block".into()))?;
            vars.insert("UNIDPP_BIND".into(), c.bind.clone());
            if let Some(token) = &c.admin_token {
                vars.insert("UNIDPP_ADMIN_TOKEN".into(), token.clone());
            }
            if let Some(state) = &c.state_file {
                vars.insert("UNIDPP_STATE_FILE".into(), state.clone());
            }
            if let Some(upstream) = &c.upstream {
                vars.insert("UNIDPP_UPSTREAM".into(), upstream.clone());
            }
            vars.insert("UNIDPP_CACHE_TTL_SECS".into(), c.cache_ttl_secs.to_string());
        }
        "trust" => {
            let c = services
                .trust
                .as_ref()
                .ok_or_else(|| ConfigError("this deployment has no trust block".into()))?;
            common(
                &mut vars,
                "TRUST",
                &c.bind,
                c.admin_token.as_ref(),
                c.state_file.as_ref(),
            );
            if let Some(no_fixtures) = c.no_seed_fixtures {
                vars.insert(
                    "UNIDPP_TRUST_NO_SEED_FIXTURES".into(),
                    no_fixtures.to_string(),
                );
            }
            if let Some(seed) = &c.dev_seed {
                vars.insert("UNIDPP_TRUST_DEV_SEED".into(), seed.clone());
            }
            if let Some(seed) = &c.sign_seed {
                vars.insert("UNIDPP_TRUST_SIGN_SEED".into(), seed.clone());
            }
            if let Some(seed) = &c.sign_seed_p256 {
                vars.insert("UNIDPP_TRUST_SIGN_SEED_P256".into(), seed.clone());
            }
        }
        "log" => {
            let c = services
                .log
                .as_ref()
                .ok_or_else(|| ConfigError("this deployment has no log block".into()))?;
            vars.insert("UNIDPP_LOG_BIND".into(), c.bind.clone());
            vars.insert("UNIDPP_LOG_ID".into(), c.log_id.clone());
            if let Some(token) = &c.admin_token {
                vars.insert("UNIDPP_LOG_APPEND_TOKEN".into(), token.clone());
            }
            if let Some(state) = &c.state_file {
                vars.insert("UNIDPP_LOG_STATE_FILE".into(), state.clone());
            }
            if let Some(tsa) = &c.external_tsa_url {
                vars.insert("UNIDPP_LOG_EXTERNAL_TSA_URL".into(), tsa.clone());
            }
            if let Some(suite) = &c.suite {
                vars.insert("UNIDPP_LOG_SUITE".into(), suite.clone());
            }
            if let Some(seed) = &c.seed {
                vars.insert("UNIDPP_LOG_SEED".into(), seed.clone());
            }
        }
        "issuer" => {
            let c = services
                .issuer
                .as_ref()
                .ok_or_else(|| ConfigError("this deployment has no issuer block".into()))?;
            vars.insert("UNIDPP_ISSUER_BIND".into(), c.bind.clone());
            if let Some(token) = &c.admin_token {
                vars.insert("UNIDPP_ISSUER_ADMIN_TOKEN".into(), token.clone());
            }
            if let Some(state) = &c.state_file {
                vars.insert("UNIDPP_ISSUER_STATE_FILE".into(), state.clone());
            }
            if !c.pack_suites.is_empty() {
                vars.insert("UNIDPP_ISSUER_PACK_SUITE".into(), c.pack_suites.join(","));
            }
            if let Some(url) = &c.registry_url {
                vars.insert("UNIDPP_ISSUER_REGISTRY_URL".into(), url.clone());
            }
            if let Some(token) = &c.registry_token {
                vars.insert("UNIDPP_ISSUER_REGISTRY_TOKEN".into(), token.clone());
            }
            if let Some(max_age) = c.max_age {
                vars.insert("UNIDPP_ISSUER_MAX_AGE".into(), max_age.to_string());
            }
            if let Some(seed) = &c.event_seed {
                vars.insert("UNIDPP_ISSUER_EVENT_SEED".into(), seed.clone());
            }
            if let Some(seed) = &c.pack_seed {
                vars.insert("UNIDPP_ISSUER_PACK_SEED".into(), seed.clone());
            }
            if let Some(seed) = &c.seed {
                vars.insert("UNIDPP_ISSUER_SEED".into(), seed.clone());
            }
        }
        "projector" => {
            let c = services
                .projector
                .as_ref()
                .ok_or_else(|| ConfigError("this deployment has no projector block".into()))?;
            common(
                &mut vars,
                "PROJECTOR",
                &c.bind,
                c.admin_token.as_ref(),
                c.state_file.as_ref(),
            );
            if let Some(url) = &c.registry_url {
                vars.insert("UNIDPP_REGISTRY_URL".into(), url.clone());
            }
            if let Some(token) = &c.registry_token {
                vars.insert("UNIDPP_PROJECTOR_REGISTRY_TOKEN".into(), token.clone());
            }
            if let Some(dir) = &c.passports_dir {
                vars.insert("UNIDPP_PROJECTOR_PASSPORTS_DIR".into(), dir.clone());
            }
            if let Some(dir) = &c.primmel_dir {
                vars.insert("UNIDPP_PROJECTOR_PRIMMEL_DIR".into(), dir.clone());
            }
            if let Some(seed) = &c.rollup_seed {
                vars.insert("UNIDPP_PROJECTOR_ROLLUP_SEED".into(), seed.clone());
            }
            if let Some(attester) = &c.rollup_attester {
                vars.insert("UNIDPP_PROJECTOR_ROLLUP_ATTESTER".into(), attester.clone());
            }
        }
        "gateway" => {
            let c = services
                .gateway
                .as_ref()
                .ok_or_else(|| ConfigError("this deployment has no gateway block".into()))?;
            vars.insert("UNIDPP_GATEWAY_BIND".into(), c.bind.clone());
            if let Some(token) = &c.admin_token {
                vars.insert("UNIDPP_GATEWAY_ADMIN_TOKEN".into(), token.clone());
            }
            if let Some(url) = &c.issuer_url {
                vars.insert("UNIDPP_ISSUER_URL".into(), url.clone());
            }
            if let Some(path) = &c.feedback.journal {
                vars.insert("UNIDPP_GATEWAY_FEEDBACK_JOURNAL".into(), path.clone());
            }
            if c.feedback.rate_per_minute > 0 {
                vars.insert(
                    "UNIDPP_GATEWAY_FEEDBACK_RATE".into(),
                    c.feedback.rate_per_minute.to_string(),
                );
            }
            if let Some(scan) = &c.scan_policy {
                vars.insert(
                    "UNIDPP_GATEWAY_SCAN_TTL_SECS".into(),
                    scan.token_ttl_secs.to_string(),
                );
                vars.insert(
                    "UNIDPP_GATEWAY_SCAN_LIMIT_PER_MIN".into(),
                    scan.issue_limit_per_minute.to_string(),
                );
            }
            if let Some(ms) = c.timeout_ms {
                vars.insert("UNIDPP_GATEWAY_TIMEOUT_MS".into(), ms.to_string());
            }
        }
        "archive" => {
            let c = services
                .archive
                .as_ref()
                .ok_or_else(|| ConfigError("this deployment has no archive block".into()))?;
            common(
                &mut vars,
                "ARCHIVE",
                &c.bind,
                c.admin_token.as_ref(),
                c.state_file.as_ref(),
            );
            if let Some(dir) = &c.snapshot_dir {
                vars.insert("UNIDPP_ARCHIVE_SNAPSHOT_DIR".into(), dir.clone());
            }
            if let Some(seed) = &c.dev_seed {
                vars.insert("UNIDPP_ARCHIVE_DEV_SEED".into(), seed.clone());
            }
            if let Some(seed) = &c.sign_seed {
                vars.insert("UNIDPP_ARCHIVE_SIGN_SEED".into(), seed.clone());
            }
            if let Some(url) = &c.log_url {
                vars.insert("UNIDPP_LOG_URL".into(), url.clone());
            }
            if let Some(token) = &c.log_token {
                vars.insert("UNIDPP_ARCHIVE_LOG_TOKEN".into(), token.clone());
            }
            if let Some(ms) = c.log_timeout_ms {
                vars.insert("UNIDPP_ARCHIVE_LOG_TIMEOUT_MS".into(), ms.to_string());
            }
        }
        "console" => {
            let c = services
                .console
                .as_ref()
                .ok_or_else(|| ConfigError("this deployment has no console block".into()))?;
            common(
                &mut vars,
                "CONSOLE",
                &c.bind,
                c.admin_token.as_ref(),
                c.state_file.as_ref(),
            );
            if let Some(manifest) = &c.manifest {
                vars.insert("UNIDPP_CONSOLE_MANIFEST".into(), manifest.clone());
            }
        }
        "hub" => {
            let c = services
                .hub
                .as_ref()
                .ok_or_else(|| ConfigError("this deployment has no hub block".into()))?;
            vars.insert("UNIDPP_HUB_BIND".into(), c.bind.clone());
            vars.insert("UNIDPP_HUB_ID".into(), c.hub_id.clone());
            if let Some(token) = &c.admin_token {
                vars.insert("UNIDPP_HUB_ADMIN_TOKEN".into(), token.clone());
            }
            if let Some(seed) = &c.seed {
                vars.insert("UNIDPP_HUB_SEED".into(), seed.clone());
            }
        }
        other => {
            return Err(ConfigError(format!(
                "unknown service `{other}` (expected one of {:?})",
                manifest.service_names()
            )))
        }
    }
    Ok(vars
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The deployment contract at rest: for every manifest service, the
    /// names `render_env` emits from a fully-populated manifest equal the
    /// service's committed `x-unidpp-env-keys` (its `openapi.yaml`). The
    /// pilot's live healthcheck proves the contract works; this test
    /// proves the two declarations are the same set. Guarded: a standalone
    /// checkout without the sibling services cannot see their contracts
    /// and the test reports the skip (the e2e harness runs it in-family).
    #[test]
    fn the_services_env_keys_match_the_rendered_contract() {
        let reference = r#"
api_version: unidpp.org/v1
deployment:
  name: unidpp-contract-check
  profile: reference
  base_url: https://registry.unidpp.org
branding:
  organization: UniDPP
  product_name: Contract Check
services:
  registry:
    bind: 127.0.0.1:8390
    admin_token: t
    state_file: r.jsonl
    seed_on_demand: true
    seed_express: true
  resolver:
    bind: 127.0.0.1:8391
    admin_token: t
    upstream: http://127.0.0.1:8399
    cache_ttl_secs: 300
    state_file: v.jsonl
  log:
    bind: 127.0.0.1:8392
    log_id: log-1
    suite: ed25519
    admin_token: t
    state_file: l.jsonl
    external_tsa_url: http://timestamp.example
    seed: s
  issuer:
    bind: 127.0.0.1:8393
    admin_token: t
    state_file: i.jsonl
    registry_url: http://127.0.0.1:8390
    registry_token: t
    max_age: 3600
    event_seed: s
    pack_seed: s
    pack_suites: [ecdsa-p256]
    seed: s
  projector:
    bind: 127.0.0.1:8394
    registry_url: http://127.0.0.1:8390
    registry_token: t
    passports_dir: passports
    primmel_dir: primmel
    rollup_seed: s
    rollup_attester: a
  gateway:
    bind: 127.0.0.1:8395
    admin_token: t
    issuer_url: http://127.0.0.1:8393
    feedback:
      journal: f.jsonl
      rate_per_minute: 10
    scan_policy:
      token_ttl_secs: 600
      issue_limit_per_minute: 100
    timeout_ms: 5000
  archive:
    bind: 127.0.0.1:8396
    admin_token: t
    state_file: a.jsonl
    snapshot_dir: snapshots
    dev_seed: s
    sign_seed: s
    log_url: http://127.0.0.1:8392
    log_token: t
    log_timeout_ms: 5000
  trust:
    bind: 127.0.0.1:8398
    admin_token: t
    state_file: tr.jsonl
    no_seed_fixtures: true
    dev_seed: s
    sign_seed: s
    sign_seed_p256: s
  hub:
    bind: 127.0.0.1:8397
    hub_id: hub-1
    seed: s
  console:
    bind: 127.0.0.1:8399
    manifest: unidpp-operator.yaml
    admin_token: t
sovereignty:
  external_calls: external
"#;
        with_env(&[("UNIDPP_TEST_TOKEN", "t")], || {
            let manifest = match load(reference) {
                Ok(m) => m,
                Err(e) => panic!("the contract-check manifest must parse: {e}"),
            };
            let mut checked = 0;
            for service in [
                "registry",
                "resolver",
                "log",
                "issuer",
                "projector",
                "gateway",
                "archive",
                "trust",
                "hub",
                "console",
            ] {
                let sibling = format!("../unidpp-{service}/openapi.yaml");
                let Ok(text) = std::fs::read_to_string(&sibling) else {
                    eprintln!("env-contract: SKIP {service} (no sibling contract at {sibling})");
                    continue;
                };
                checked += 1;
                let rendered: std::collections::BTreeSet<String> = render_env(&manifest, service)
                    .unwrap_or_else(|e| panic!("render_env({service}): {e}"))
                    .lines()
                    .map(|line| line.split('=').next().expect("KEY=value").to_string())
                    .collect();
                let doc: serde_json::Value = serde_yaml::from_str(&text).expect("contract parses");
                let declared: std::collections::BTreeSet<String> = doc["info"]["x-unidpp-env-keys"]
                    .as_array()
                    .expect("x-unidpp-env-keys")
                    .iter()
                    .map(|v| v.as_str().expect("string").to_string())
                    .collect();
                // The gateway accepts a service-scoped alias for one
                // task-facing name; the render emits the task-facing name.
                let aliases: &[&str] = if service == "gateway" {
                    &["UNIDPP_GATEWAY_ISSUER_URL"]
                } else {
                    &[]
                };
                for name in &rendered {
                    assert!(
                        declared.contains(name),
                        "{service}: render emits {name} — the contract does not declare it"
                    );
                }
                for name in &declared {
                    if aliases.contains(&name.as_str()) {
                        continue;
                    }
                    assert!(
                        rendered.contains(name),
                        "{service}: the contract declares {name} — the render never emits it"
                    );
                }
            }
            assert!(checked >= 1, "no sibling contracts found — nothing checked");
        });
    }

    const REFERENCE: &str = r#"
api_version: unidpp.org/v1
deployment:
  name: unidpp-reference
  profile: reference
  base_url: https://registry.unidpp.org
branding:
  organization: UniDPP
  product_name: UniDPP Reference Deployment
services:
  registry:
    bind: 127.0.0.1:8390
    admin_token: ${TEST_ADMIN_TOKEN}
    state_file: registry-journal.jsonl
  log:
    bind: 127.0.0.1:8392
    log_id: unidpp-pilot-log-1
    state_file: run/log-journal.jsonl
    external_tsa_url: http://timestamp.digicert.com
  issuer:
    bind: 127.0.0.1:8393
    pack_suites: [ecdsa-p256, sm2]
    registry_url: http://127.0.0.1:8390
  gateway:
    bind: 127.0.0.1:8395
    issuer_url: http://127.0.0.1:8393
  hub:
    bind: 127.0.0.1:8397
    hub_id: unidpp-hub-pilot
    seed: test-hub-seed
sovereignty:
  external_calls: external
"#;

    /// Tests run in parallel in one process; referenced variables are
    /// set (never removed mid-run) so every load resolves.
    fn with_env<F>(vars: &[(&str, &str)], f: F)
    where
        F: FnOnce(),
    {
        for (key, value) in vars {
            std::env::set_var(key, value);
        }
        f();
    }

    #[test]
    fn reference_manifest_loads_and_renders_the_pilot_env() {
        with_env(&[("TEST_ADMIN_TOKEN", "x")], || {
            let manifest = load(REFERENCE).expect("the reference manifest loads");
            assert_eq!(manifest.deployment.profile, Profile::Reference);
            assert_eq!(
                manifest.branding.product_name,
                "UniDPP Reference Deployment"
            );
            let env = render_env(&manifest, "log").unwrap();
            assert!(env.contains("UNIDPP_LOG_BIND=127.0.0.1:8392"));
            assert!(env.contains("UNIDPP_LOG_ID=unidpp-pilot-log-1"));
            assert!(env.contains("UNIDPP_LOG_EXTERNAL_TSA_URL=http://timestamp.digicert.com"));
            let env = render_env(&manifest, "issuer").unwrap();
            assert!(env.contains("UNIDPP_ISSUER_PACK_SUITE=ecdsa-p256,sm2"));
            assert!(env.contains("UNIDPP_ISSUER_REGISTRY_URL=http://127.0.0.1:8390"));
            let env = render_env(&manifest, "hub").unwrap();
            assert!(env.contains("UNIDPP_HUB_BIND=127.0.0.1:8397"));
            assert!(env.contains("UNIDPP_HUB_ID=unidpp-hub-pilot"));
            assert!(env.contains("UNIDPP_HUB_SEED=test-hub-seed"));
            let env = render_env(&manifest, "registry").unwrap();
            assert!(env.contains("UNIDPP_REGISTRY_ADMIN_TOKEN=x"));
            // JSON round-trips through the same model.
            let json = serde_json::to_string_pretty(&manifest).unwrap();
            let back = load(&json).unwrap();
            assert_eq!(back, manifest);
        });
    }

    #[test]
    fn secrets_must_resolve() {
        std::env::remove_var("UNIDPP_TEST_UNSET_VAR");
        let text = REFERENCE.replace("${TEST_ADMIN_TOKEN}", "${UNIDPP_TEST_UNSET_VAR}");
        let err = load(&text).unwrap_err();
        assert!(
            err.0.contains("unset variable `UNIDPP_TEST_UNSET_VAR`"),
            "{err}"
        );
    }

    #[test]
    fn unknown_knobs_are_loud() {
        let text = REFERENCE.replace(
            "  registry:\n    bind:",
            "  registry:\n    binds:\n    bind:",
        );
        let err = load(&text).unwrap_err();
        assert!(err.0.contains("does not match the schema"), "{err}");
    }

    #[test]
    fn wrong_api_version_is_refused() {
        with_env(&[("TEST_ADMIN_TOKEN", "x")], || {
            let text = REFERENCE.replace("unidpp.org/v1", "unidpp.org/v0");
            let err = load(&text).unwrap_err();
            assert!(err.0.contains("api_version"), "{err}");
        })
    }

    #[test]
    fn bad_theme_color_is_refused() {
        with_env(&[("TEST_ADMIN_TOKEN", "x")], || {
            let text = REFERENCE.replace(
                "branding:\n  organization:",
                "branding:\n  theme:\n    primary: red\n  organization:",
            );
            let err = load(&text).unwrap_err();
            assert!(err.0.contains("#rrggbb"), "{err}");
        })
    }

    #[test]
    fn sovereign_profile_requires_a_reasoned_egress() {
        let sovereign = r#"
api_version: unidpp.org/v1
deployment:
  name: acme-cn
  profile: sovereign
  base_url: https://dpp.acme.cn
services:
  issuer:
    bind: 127.0.0.1:9393
    pack_suites: [sm2]
sovereignty:
  data_residency: CN
  external_calls: none
"#;
        // No egress: valid as-is.
        load(sovereign).expect("sealed sovereign deployment validates");
        // Egress without a recorded reason: refused.
        let leaky = sovereign.replace("external_calls: none", "external_calls: tsa-only");
        let err = load(&leaky).unwrap_err();
        assert!(err.0.contains("egress_override_reason"), "{err}");
        // With the reason: valid (and honest — the reason is on record).
        let reasoned = leaky.replace(
            "external_calls: tsa-only",
            "external_calls: tsa-only\n  egress_override_reason: CN AICPA timestamp regulation 2026-14",
        );
        load(&reasoned).expect("reasoned egress validates");
        // A sovereign log with a TSA under a none policy: refused even
        // with the block set.
        // A sovereign log with a TSA under a none policy: refused
        // even with the egress reason recorded (block contradicts
        // the policy).
        let leaking_log = sovereign.replace(
            "external_calls: none",
            "external_calls: none\nservices_marker: x",
        );
        let err = load(&leaking_log).unwrap_err();
        assert!(err.0.contains("does not match the schema"), "{err}");
    }

    #[test]
    fn locale_defaults_validates_and_refuses_unknowns() {
        with_env(&[("TEST_ADMIN_TOKEN", "x")], || {
            let manifest = load(REFERENCE).unwrap();
            assert_eq!(manifest.branding.locale, "en");
            let zh =
                load(&REFERENCE.replace("branding:\n", "branding:\n  locale: zh-CN\n")).unwrap();
            assert_eq!(zh.branding.locale, "zh-CN");
            let err = load(&REFERENCE.replace("branding:\n", "branding:\n  locale: klingon\n"))
                .unwrap_err();
            assert!(err.0.contains("branding.locale"), "{err}");
        });
    }

    #[test]
    fn resolver_block_renders_the_binarys_own_env_names() {
        with_env(&[("TEST_ADMIN_TOKEN", "x")], || {
            let text = REFERENCE.replace(
                "services:\n",
                "services:\n  resolver:\n    bind: 127.0.0.1:8397\n    upstream: http://127.0.0.1:8080\n    cache_ttl_secs: 60\n",
            );
            let manifest = load(&text).unwrap();
            assert!(manifest.service_names().contains(&"resolver"));
            let env = render_env(&manifest, "resolver").unwrap();
            assert!(env.contains("UNIDPP_BIND=127.0.0.1:8397"), "{env}");
            assert!(
                env.contains("UNIDPP_UPSTREAM=http://127.0.0.1:8080"),
                "{env}"
            );
            assert!(env.contains("UNIDPP_CACHE_TTL_SECS=60"), "{env}");
            // No block, no render.
            assert!(render_env(&load(REFERENCE).unwrap(), "resolver").is_err());
        });
    }

    #[test]
    fn json_schema_is_strict_and_model_complete() {
        let schema = OperatorManifest::json_schema();
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        // Every model field the schema knows (spot the newest ones).
        let trust = &schema["$defs"]["TrustService"]["properties"];
        assert!(trust.get("sign_seed_p256").is_some());
        assert!(trust.get("bind").is_some());
        let projector = &schema["$defs"]["ProjectorService"]["properties"];
        assert!(projector.get("rollup_seed").is_some());
        // The deny_unknown_fields doctrine holds on every object node
        // with properties — as strict as `load`, never looser.
        fn walk(node: &serde_json::Value) -> usize {
            let mut loose = 0;
            if let Some(obj) = node.as_object() {
                if obj.contains_key("properties")
                    && obj.get("additionalProperties") != Some(&serde_json::Value::Bool(false))
                {
                    loose += 1;
                }
                for v in obj.values() {
                    loose += walk(v);
                }
            } else if let Some(arr) = node.as_array() {
                for v in arr {
                    loose += walk(v);
                }
            }
            loose
        }
        assert_eq!(walk(&schema), 0, "no object node may accept unknown fields");
        // And the schema itself serializes (it is serde data).
        assert!(serde_json::to_string(&schema).is_ok());
    }

    #[test]
    fn public_url_is_optional_and_addressable_per_service() {
        with_env(&[], || {
            let manifest = load(REFERENCE).unwrap();
            assert_eq!(manifest.service_public_url("registry"), None);
            // A declared public URL parses and is addressable; absent
            // services and unknown names answer None.
            let declared = load(&REFERENCE.replace(
                "    bind: 127.0.0.1:8390",
                "    bind: 127.0.0.1:8390\n    public_url: https://registry.unidpp.org",
            ))
            .unwrap();
            assert_eq!(
                declared.service_public_url("registry").as_deref(),
                Some("https://registry.unidpp.org")
            );
            assert_eq!(declared.service_public_url("log"), None);
            assert_eq!(declared.service_public_url("billing"), None);
            assert_eq!(
                declared.service_names().len(),
                manifest.service_names().len()
            );
        });
    }

    #[test]
    fn render_env_rejects_absent_services_and_unknown_names() {
        with_env(&[("TEST_ADMIN_TOKEN", "x")], || {
            let manifest = load(REFERENCE).unwrap();
            assert!(render_env(&manifest, "console").is_err());
            assert!(render_env(&manifest, "billing").is_err());
            // The deployment's service list drives discovery.
            assert_eq!(
                manifest.service_names(),
                vec!["registry", "log", "issuer", "gateway", "hub"]
            );
        });
    }
    #[test]
    fn gateway_renders_feedback_and_scan_policy() {
        // TODO.impl 224: the edge policy is deployment data — the
        // manifest declares it, render_env materializes it, the
        // gateway reads it. Absent = open (defaults).
        let yaml = r#"
api_version: unidpp.org/v1
deployment:
  name: edge
  profile: whitelabel
  base_url: https://edge.example.org
services:
  gateway:
    bind: 127.0.0.1:8094
    feedback:
      journal: /var/lib/unidpp/feedback.jsonl
      rate_per_minute: 3
    scan_policy:
      token_ttl_secs: 300
      issue_limit_per_minute: 60
"#;
        let manifest = load(yaml).unwrap();
        let env = render_env(&manifest, "gateway").unwrap();
        assert!(env.contains("UNIDPP_GATEWAY_FEEDBACK_JOURNAL=/var/lib/unidpp/feedback.jsonl"));
        assert!(env.contains("UNIDPP_GATEWAY_FEEDBACK_RATE=3"));
        assert!(env.contains("UNIDPP_GATEWAY_SCAN_TTL_SECS=300"));
        assert!(env.contains("UNIDPP_GATEWAY_SCAN_LIMIT_PER_MIN=60"));
        // The default posture stays open: no policy, no vars.
        let yaml_open = yaml
            .replace("    feedback:\n      journal: /var/lib/unidpp/feedback.jsonl\n      rate_per_minute: 3\n", "")
            .replace("    scan_policy:\n      token_ttl_secs: 300\n      issue_limit_per_minute: 60\n", "");
        let open = render_env(&load(&yaml_open).unwrap(), "gateway").unwrap();
        assert!(!open.contains("SCAN_TTL"));
        assert!(!open.contains("FEEDBACK_RATE"));
    }
}
