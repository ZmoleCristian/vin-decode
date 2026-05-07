//! Offline rip → FST build pipeline runner.
//!
//! Consumes the merged TSVs (`wmi_merged.tsv`, `brands.tsv`,
//! `brand_models.tsv`, `engines.tsv`) produced by the `data-rip` scripts and
//! writes paired FST/rkyv files into the output directory.
//!
//! Usage: `vin-decode-rip-build <rip_dir> <out_dir>`

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(rip_dir) = args.next() else {
        eprintln!("usage: vin-decode-rip-build <rip_dir> <out_dir>");
        return ExitCode::from(2);
    };
    let Some(out_dir) = args.next() else {
        eprintln!("usage: vin-decode-rip-build <rip_dir> <out_dir>");
        return ExitCode::from(2);
    };
    if let Err(e) =
        vin_decode::build::build_from_rip(&PathBuf::from(rip_dir), &PathBuf::from(out_dir))
    {
        eprintln!("build failed: {e}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
