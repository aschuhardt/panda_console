use std::fmt;
use ui::{UIElement, IndexedElement};
use Console;
use Text;
use colors;
use KeyCode;

/// Represents a UI element containing multiple options, of which one can be selected.
pub struct SingleSelect<T: fmt::Display> {
    color: [f32; 4],
    items: Vec<T>,
    index: usize,
    pos_x: i32,
    pos_y: i32,
}

impl<T: fmt::Display> UIElement for SingleSelect<T> {
    fn set_color(&mut self, color: [f32; 4]) {
        self.color = color;
    }

    fn set_pos_x(&mut self, x: i32) {
        self.pos_x = x;
    }

    fn set_pos_y(&mut self, y: i32) {
        self.pos_y = y;
    }

    fn draw(&self, c: &Console) {
        let line_count = self.items.len();
        for i in 0..line_count {
            let mut line = format!("{}", self.items[i]);

            if i == self.index {
                line = format!("> {}", line);
            } else {
                line = format!(" {}", line);
            }

            let to_draw = Text {
                content: line,
                pos_x: self.pos_x,
                pos_y: self.pos_y + (i as i32 * c.line_height()),
                color: self.color,
            };

            c.draw_text(to_draw);
        }
    }

    fn draw_at(&self, c: &Console, x: i32, y: i32) {
        let line_count = self.items.len();
        for i in 0..line_count {
            let mut line = format!("{}", self.items[i]);

            if i == self.index {
                line = format!("> {}", line);
            } else {
                line = format!(" {}", line);
            }

            let to_draw = Text {
                content: line,
                pos_x: x,
                pos_y: y + (i as i32 * c.line_height()),
                color: self.color,
            };

            c.draw_text(to_draw);
        }
    }

    fn update(&mut self, c: &mut Console) {
        if self.index > 0 && c.key_pressed(KeyCode::Up) {
            self.index -= 1;
        } else if self.index < self.items.len() - 1 && c.key_pressed(KeyCode::Down) {
            self.index += 1;
        }
    }

    fn reset(&mut self) {
        self.color = colors::WHITE;
        self.items = Vec::<T>::new();
        self.index = 0;
        self.pos_x = 0;
        self.pos_y = 0;
    }
}

impl<T: fmt::Display> IndexedElement<T> for SingleSelect<T> {
    fn current(&self) -> &T {
        &self.items[self.index]
    }

    fn current_index(&self) -> usize {
        self.index
    }
}

impl<T: fmt::Display> SingleSelect<T> {
    pub fn new() -> SingleSelect<T> {
        SingleSelect {
            color: colors::WHITE,
            items: Vec::<T>::new(),
            index: 0,
            pos_x: 0,
            pos_y: 0,
        }
    }

    pub fn add(&mut self, item: T) {
        self.items.push(item);
    }
}
