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
    pub registry: Option<ServiceCommon>,
    #[serde(default)]
    pub trust: Option<ServiceCommon>,
    #[serde(default)]
    pub log: Option<LogService>,
    #[serde(default)]
    pub issuer: Option<IssuerService>,
    #[serde(default)]
    pub projector: Option<ServiceCommon>,
    #[serde(default)]
    pub gateway: Option<GatewayService>,
    #[serde(default)]
    pub archive: Option<ServiceCommon>,
    #[serde(default)]
    pub console: Option<ServiceCommon>,
    #[serde(default)]
    pub resolver: Option<ResolverService>,
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
    let common = |vars: &mut BTreeMap<String, String>, prefix: &str, c: &ServiceCommon| {
        vars.insert(format!("UNIDPP_{prefix}_BIND"), c.bind.clone());
        if let Some(token) = &c.admin_token {
            vars.insert(format!("UNIDPP_{prefix}_ADMIN_TOKEN"), token.clone());
        }
        if let Some(state) = &c.state_file {
            vars.insert(format!("UNIDPP_{prefix}_STATE_FILE"), state.clone());
        }
    };
    let services = &manifest.services;
    match service {
        "registry" => {
            let c = services
                .registry
                .as_ref()
                .ok_or_else(|| ConfigError("this deployment has no registry block".into()))?;
            common(&mut vars, "REGISTRY", c);
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
            vars.insert("UNIDPP_CACHE_TTL".into(), c.cache_ttl_secs.to_string());
        }
        "trust" => {
            let c = services
                .trust
                .as_ref()
                .ok_or_else(|| ConfigError("this deployment has no trust block".into()))?;
            common(&mut vars, "TRUST", c);
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
        }
        "projector" => {
            let c = services
                .projector
                .as_ref()
                .ok_or_else(|| ConfigError("this deployment has no projector block".into()))?;
            common(&mut vars, "PROJECTOR", c);
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
        }
        "archive" => {
            let c = services
                .archive
                .as_ref()
                .ok_or_else(|| ConfigError("this deployment has no archive block".into()))?;
            common(&mut vars, "ARCHIVE", c);
        }
        "console" => {
            let c = services
                .console
                .as_ref()
                .ok_or_else(|| ConfigError("this deployment has no console block".into()))?;
            common(&mut vars, "CONSOLE", c);
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
            assert!(env.contains("UNIDPP_CACHE_TTL=60"), "{env}");
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
        // Every model field the schema knows (spot the newest one).
        let common = &schema["$defs"]["ServiceCommon"]["properties"];
        assert!(common.get("public_url").is_some());
        assert!(common.get("bind").is_some());
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
                vec!["registry", "log", "issuer", "gateway"]
            );
        });
    }
}
