#![feature(windows_subsystem)]
#![windows_subsystem = "windows"]
#[macro_use]
extern crate log;
extern crate panda_console;
extern crate env_logger;
extern crate fps_counter;

use std::env;
use panda_console::{colors, Console, Text, KeyCode};
use panda_console::ui::UIElement;
use panda_console::ui::SingleSelect;
use fps_counter::FPSCounter;


fn main() {
    // env::set_var("RUST_LOG", "info");

    env_logger::init().unwrap();

    info!("Starting panda_console example implementation...");

    info!("Creating a new Console object with default font settings...");
    let mut c = Console::new_with_default_typeface(640, 480, "Hello world!");

    c.set_font_size(36u8);

    info!("Initializing Console object now!");
    c.init();

    let mut fps = FPSCounter::new();

    let mut menu = SingleSelect::<&str>::new();
    menu.add("Item 1");
    menu.add("Item 2");
    menu.add("Item 3");
    menu.add("Item 4");
    menu.add("Item 5");

    info!("Checking whether Console is alive...  Result: {}", c.is_alive());
    while c.is_alive() {
        c.clear();

        c.draw_text(Text {
            content: format!("FPS: {}", fps.tick()),
            pos_x: 100,
            pos_y: 100,
            color: colors::GREEN,
        });

        menu.update(&mut c);
        menu.draw(&c);

    }
}
