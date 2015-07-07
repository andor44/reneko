#![feature(slice_patterns)]

extern crate irc;
#[macro_use] extern crate log;

use irc::client::prelude::*;
use irc::client::data::command::Command::{PRIVMSG, ERROR};

fn main() {
    // Load config file
    let config = if let Some(path) = std::env::args().nth(1) {
        Config::load_utf8(&path[..])
    }
    else {
        match std::fs::metadata("config.json") {
            Ok(ref metadata) if metadata.is_file() => {
                warn!("No config specified but config.json found, loading");
                Config::load_utf8("config.json")
            }
            _ => {
                error!("No usable config found, exiting!");
                std::process::exit(1);
            }
        }
    };

    // Parse config
    let config = match config {
        Ok(config) => config,
        Err(e) => {
            error!("Error with configuration file: {}", e);
            std::process::exit(2);
        }
    };

    let prefix = config.get_option("prefix").to_string();

    // Pass it to server connection
    let server = IrcServer::from_config(config).unwrap();
    // Send auth info
    server.identify().unwrap();

    // Begin event loop
    for message in server.iter() {
        if let Ok(message) = message {
            if let Ok(command) = Command::from_message(&message) {
                match command {
                    ERROR(message) => {
                        println!("ERROR: {}", message);
                    },
                    PRIVMSG(target, msg) => {
                        println!("[{}]<{}>{}", target, message.get_source_nickname().unwrap(), msg);

                        // Command
                        if msg.starts_with(&prefix) {
                            // XXX: this should probably be `split_whitespace`
                            // But that doesn't preserve whitespace
                            let split: Vec<_> = msg[1..].split(' ').collect();
                            
                            // Message consisting of nothing but the prefix
                            // if split.count() == 0 { continue; }

                            match &split[..] {
                                [cmd, target, to_say..] if cmd == "say" || cmd == "msg" => {
                                    let _ = server.send_privmsg(target, &to_say.connect(" ")[..]);
                                },
                                [cmd, channel] if cmd == "join" => {
                                    let _ = server.send_join(channel);
                                },
                                // TODO: part is missing, serious?
                                /*
                                [cmd, channel] if cmd == "part" || cmd == "leave" => {
                                    let _ = server.send_part(channel);
                                },
                                */
                                [cmd, reason] if cmd == "quit" || cmd == "exit" => {
                                    let reason = if reason.trim().is_empty() { "reneko" } else { reason };
                                    let _ = server.send_quit(reason);
                                },
                                [cmd, target, to_say..] if cmd == "me" => {
                                    let _ = server.send_action(target, &to_say.connect(" ")[..]);
                                },
                                _ => {
                                    // Unknow command
                                }
                            }
                        }
                    },
                    command => {
                        println!("Unknown command: {:?}", command);
                    }
                }
            }
        }
        else {
            warn!("WARNING Error processing message from server?!");
        }
    }
}
