mod cli;
mod crypto;
mod error;
mod mcp;
mod network;
mod services;
mod types;
mod ui;
mod version;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use colored::Colorize;
use std::process;

use crate::{
    network::ItylosApi,
    services::{read_secret, send_secret, verify_proof},
    types::{SendOptions, Ttl},
};

fn main() {
    if let Err(error) = run() {
        match error.downcast_ref::<clap::Error>() {
            Some(clap_error)
                if matches!(
                    clap_error.kind(),
                    clap::error::ErrorKind::DisplayHelp
                        | clap::error::ErrorKind::DisplayVersion
                        | clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
                        | clap::error::ErrorKind::MissingSubcommand
                ) =>
            {
                clap_error.print().ok();
                process::exit(0);
            }
            _ => {
                eprintln!("{} {}", "[ERROR]".red().bold(), error);
                process::exit(2);
            }
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::try_parse()?;

    let Some(command) = cli.command else {
        ui::print_banner();
        return Ok(());
    };

    // Vérification de mise à jour non-bloquante (sauf mcp)
    if !matches!(command, Commands::Mcp) {
        version::check_for_update();
    }

    match command {
        Commands::Send {
            text,
            duration,
            file,
            password,
        } => {
            let api = ItylosApi::new()?;
            let pwd = if password {
                Some(ui::prompt_new_password()?)
            } else {
                None
            };
            send_secret(
                &api,
                SendOptions {
                    text: text.unwrap_or_default(),
                    file,
                    ttl: Ttl::parse(&duration),
                    password: pwd,
                },
            )?;
        }
        Commands::Read { url } => {
            let api = ItylosApi::new()?;
            read_secret(&api, &url)?;
        }
        Commands::Verify { proof } => {
            let api = ItylosApi::new()?;
            verify_proof(&proof, Some(&api))?;
        }
        Commands::Mcp => {
            mcp::start_mcp_server()?;
        }
        Commands::Update => {
            version::run_self_update();
        }
    }

    Ok(())
}
