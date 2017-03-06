mod single_select;

pub use self::single_select::SingleSelect;

use Console;

pub trait UIElement {
    fn set_color(&mut self, color: [f32; 4]);
    fn set_pos_x(&mut self, x: i32);
    fn set_pos_y(&mut self, y: i32);
    fn draw(&self, c: &Console);
    fn draw_at(&self, c: &Console, x: i32, y: i32);
    fn update(&mut self, c: &mut Console);
    fn reset(&mut self);
}

pub trait IndexedElement<T> {
    fn current(&self) -> &T;
    fn current_index(&self) -> usize;
}
