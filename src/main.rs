use clap::{Parser, Subcommand};
use console::style;

use crate::checker::install_vscode;

mod checker;
mod logger;

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
    /// Remove vscode
    Remove(Remove),
}

#[derive(Parser, Debug)]
struct Install {
    #[clap(short = 'f', long)]
    force: bool,
}

#[derive(Parser, Debug)]
struct Check;
#[derive(Parser, Debug)]
struct Remove;

fn main() {
    let args = Args::parse();
    match args.subcommand {
        VsdownCommand::Install(Install { force }) => {
            if force {
                if let Err(e) = install_vscode() {
                    error!("{}", e);
                    std::process::exit(1);
                } else {
                    info!("Installation finished!");
                }
            }
            if let Err(e) = checker::update_checker() {
                info!("{}", e);
                if let Err(e) = install_vscode() {
                    error!("{}", e);
                    std::process::exit(1);
                } else {
                    info!("Installation finished!");
                }
            } else {
                info!("Your VSCode version is lastest!");
            }
        }
        VsdownCommand::Check(_) => {
            if let Err(e) = checker::update_checker() {
                info!("{}", e);
            } else {
                info!("Your VSCode version is lastest!");
            }
        }
        VsdownCommand::Remove(_) => {
            if let Err(e) = checker::remove_vscode() {
                error!("{}", e);
                std::process::exit(1);
            } else {
                info!("VSCode has removed!");
            }
        }
    }
}
