//! `unidpp-config` — the manifest CLI.
//!
//!   unidpp-config validate <manifest>       parse + semantic checks
//!   unidpp-config render-env <svc> <manifest>  the service's env block
//!   unidpp-config services <manifest>       the deployment's services

use std::process::ExitCode;

const USAGE: &str = "\
unidpp-config — the operator manifest tool

USAGE:
  unidpp-config validate <manifest>
  unidpp-config render-env <service> <manifest>
  unidpp-config services <manifest>

Secrets substitute from the environment (${VAR}); an unset reference
is an error. Unknown knobs are rejected.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let run = match args.first().map(String::as_str) {
        Some("validate") if args.len() == 2 => {
            let text = read(&args[1]);
            unidpp_config::load(&text).map(|m| {
                println!(
                    "valid: {} (profile {}, {} service(s): {:?})",
                    m.deployment.name,
                    m.deployment.profile,
                    m.service_names().len(),
                    m.service_names()
                );
            })
        }
        Some("render-env") if args.len() == 3 => {
            let text = read(&args[2]);
            unidpp_config::load(&text)
                .and_then(|m| unidpp_config::render_env(&m, &args[1]))
                .map(|env| println!("{env}"))
        }
        Some("schema") if args.len() == 1 => {
            println!(
                "{}",
                serde_json::to_string_pretty(&unidpp_config::OperatorManifest::json_schema())
                    .expect("the schema is serde data")
            );
            Ok(())
        }
        Some("services") if args.len() == 2 => {
            let text = read(&args[1]);
            unidpp_config::load(&text).map(|m| println!("{}", m.service_names().join("\n")))
        }
        _ => Err(unidpp_config::ConfigError(USAGE.to_string())),
    };
    match run {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("unidpp-config: {e}");
            ExitCode::FAILURE
        }
    }
}

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("unidpp-config: cannot read {path}: {e}");
        std::process::exit(2);
    })
}
