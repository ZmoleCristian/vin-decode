//! CSV → FST/bin build pipeline runner. Used by the monthly refresh-vpic CI.
//!
//! Usage: `vin-decode-build <csv_dir> <out_dir>`

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(csv_dir) = args.next() else {
        eprintln!("usage: vin-decode-build <csv_dir> <out_dir>");
        return ExitCode::from(2);
    };
    let Some(out_dir) = args.next() else {
        eprintln!("usage: vin-decode-build <csv_dir> <out_dir>");
        return ExitCode::from(2);
    };
    if let Err(e) =
        vin_decode::build::build_from_csv(&PathBuf::from(csv_dir), &PathBuf::from(out_dir))
    {
        eprintln!("build failed: {e}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
