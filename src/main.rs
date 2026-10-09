use clap::Parser;

fn main() {
    let cli = stealthlingo::cli::Cli::parse();
    if let Err(err) = stealthlingo::commands::run(cli) {
        eprintln!("Error: {err:#}");
        std::process::exit(1);
    }
}
