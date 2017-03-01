#[macro_use]
extern crate log;
extern crate panda_console;
extern crate env_logger;
extern crate fps_counter;

use std::env;

use panda_console::{colors, Console, Text, VirtualKeyCode};

use fps_counter::FPSCounter;

fn main() {
    // env::set_var("RUST_LOG", "info");

    env_logger::init().unwrap();

    info!("Starting panda_console example implementation...");

    info!("Creating a new Console object with default font settings...");
    let mut c = Console::new_with_default_typeface(640, 480, "Hello world!");

    info!("Initializing Console object now!");
    c.init();

    let mut fps = FPSCounter::new();

    let mut show_message = false;

    info!("Checking whether Console is alive...  Result: {}", c.is_alive());
    while c.is_alive() {
        c.clear();

        c.draw_text(Text {
            content: format!("FPS: {}", fps.tick()),
            pos_x: 100,
            pos_y: 100,
            color: colors::GREEN,
        });

        if !show_message && c.key_pressed(VirtualKeyCode::A) {
            show_message = true;
        }

        if show_message && c.key_released(VirtualKeyCode::A) {
            show_message = false;
        }

        if show_message {
            c.draw_text(Text {
                content: format!("The A-key is pressed!"),
                pos_x: 100,
                pos_y: 150,
                color: colors::RED,
            });            
        }
    }
}
