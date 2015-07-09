extern crate kitten;
use kitten::Plugin;

struct Foo;

impl kitten::Plugin for Foo {
    fn process_privmsg(&self, target: &str, message: &str) -> Option<String> {
        if message.contains("pepe") {
            Some("GET THE FUCK OFF MY BOARD FUCKING NORMIES RRREEEEEEEEEEEEEEEE".to_string())
        } else {
            None
        }
    }
}


#[no_mangle]
pub extern fn init_plugin() -> Box<Plugin> {
    Box::new(Foo)
}
