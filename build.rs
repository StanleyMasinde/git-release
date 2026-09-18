use std::{env, io, path::PathBuf};

#[allow(dead_code)]
#[path = "src/ecosystem/cli.rs"]
mod cli;

#[allow(dead_code)]
#[path = "src/ecosystem/mod.rs"]
mod ecosystem;

#[allow(dead_code)]
#[path = "src/errors.rs"]
mod errors;

fn main() -> io::Result<()> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"));

    // Writes git-release.1, plus git-release-<sub>.1 for any subcommands
    clap_mangen::generate_to(cli::cli(), &out_dir)?;

    Ok(())
}
