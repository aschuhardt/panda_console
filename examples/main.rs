#[macro_use]
extern crate log;
extern crate panda_console;
extern crate env_logger;

use panda_console::{colors, Console, Text, VirtualKeyCode};
use std::thread;
use std::time::Duration;
use std::env;

fn main() {
    env::set_var("RUST_LOG", "info");

    env_logger::init().unwrap();

    info!("Starting panda_console example implementation...");

    info!("Creating a new Console object with default font settings...");
    let mut c = Console::new_with_default_typeface(640, 480, "Hello world!");

    info!("Initializing Console object now!");
    c.init();

    let mut counter = 0;

    info!("Checking whether Console is alive...  Result: {}", c.is_alive());
    while c.is_alive() {
        c.clear();

        c.draw_text(Text {
            content: format!("Milliseconds elapsed: {}", counter),
            pos_x: 100,
            pos_y: 100,
            color: colors::GREEN,
        });

        if c.key_pressed(VirtualKeyCode::A) {
            c.draw_text(Text {
                content: format!("The A key is being pressed!"),
                pos_x: 100,
                pos_y: 150,
                color: colors::RED,
            });
        }

        counter += 1;
        thread::sleep(Duration::from_millis(1));
    }
}
