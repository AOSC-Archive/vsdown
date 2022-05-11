use checker::download_vscode;
use clap::{Parser, Subcommand};

mod checker;

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    #[clap(subcommand)]
    subcommand: VsdownCommand,
}

#[derive(Subcommand, Debug)]
enum VsdownCommand {
    /// Install vscode
    Install(Install),
    /// Check vscode update
    Check(Check),
}

#[derive(Parser, Debug)]
struct Install;
#[derive(Parser, Debug)]
struct Check;

fn main() {
    let args = Args::parse();
    match args.subcommand {
        VsdownCommand::Install(_) => {
            if let Err(e) = checker::update_checker() {
                println!("{}", e);
                download_vscode().unwrap();
            } else {
                println!("Your VSCode version is lastest!");
            }
        }
        VsdownCommand::Check(_) => {
            if let Err(e) = checker::update_checker() {
                println!("{}", e);
            } else {
                println!("Your VSCode version is lastest!");
            }
        }
    }
}