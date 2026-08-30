use std::path::PathBuf;

use clap::{arg, builder::EnumValueParser, command, value_parser};

use crate::ecosystem::types::ReleaseKind;

pub fn cli() -> clap::ArgMatches {
    command!()
        .arg(arg!([kind] "The release version e.g major.").required(true).value_parser(EnumValueParser::<ReleaseKind>::new()))
        .arg(arg!(
            -r --repo <PATH> "Specify the git repo."
        ).value_parser(value_parser!(PathBuf)))
        .after_help("This util helps streamline the release process. Calling git release increments the tag.")
        .get_matches()
}
