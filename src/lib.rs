#[macro_use]
extern crate log;
extern crate gfx;
extern crate glutin;
extern crate gfx_window_glutin;
extern crate gfx_text;

pub mod colors;

use std::fs::{self, File};
use std::io::{Write, BufWriter};
use std::path::Path;
use std::thread;
use std::sync::mpsc::{self, Sender, Receiver};
use std::time::Duration;
use std::fmt;

pub use glutin::{VirtualKeyCode};
use glutin::{WindowBuilder, GL_CORE, Event, ElementState};
use gfx::traits::Device;
use gfx_window_glutin as gfxw;

const DEFAULT_FONT_PATH: &'static str = "fonts/MorePerfectDOSVGA.ttf";
const DEFAULT_FONT: &'static [u8; 78252] = include_bytes!("assets/MorePerfectDOSVGA.ttf");
const DEFAULT_FONT_SIZE: u8 =  16;
const RENDER_LOOP_DELAY: u64 = 10;
const ERROR_MSG_PRE_INIT_COMMS: &'static str =
    "An attempt was made to communicate with the render loop before the console was initialized";

enum RenderLoopMessage {
    Add {t: Text},
    Clear,
    Quit,
    LiveCheck,
}

#[derive(Clone, Debug)]
pub struct Text {
    pub content: String,
    pub pos_x: i32,
    pub pos_y: i32,
    pub color: [f32; 4],
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "content: \"{}\", x: {}, y: {}", self.content, self.pos_x, self.pos_y)
    }
}

#[derive(Clone)]
struct ConsoleInfo {
    width: u32,
    height: u32,
    title: &'static str,
    font_path: &'static str,
    font_size: u8,
}

pub struct Console {
    info: ConsoleInfo,
    msg_sender: Option<Sender<RenderLoopMessage>>,
    render_alive_reciever: Option<Receiver<bool>>,
    window_input_reciever: Option<Receiver<Vec<Event>>>,
}

impl Console {
    /// Returns a new Console object with pre-defined typeface settings.
    /// If the default typeface file is not found in the `fonts` directory, then it will
    /// be exported and saved to that path.
    pub fn new_with_default_typeface(width: u32, height: u32, title: &'static str) -> Console {
        info!("Creating a Console instance with default path parameters...");
        //if the default font file doesn't exists, then export a copy
        let font_path = Path::new(DEFAULT_FONT_PATH);
        if !Path::new(font_path).exists() {
            info!("Font file at {} was not found.  Exporting that now...", DEFAULT_FONT_PATH);
            Console::export_default_typeface(font_path);
        }
        //return a new Console with all default information
        Console {
            info: ConsoleInfo {
                width: width,
                height: height,
                title: title,
                font_path: DEFAULT_FONT_PATH,
                font_size: DEFAULT_FONT_SIZE,
            },
            msg_sender: None,
            render_alive_reciever: None,
            window_input_reciever: None,
        }
    }

    /// Returns a new Console object with user-defined typeface settings.
    pub fn new(width: u32, height: u32, font_path: &'static str, title: &'static str,
               font_size: u8) -> Console {
        info!("Creating a Console instance with font located at {} (size: {})...",
              font_path, font_size);
        Console {
            info: ConsoleInfo {
                width: width,
                height: height,
                title: title,
                font_path: font_path,
                font_size: font_size,
            },
            msg_sender: None,
            render_alive_reciever: None,
            window_input_reciever: None,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.send_live_check();
        if let Some(ref rx) = self.render_alive_reciever {
            match rx.recv() {
                Ok(result)  => result,
                Err(why)    => panic!("Failed to hear back from the render loop after performing live-check: {}",
                                      why)
            }
        } else {
            panic!(ERROR_MSG_PRE_INIT_COMMS);
        }
    }

    pub fn key_pressed(&self, key: VirtualKeyCode) -> bool {
        self.check_keypress_input(ElementState::Pressed, key)
    }

    pub fn key_released(&self, key: VirtualKeyCode) -> bool {
        self.check_keypress_input(ElementState::Released, key)
    }

    pub fn char_entered(&self) -> Option<char> {
        if let Some(ref rx) = self.window_input_reciever {
            let mut input_char = None;
            while let Ok(buffer) = rx.try_recv() {
                for e in &buffer {
                    match e {
                        &Event::ReceivedCharacter(c) => {
                            input_char = Some(c);
                            break;
                        },
                        _ => { },
                    }
                }
            }
            input_char
        } else {
            panic!(ERROR_MSG_PRE_INIT_COMMS);
        }
    }

    pub fn draw_text(&self, t: Text) {
        info!("Adding Text object to shared cache: {}...", t);
        if let Some(ref tx) = self.msg_sender {
            tx.send(RenderLoopMessage::Add{t: t.clone()}).unwrap();
        } else {
            panic!(ERROR_MSG_PRE_INIT_COMMS);
        }
    }

    pub fn clear(&self) {
        info!("Clearing shared Text cache...");
        if let Some(ref tx) = self.msg_sender {
            tx.send(RenderLoopMessage::Clear).unwrap();
        } else {
            panic!(ERROR_MSG_PRE_INIT_COMMS);
        }
    }

    pub fn quit(&self) {
        info!("Sending Quit message to render loop...");
        if let Some(ref tx) = self.msg_sender {
            tx.send(RenderLoopMessage::Quit).unwrap();
        }
    }

    pub fn init(&mut self) {
        //set up cross-thread communications channels
        let (msg_sender, msg_receiver) = mpsc::channel();
        let (alive_sender, alive_reciever) = mpsc::channel();
        let (input_sender, input_receiver) = mpsc::channel();
        self.msg_sender = Some(msg_sender);
        self.render_alive_reciever = Some(alive_reciever);
        self.window_input_reciever = Some(input_receiver);

        Console::init_render_thread(msg_receiver, alive_sender, input_sender, self.info.clone());
    }

    /// Initializes the window and rendering mechanisms, then kicks off the rendering thread.
    fn init_render_thread(msg_receiver: Receiver<RenderLoopMessage>, alive_sender: Sender<bool>,
                          input_sender: Sender<Vec<Event>>, parent_info: ConsoleInfo) {
        info!("Spawning Console render thread...");

        //spawn render thread
        thread::spawn(move || {
            info!("Instantiating renderer Text object buffer...");
            let mut text_buffer = Vec::<Text>::new();

            info!("Instantiating window input buffer...");
            let mut input_buffer = Vec::<Event>::new();

            info!("Building gfx window and device...");
            let (window, mut device, mut factory, main_color, _) = {
                let builder = WindowBuilder::new()
                    .with_dimensions(parent_info.width, parent_info.height)
                    .with_title(parent_info.title)
                    .with_gl(GL_CORE);
                gfxw::init::<gfx::format::Rgba8, gfx::format::Depth>(builder)
            };

            info!("Instantiating gfx Encoder object...");
            let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

            info!("Instantiating gfx_text text renderer object...");
            let mut text_renderer = gfx_text::new(factory.clone())
                .with_size(parent_info.font_size)
                .with_font(parent_info.font_path)
                .unwrap();

            let mut quit = false;

            info!("Initialization successful.  Beginning render loop!");
            while !quit {
                //process events
                for event in window.poll_events() {
                    match event {
                        Event::Closed  => {
                                quit = true;
                                break;
                            },
                        _ => input_buffer.push(event),
                    }
                }
                if !input_buffer.is_empty() {
                    input_sender.send(input_buffer.clone()).unwrap();
                    input_buffer.clear();
                }

                while let Ok(incoming_msg) = msg_receiver.try_recv() {
                    match incoming_msg {
                        RenderLoopMessage::Add { t }    => text_buffer.push(t),
                        RenderLoopMessage::Clear        => text_buffer.clear(),
                        RenderLoopMessage::Quit         => quit = true,
                        RenderLoopMessage::LiveCheck    => alive_sender.send(!quit).unwrap(),
                    };
                }

                for t in &text_buffer {
                    text_renderer.add(t.content.as_str(), [t.pos_x, t.pos_y], t.color);
                }

                encoder.clear(&main_color, colors::BLACK);
                text_renderer.draw(&mut encoder, &main_color).unwrap();
                encoder.flush(&mut device);
                window.swap_buffers().unwrap();
                device.cleanup();

                thread::sleep(Duration::from_millis(RENDER_LOOP_DELAY));
            }
            //let the parent thread know we've died just in case it hasn't had a chance to check
            alive_sender.send(false).unwrap();
        });
    }

    fn send_live_check(&self) {
        if let Some(ref tx) = self.msg_sender {
            tx.send(RenderLoopMessage::LiveCheck).unwrap();
        } else {
            panic!(ERROR_MSG_PRE_INIT_COMMS);
        }
    }

    fn check_keypress_input(&self, state: ElementState, key: VirtualKeyCode) -> bool {
        let mut hit = false;
        if let Some(ref rx) = self.window_input_reciever {
            while let Ok(buffer) = rx.try_recv() {
                for e in &buffer {
                    match e {
                        &Event::KeyboardInput(key_state, _, key_code) => {
                            if key_state == state && key_code == Some(key) {
                                hit = true;
                                break;
                            }
                        },
                        _ => { },
                    }
                }
                if hit { break; }
            }
        } else {
            panic!(ERROR_MSG_PRE_INIT_COMMS);
        }
        hit
    }

    fn export_default_typeface(p: &Path) {
        let font_directory = p.parent().unwrap();
        if let Ok(_) = fs::create_dir_all(font_directory) {
            if let Ok(f) = File::create(p) {
                let mut writer = BufWriter::new(f);
                if let Err(why) = writer.write_all(DEFAULT_FONT) {
                    info!("Unable to write default TTF contents to file: {}", why);
                    panic!();
                }
            } else {
                info!("Unable create destination file for default TTF!");
                panic!();
            }
        } else {
            info!("Unable to create directory to store exported default font at {}.",
                   font_directory.to_str().unwrap());
        }
    }
}
