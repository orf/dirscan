use crate::formats::Format;
use std::path::PathBuf;
use structopt::StructOpt;
use strum::VariantNames;
use strum_macros::{Display, EnumString, EnumVariantNames};

#[derive(StructOpt)]
#[structopt(name = "dirscan", about = "Summarize directories, fast.")]
pub struct Args {
    #[structopt(subcommand)]
    pub cmd: Command,
}

#[derive(StructOpt)]
pub enum Command {
    #[structopt(about = "Scan a directory")]
    Scan {
        #[structopt(short = "t", long = "threads")]
        threads: Option<usize>,

        #[structopt(short = "i", long = "ignore-hidden", help = "Ignore hidden files")]
        ignore_hidden: bool,

        #[structopt(short = "a", long = "actual-size", help = "Calculate the actual size")]
        actual_size: bool,

        #[structopt(short = "o", long = "output", parse(from_os_str))]
        output: Option<PathBuf>,

        #[structopt(parse(from_os_str))]
        path: PathBuf,

        #[structopt(
        short = "f",
        long = "format",
        default_value = "json",
        possible_values = &Format::VARIANTS
        )]
        format: Format,
    },
    #[structopt(about = "Parse results files")]
    Parse {
        #[structopt(short = "d", long = "depth", default_value = "1")]
        depth: usize,

        #[structopt(short = "p", long = "prefix", default_value = "")]
        prefix: String,

        #[structopt(parse(from_os_str))]
        input: PathBuf,

        #[structopt(
        short = "f",
        long = "format",
        default_value = "json",
        possible_values = &Format::VARIANTS
        )]
        format: Format,

        #[structopt(
        short = "s",
        long = "sort",
        default_value = "name",
        possible_values = &SortType::VARIANTS
        )]
        sort: SortType,
    },
}

#[derive(EnumString, EnumVariantNames, Display)]
#[strum(serialize_all = "kebab_case")]
pub enum SortType {
    Name,
    Files,
    Size,
}
