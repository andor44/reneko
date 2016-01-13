#![feature(slice_patterns)]

extern crate irc;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate libkitten;
extern crate libloading;

use libloading::{Library, Symbol, Result as LibraryResult};
use irc::client::prelude::*;
use irc::client::data::command::Command::{PRIVMSG, ERROR};
use libkitten::{Plugin, LOADER_NAME, PluginLoader};

struct RenekoPlugin {
    plugin: Box<Plugin>,
    _library: Library,
}

#[derive(Debug)]
enum PluginLoadingError {
    LibraryError(std::io::Error),
    PluginInitializationError(String),
}

fn load_plugin(name: &str) -> Result<RenekoPlugin, PluginLoadingError> {
    let library = Library::new(name);
    let result = match library {
        Ok(ref library) => {
            let symbol: LibraryResult<Symbol<PluginLoader>> = unsafe { library.get(LOADER_NAME.as_bytes()) };

            match symbol {
                Ok(function) => {
                    match function() {
                        Ok(plugin) => Ok(plugin),
                        Err(e) => Err(PluginLoadingError::PluginInitializationError(e)),
                    }
                }
                Err(e) => Err(PluginLoadingError::LibraryError(e)),
            }
        },
        Err(ref e) => return Err(PluginLoadingError::PluginInitializationError(format!("{}", e))),
    };
    result.map(|plugin| RenekoPlugin { plugin: plugin, _library: library.unwrap() })
}

fn main() {
    env_logger::init().expect("Unable to initialize logging");
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
    // DynamicLibrary::prepend_search_path(std::env::current_dir().unwrap().as_path());

    // Begin event loop
    for msg in server.iter() {
        let message = msg.unwrap().clone();
        if let Ok(command) = Command::from_message_io(Ok(message.clone())) {
            match command {
                ERROR(message) => {
                    error!("ERROR: {}", message);
                },
                PRIVMSG(target, msg) => {
                    trace!("[{}]<{}>{}", target, message.get_source_nickname().unwrap(), msg);

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
                            [cmd, channel] if cmd == "part" || cmd == "leave" => {
                                let _ = server.send(Command::PART(channel.to_owned(), None));
                            },
                            [cmd, reason] if cmd == "quit" || cmd == "exit" => {
                                let reason = if reason.trim().is_empty() { "reneko" } else { reason };
                                let _ = server.send_quit(reason);
                            },
                            [cmd, target, to_say..] if cmd == "me" => {
                                let _ = server.send_action(target, &to_say.join(" "));
                            },
                            [cmd, plugin_name] if cmd == "load" => {
                                match load_plugin(plugin_name) {
                                    Ok(plugin) => plugins.push(plugin),
                                    Err(e) => error!("Error loading '{}': {:?}", plugin_name, e)
                                }
                            },
                            _ => {
                                // Unknow command
                            }
                        }
                    }
                },
                command => {
                    warn!("Unknown command: {:?}", command);
                }
            }
        }
    }
}
