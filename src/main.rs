#[cfg(feature = "cli")]
mod cli;

#[cfg(feature = "cli")]
fn main() {
    cli::run();
}

#[cfg(not(feature = "cli"))]
fn main() {
    eprintln!("Build with `--features cli` to enable the CLI");
}
