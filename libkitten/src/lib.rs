#![crate_type = "dylib"]

extern crate irc;

use std::io::{BufReader, BufWriter};

use irc::client::server::Server;
use irc::client::conn::NetStream;


pub type KittenServer<'a> = Server<'a, BufReader<NetStream>, BufWriter<NetStream>>;

pub trait Plugin {
    /// Process a private message event. An optionally returned string will be automatically sent
    /// to the channel or the sender.
    fn process_privmsg(&self, connection: &KittenServer, source: &str, target: &str, message: &str) -> Option<String>;
}

pub type PluginLoader = fn() -> Result<Box<Plugin>, String>;
pub static LOADER_NAME: &'static str = "init_plugin";
