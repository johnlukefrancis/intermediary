// Path: crates/im_zip/src/bin/im_zip_cli.rs
// Description: CLI entry point for im_zip - accepts plan file, outputs progress JSON

use std::env;
use std::path::Path;
use std::process::ExitCode;

use im_zip::plan::ZipPlan;
use im_zip::writer::write_zip;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: im_zip_cli <plan-file>");
        eprintln!();
        eprintln!("Reads a JSON plan file and creates a zip archive.");
        eprintln!("Progress is emitted to stdout as JSON lines.");
        return ExitCode::from(1);
    }

    let plan_path = Path::new(&args[1]);

    match run(plan_path) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(1)
        }
    }
}

fn run(plan_path: &Path) -> Result<(), im_zip::error::ZipError> {
    let plan = ZipPlan::load(plan_path)?;
    write_zip(&plan)?;
    Ok(())
}
