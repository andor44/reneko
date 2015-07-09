#![crate_type = "dylib"]

extern crate irc;

use irc::client::server::Server;


pub trait Plugin {
    fn process_privmsg(&self, connection: &Server, source: &str, target: &str, message: &str) -> Option<String>;
}
