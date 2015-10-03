#![feature(slice_patterns)]
#![feature(dynamic_lib)]

extern crate irc;
#[macro_use] extern crate log;
extern crate kitten;

use std::dynamic_lib::DynamicLibrary;
use std::path::Path;

use irc::client::prelude::*;
use irc::client::data::command::Command::{PRIVMSG, ERROR};

use kitten::{Plugin, LOADER_NAME, PluginLoader};

struct RenekoPlugin {
    plugin: Box<Plugin>,
    library: DynamicLibrary,
}

fn main() {
    // Load config file
    let config = if let Some(path) = std::env::args().nth(1) {
        Config::load(&path)
    }
    else {
        match std::fs::metadata("config.json") {
            Ok(ref metadata) if metadata.is_file() => {
                warn!("No config specified but config.json found, loading");
                Config::load("config.json")
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

    // Set up plugin stuff
    let mut plugins: Vec<RenekoPlugin> = vec![];
    DynamicLibrary::prepend_search_path(std::env::current_dir().unwrap().as_path());

    // Begin event loop
    for msg in server.iter() {
        let message = msg.unwrap().clone();
        if let Ok(command) = Command::from_message_io(Ok(message.clone())) {
            match command {
                ERROR(message) => {
                    println!("ERROR: {}", message);
                },
                PRIVMSG(target, msg) => {
                    println!("[{}]<{}>{}", target, message.get_source_nickname().unwrap(), msg);

                    for plugin in &plugins {
                        let result: Option<String> = plugin.plugin.process_privmsg(&server, "me lol", &target, &msg);
                        if let Some(result) = result {
                            let _ = server.send_privmsg(&target, &result);
                        }
                    }

                    // Command
                    if msg.starts_with(&prefix) {
                        // XXX: this should probably be `split_whitespace`
                        // But that doesn't preserve whitespace
                        let split: Vec<_> = msg[1..].split(' ').collect();

                        // Message consisting of nothing but the prefix
                        // if split.count() == 0 { continue; }

                        match &split[..] {
                            [cmd, target, to_say..] if cmd == "say" || cmd == "msg" => {
                                let _ = server.send_privmsg(target, &to_say.join(" "));
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
                                let _ = server.send_action(target, &to_say.join(" "));
                            },
                            [cmd, plugin_name] if cmd == "load" => {
                                match DynamicLibrary::open(Some(&Path::new(plugin_name))) {
                                    Ok(library) => {
                                        match unsafe { library.symbol::<()>(LOADER_NAME) } {
                                            Ok(symbol) => {
                                                let loader: PluginLoader = unsafe { std::mem::transmute(symbol) };
                                                let plugin = loader();
                                                match plugin {
                                                    Ok(plugin) => {
                                                        plugins.push(RenekoPlugin { plugin: plugin, library: library });
                                                    },
                                                    Err(err) => {
                                                        let _ = server.send_privmsg(&target, &format!("Failed to load plugin: {}", err));
                                                    }
                                                }
                                            },
                                            Err(reason) => {
                                                let _ = server.send_privmsg(&target, &format!("Failed to load plugin: {}", reason));
                                            }
                                        }
                                    },
                                    Err(reason) => {
                                        let _ = server.send_privmsg(&target, &format!("Failed to load plugin: {}", reason));
                                    }
                                }
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
}
