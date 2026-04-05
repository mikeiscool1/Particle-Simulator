use crate::State;

pub mod alert;
pub mod editor;
pub mod fps;
pub mod grid;
pub mod particles;

pub use alert::Alert;
pub use editor::Editor;
pub use fps::FPS;
pub use grid::Grid;
pub use particles::{Particle, Particles};

pub enum Event {
    Alert(String),
    ResetSimulation,
}

pub trait Component {
    fn draw(&self, _state: &State);
    fn handle_input(&mut self, _state: &mut State);
    fn update(&mut self, _dt: f32, _state: &mut State);
}
