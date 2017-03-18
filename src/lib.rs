//panda_console/lib.rs

#[macro_use]
extern crate log;
extern crate gfx;
extern crate glutin;
extern crate gfx_window_glutin;
extern crate gfx_text;

pub mod colors;
pub mod ui;

use std::fs::{self, File};
use std::io::{Write, BufWriter};
use std::path::Path;
use std::thread;
use std::sync::mpsc::{self, Sender, Receiver};
use std::time::Duration;
use std::fmt;

pub use glutin::VirtualKeyCode as KeyCode;
use glutin::{WindowBuilder, GL_CORE, Event, ElementState};
use gfx::traits::Device;
use gfx_window_glutin as gfxw;

pub const DEFAULT_FONT_PATH: &'static str = "fonts/MorePerfectDOSVGA.ttf";
const DEFAULT_FONT: &'static [u8; 78252] = include_bytes!("assets/MorePerfectDOSVGA.ttf");
const DEFAULT_FONT_SIZE: u8 =  16;
const RENDER_LOOP_DELAY: u64 = 10;
const ERROR_MSG_PRE_INIT_COMMS: &'static str =
    "An attempt was made to communicate with the render loop before the console was initialized";
const LINE_PADDING: u8 = 2;

enum RenderLoopMessage {
    Add {t: Text},
    Clear,
    Quit,
    LiveCheck,
}

/// Represents a piece of text that can be rendered on the console window
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

/// Contains information about the Console that should be sent to the render thread.
#[derive(Clone)]
struct ConsoleInfo {
    width: u32,
    height: u32,
    title: &'static str,
    font_path: &'static str,
    font_size: u8,
}

/// Represents a window onto which text can be rendered, and through which user-input can be detected.
pub struct Console {
    info: ConsoleInfo,
    msg_sender: Option<Sender<RenderLoopMessage>>,
    render_alive_reciever: Option<Receiver<bool>>,
    window_input_reciever: Option<Receiver<Vec<Event>>>,
    buffer_clear_flag_receiver: Option<Receiver<bool>>,
    input_cache: Vec<Event>,
}

impl Console {
    /// Returns a new Console object with pre-defined typeface settings.
    /// If the default typeface file is not found in the `fonts` directory, then it will
    /// be exported and saved to that path.
    pub fn new_with_default_typeface(width: u32, height: u32, title: &'static str) -> Console {
        info!("Creating a Console instance with default path parameters...");
        //if the default font file doesn't exists, then export a copy
        let font_path = Path::new(DEFAULT_FONT_PATH);
        if !font_path.exists() {
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
            buffer_clear_flag_receiver: None,
            input_cache: Vec::<Event>::new(),
        }
    }

    /// Returns a new Console object with user-defined typeface settings.
    pub fn new(width: u32, height: u32, font_path: &'static str, title: &'static str,
               font_size: u8) -> Console {
        info!("Creating a Console instance with font located at {} (size: {})...",
              font_path, font_size);

        if font_path == DEFAULT_FONT_PATH {
            let p = Path::new(DEFAULT_FONT_PATH);
            if !p.exists() {
                Console::export_default_typeface(p);
            }
        }

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
            buffer_clear_flag_receiver: None,
            input_cache: Vec::<Event>::new(),
        }
    }

    ///Sets the font size to be used by the console
    pub fn set_font_size(&mut self, font_size: u8) {
        let info = self.info.clone();
        self.info = ConsoleInfo {
            width: info.width,
            height: info.height,
            title: info.title,
            font_path: info.font_path,
            font_size: font_size,
        }
    }

    /// Returns the height of lines on the console.  Currently calculated as font_size + padding.
    pub fn line_height(&self) -> i32 {
        (self.info.font_size + LINE_PADDING) as i32
    }

    /// Asks the render thread to indicate whether it's alive, and returns the response
    /// if there is one (will return False upon receiving no response)
    pub fn is_alive(&self) -> bool {
        self.check_render_thread_alive()
    }

    /// Checks whether or not the given key is being pressed
    pub fn key_pressed(&mut self, key: KeyCode) -> bool {
        self.check_keypress_input(ElementState::Pressed, key)
    }

    /// Checks whether or not the given key is being released
    pub fn key_released(&mut self, key: KeyCode) -> bool {
        self.check_keypress_input(ElementState::Released, key)
    }

    /// If there is a key being pressed, returns the key's associated character
    pub fn char_entered(&mut self) -> Option<char> {
        self.check_char_entered()
    }

    /// Draws the provided Text instance to the window
    pub fn draw_text(&self, t: Text) {
        self.send_to_render_thread(RenderLoopMessage::Add{t: t.clone()});
    }

    /// Clears all Text instances from the window
    pub fn clear(&self) {
        self.send_to_render_thread(RenderLoopMessage::Clear);
    }

    /// Kills the console's render thread
    pub fn quit(&self) {
        self.send_to_render_thread(RenderLoopMessage::Quit);
    }

    /// Initializes the render thread and all cross-thread communications
    pub fn init(&mut self) {
        //set up cross-thread communications channels
        let (msg_sender, msg_receiver) = mpsc::channel();
        let (alive_sender, alive_reciever) = mpsc::channel();
        let (input_sender, input_receiver) = mpsc::channel();
        let (buffer_clear_sender, buffer_clear_receiver) = mpsc::channel();
        self.msg_sender = Some(msg_sender);
        self.render_alive_reciever = Some(alive_reciever);
        self.window_input_reciever = Some(input_receiver);
        self.buffer_clear_flag_receiver = Some(buffer_clear_receiver);

        Console::init_render_thread(msg_receiver, alive_sender, input_sender, buffer_clear_sender, self.info.clone());
    }

    /// Sends the given RenderLoopMessage to the render thread
    fn send_to_render_thread(&self, msg: RenderLoopMessage) {
        if let Some(ref tx) = self.msg_sender {
            tx.send(msg).unwrap();
        } else {
            panic!(ERROR_MSG_PRE_INIT_COMMS);
        }
    }

    /// After refreshing the input buffer, checks whether it contains an event corresponding to
    /// the given state and key code.
    fn check_keypress_input(&mut self, state: ElementState, key: KeyCode) -> bool {
        self.refresh_input_cache();
        let mut hit = false;
        for e in &self.input_cache {
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
        hit
    }

    /// Checks for ReceivedChatracter events in the current input buffer and returns the
    /// character associated with the first such event found.
    fn check_char_entered(&mut self) -> Option<char> {
        let mut input_char = None;
        self.refresh_input_cache();
        for e in &self.input_cache {
            match e {
                &Event::ReceivedCharacter(c) => {
                    input_char = Some(c);
                    break;
                },
                _ => { },
            }
        }
        input_char
    }

    /// Checks for an indication from the render thread that the input buffer should be cleared.
    /// If such an indication is received, then the buffer is cleared and the latest input frame is
    /// retrieved from the render thread and stored in the input buffer.
    fn refresh_input_cache(&mut self) {
        let mut should_refresh = false;
        if let Some(ref rx) = self.buffer_clear_flag_receiver {
            if let Ok(true) = rx.try_recv() {
                should_refresh = true;
                //drain all flags from the channel
                while let Ok(_) = rx.try_recv() { }
            }
        }

        if should_refresh {
            //get the latest input frame from the render thread
            self.input_cache.clear();
            if let Some(ref rx) = self.window_input_reciever {
                let mut temp_input_buffer: Option<Vec<Event>> = None;
                while let Ok(buffer) = rx.try_recv() {
                    temp_input_buffer = Some(buffer);
                }
                if let Some(buf) = temp_input_buffer {
                    for e in &buf {
                        self.input_cache.push(e.clone());
                    }
                }
            }
        }
    }

    /// Exports a copy of the included default typeface
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

    /// Sends the render thread a LiveCheck message, then waits for a response.
    /// Returns True if a response is received, otherwise returns False.
    fn check_render_thread_alive(&self) -> bool {
        self.send_to_render_thread(RenderLoopMessage::LiveCheck);
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

    /// Initializes the window and rendering mechanisms, then kicks off the rendering thread.
    fn init_render_thread(msg_receiver: Receiver<RenderLoopMessage>, alive_sender: Sender<bool>,
                          input_sender: Sender<Vec<Event>>, buffer_clear_sender: Sender<bool>,
                          parent_info: ConsoleInfo) {
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

                //signal to the parent thread that by this time whatever input cache it has
                //is stale and should be cleared
                buffer_clear_sender.send(true).unwrap();

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
}
