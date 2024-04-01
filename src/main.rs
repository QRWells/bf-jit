pub mod error;
pub mod ir;
pub mod jit;
pub mod vm;

use std::{
    io::{stdin, stdout},
    path::PathBuf,
};

use clap::Parser;
use vm::VM;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Cli {
    #[clap(short = 'j', long = "jit", help = "Run in JIT mode")]
    jit: bool,

    #[clap(name = "FILE")]
    source_file: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let opt = Cli::parse();
    let mut vm = VM::new(
        &opt.source_file,
        Box::new(stdin().lock()),
        Box::new(stdout().lock()),
    )?;

    let ret = if opt.jit {
        vm.run_jit()
    } else {
        vm.run()
    };

    if let Err(e) = &ret {
        eprintln!("Error: {}!", e);
    }

    std::process::exit(ret.is_err() as i32)
}
