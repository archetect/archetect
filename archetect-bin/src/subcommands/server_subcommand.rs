use std::path::PathBuf;

use clap::parser::ValueSource;
use clap::ArgMatches;

use archetect_core::configuration::ConfigurationServerSection;
use archetect_core::errors::ArchetectError;
use archetect_core::server::{ArchetectServer, ArchetectServiceCore, TlsConfig};
use archetect_core::Archetect;

pub fn handle_server_subcommand(
    args: &ArgMatches,
    archetect: Archetect,
) -> Result<(), ArchetectError> {
    // Config provides defaults; CLI flags override. `value_source` lets us
    // tell "user supplied this" from "clap filled in the default."
    let server_cfg = archetect.configuration().server().cloned();
    let host = resolve_host(args, server_cfg.as_ref());
    let port = resolve_port(args, server_cfg.as_ref());
    let tls = resolve_tls_config(args, server_cfg.as_ref())?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|err| {
            ArchetectError::ServerError(format!("Failed to start Tokio runtime: {}", err))
        })?;

    runtime
        .block_on(async {
            let core = ArchetectServiceCore::builder(archetect).build().await?;
            let mut builder = ArchetectServer::builder(core)
                .with_host(host)
                .with_port(port);
            if let Some(tls) = tls {
                builder = builder.with_tls(tls);
            }
            let server = builder.build().await?;

            // Graceful shutdown on Ctrl-C: signal the server, let it drain
            // in-flight streams, then wait for serve() to return. Previously
            // we dropped out of tokio::select! on the signal, which aborted
            // the serve task and any active renders went with it.
            let server_for_signal = server.clone();
            let signal_task = tokio::spawn(async move {
                if tokio::signal::ctrl_c().await.is_ok() {
                    log::info!("Ctrl-C received — initiating graceful shutdown");
                    server_for_signal.shutdown().await;
                }
            });

            let result = server.serve().await;
            signal_task.abort();
            result
        })
        .map_err(|err| ArchetectError::ServerError(err.to_string()))?;

    Ok(())
}

fn cli_explicit(args: &ArgMatches, id: &str) -> bool {
    matches!(
        args.value_source(id),
        Some(ValueSource::CommandLine | ValueSource::EnvVariable)
    )
}

fn resolve_host(args: &ArgMatches, cfg: Option<&ConfigurationServerSection>) -> String {
    if cli_explicit(args, "host") {
        return args
            .get_one::<String>("host")
            .expect("has value")
            .to_string();
    }
    if let Some(host) = cfg.and_then(|c| c.host()) {
        return host.to_string();
    }
    args.get_one::<String>("host")
        .expect("has default")
        .to_string()
}

fn resolve_port(args: &ArgMatches, cfg: Option<&ConfigurationServerSection>) -> u16 {
    if cli_explicit(args, "port") {
        return *args.get_one::<u16>("port").expect("has value");
    }
    if let Some(port) = cfg.and_then(|c| c.port()) {
        return port;
    }
    *args.get_one::<u16>("port").expect("has default")
}

/// --tls-cert + --tls-key together enable TLS. Config sub-section does the
/// same. Either a CLI pair, a config section, or neither — anything else
/// is a misconfiguration. Config values are silently ignored for an
/// individual field when the matching CLI flag is explicitly supplied.
fn resolve_tls_config(
    args: &ArgMatches,
    cfg: Option<&ConfigurationServerSection>,
) -> Result<Option<TlsConfig>, ArchetectError> {
    let cli_cert = args.get_one::<String>("tls-cert").map(PathBuf::from);
    let cli_key = args.get_one::<String>("tls-key").map(PathBuf::from);
    let cli_ca = args.get_one::<String>("tls-client-ca").map(PathBuf::from);

    // Decide effective values: CLI > config > None.
    let cfg_tls = cfg.and_then(|c| c.tls());
    let effective_cert = cli_cert.or_else(|| cfg_tls.map(|t| t.cert().clone()));
    let effective_key = cli_key.or_else(|| cfg_tls.map(|t| t.key().clone()));
    let effective_ca = cli_ca.or_else(|| cfg_tls.and_then(|t| t.client_ca().cloned()));

    match (effective_cert, effective_key) {
        (Some(cert_path), Some(key_path)) => Ok(Some(TlsConfig {
            cert_path,
            key_path,
            client_ca_path: effective_ca,
        })),
        (None, None) => {
            if effective_ca.is_some() {
                Err(ArchetectError::ServerError(
                    "client CA requires a server cert + key (via --tls-cert/--tls-key or server.tls in config)"
                        .to_string(),
                ))
            } else {
                Ok(None)
            }
        }
        _ => Err(ArchetectError::ServerError(
            "TLS cert and key must be supplied together (CLI or config)".to_string(),
        )),
    }
}
