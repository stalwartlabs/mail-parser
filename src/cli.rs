/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use std::{
    fs,
    io::{stdin, BufRead, IsTerminal},
    path::PathBuf,
    process::exit,
};

use clap::Parser;
use mail_parser::MessageParser;

/// Parse an e-mail and print its JSON structure.
#[derive(Debug, Parser)]
#[command(name = env!("CARGO_PKG_NAME"), author, version)]
pub struct Cli {
    /// Raw message content or path to a file.
    ///
    /// Omit to read from stdin.
    #[arg(trailing_var_arg = true)]
    #[arg(name = "message", value_name = "MESSAGE")]
    pub message: Vec<String>,

    /// Pretty-print JSON output.
    #[arg(short, long)]
    pub pretty: bool,
}

pub fn run() {
    let cli = Cli::parse();

    let message = if stdin().is_terminal() {
        if cli.message.is_empty() {
            eprintln!("Failed to read message");
            exit(1);
        }

        let path = PathBuf::from(&cli.message[0]);

        if path.is_file() {
            match fs::read_to_string(&path) {
                Ok(message) => message,
                Err(err) => {
                    eprintln!("Failed to read message at {}: {err}", path.display());
                    exit(1)
                }
            }
        } else {
            cli.message
                .join(" ")
                .replace('\r', "")
                .replace('\n', "\r\n")
        }
    } else {
        stdin()
            .lock()
            .lines()
            .map_while(Result::ok)
            .collect::<Vec<String>>()
            .join("\r\n")
    };

    let Some(message) = MessageParser::default().parse(&message) else {
        eprintln!("Failed to parse message");
        exit(1)
    };

    let json = if cli.pretty {
        serde_json::to_string_pretty(&message)
    } else {
        serde_json::to_string(&message)
    };

    match json {
        Ok(json) => println!("{json}"),
        Err(err) => {
            eprintln!("Failed to parse message: {err}");
            exit(1);
        }
    }
}
