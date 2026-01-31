// Path: crates/im_bundle/src/bin/im_bundle_cli.rs
// Description: CLI entry point for im_bundle - scans and writes bundle zip

use std::env;
use std::path::Path;
use std::process::ExitCode;

use im_bundle::plan::BundlePlan;
use im_bundle::writer::write_bundle;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: im_bundle_cli <plan-file>");
        eprintln!();
        eprintln!("Reads a JSON plan file, scans repo, and creates a zip archive.");
        eprintln!("Progress is emitted to stdout as NDJSON lines.");
        return ExitCode::from(1);
    }

    let plan_path = Path::new(&args[1]);

    match run(plan_path) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn run(plan_path: &Path) -> Result<(), im_bundle::error::BundleError> {
    let plan = BundlePlan::load(plan_path)?;
    write_bundle(&plan)?;
    Ok(())
}
