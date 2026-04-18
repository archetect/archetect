use std::path::PathBuf;
use std::time::Duration;

use clap::parser::ValueSource;
use clap::ArgMatches;

use archetect_core::client::{ClientOptions, ClientTlsOptions};
use archetect_core::configuration::{
    ConfigurationClientSection, ConfigurationClientTlsSection,
};
use archetect_core::errors::ArchetectError;

/// Resolve the target endpoint: CLI positional wins, then `client.endpoint`
/// in config, then error.
pub fn resolve_endpoint(
    args: &ArgMatches,
    cfg: Option<&ConfigurationClientSection>,
) -> Result<String, ArchetectError> {
    if let Some(endpoint) = args.get_one::<String>("endpoint") {
        return Ok(endpoint.clone());
    }
    if let Some(endpoint) = cfg.and_then(|c| c.endpoint()) {
        return Ok(endpoint.to_string());
    }
    Err(ArchetectError::GeneralError(
        "No endpoint supplied. Provide one as a positional argument or set client.endpoint in archetect.yaml."
            .to_string(),
    ))
}

/// Build a `ClientOptions` by layering CLI flags over config values over
/// library defaults. `value_source` lets us tell user-provided CLI values
/// apart from clap's default fill-in.
pub fn resolve_client_options(
    args: &ArgMatches,
    cfg: Option<&ConfigurationClientSection>,
) -> ClientOptions {
    let mut options = ClientOptions::default();

    // Connect section
    if let Some(connect) = cfg.and_then(|c| c.connect()) {
        if let Some(secs) = connect.timeout_secs() {
            options.connect_timeout = Duration::from_secs(secs);
        }
        if let Some(retries) = connect.retries() {
            options.max_connect_retries = retries;
        }
        if let Some(ms) = connect.backoff_base_ms() {
            options.connect_backoff_base = Duration::from_millis(ms);
        }
        if let Some(secs) = connect.max_backoff_secs() {
            options.max_backoff = Duration::from_secs(secs);
        }
    }

    // CLI overrides for connect timeouts/retries
    if cli_explicit(args, "connect-timeout") {
        if let Some(secs) = args.get_one::<u64>("connect-timeout") {
            options.connect_timeout = Duration::from_secs(*secs);
        }
    }
    if cli_explicit(args, "connect-retries") {
        if let Some(retries) = args.get_one::<u32>("connect-retries") {
            options.max_connect_retries = *retries;
        }
    }

    // Keepalive section
    if let Some(ka) = cfg.and_then(|c| c.keepalive()) {
        if let Some(secs) = ka.interval_secs() {
            options.http2_keepalive_interval = if secs == 0 {
                None
            } else {
                Some(Duration::from_secs(secs))
            };
        }
        if let Some(secs) = ka.timeout_secs() {
            options.http2_keepalive_timeout = Some(Duration::from_secs(secs));
        }
    }

    options.tls = resolve_client_tls(args, cfg);
    options
}

fn cli_explicit(args: &ArgMatches, id: &str) -> bool {
    matches!(
        args.value_source(id),
        Some(ValueSource::CommandLine | ValueSource::EnvVariable)
    )
}

/// TLS is enabled when EITHER a CLI --tls (or any --tls-* flag) is supplied
/// OR the config has a `client.tls` section present (even empty).
fn resolve_client_tls(
    args: &ArgMatches,
    cfg: Option<&ConfigurationClientSection>,
) -> Option<ClientTlsOptions> {
    let cli_explicit_flag = args.get_flag("tls");
    let cli_ca = args.get_one::<String>("tls-ca").map(PathBuf::from);
    let cli_cert = args
        .get_one::<String>("tls-client-cert")
        .map(PathBuf::from);
    let cli_key = args
        .get_one::<String>("tls-client-key")
        .map(PathBuf::from);
    let cli_domain = args.get_one::<String>("tls-domain").cloned();

    let cfg_tls: Option<&ConfigurationClientTlsSection> = cfg.and_then(|c| c.tls());

    let any_cli_tls =
        cli_explicit_flag || cli_ca.is_some() || cli_cert.is_some() || cli_key.is_some() || cli_domain.is_some();
    if !any_cli_tls && cfg_tls.is_none() {
        return None;
    }

    // CLI values win per field; config fills the rest.
    let ca_cert_path = cli_ca.or_else(|| cfg_tls.and_then(|t| t.ca().cloned()));
    let client_cert_path = cli_cert.or_else(|| cfg_tls.and_then(|t| t.client_cert().cloned()));
    let client_key_path = cli_key.or_else(|| cfg_tls.and_then(|t| t.client_key().cloned()));
    let domain_name = cli_domain.or_else(|| cfg_tls.and_then(|t| t.domain().map(String::from)));

    Some(ClientTlsOptions {
        ca_cert_path,
        client_cert_path,
        client_key_path,
        domain_name,
    })
}
