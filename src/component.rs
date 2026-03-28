use crate::State;

pub mod grid;
pub mod particles;
pub mod alert;
pub mod fps;
pub mod editor;

pub use grid::Grid;
pub use particles::{Particle, Particles};
pub use alert::Alert;
pub use fps::FPS;
pub use editor::Editor;


pub enum Event {
    Alert(String),
    ResetSimulation,
    ShowGrid(bool)
}

pub trait Component {
    fn draw(&self, _state: &State);
    fn handle_input(&mut self, _state: &mut State);
    fn update(&mut self, _dt: f32, _state: &mut State);
}