use macroquad::prelude::*;

use crate::State;
use crate::component::Component;

pub struct FPS {
    pub visible: bool,
    pub last_update_time: f64,
    pub fps: i32,
}

impl Component for FPS {
    fn draw(&self, state: &State) {
        if self.visible {
            set_default_camera();
            // draw on upper right side
            let text = format!("FPS: {}", self.fps);
            let text_dimensions = measure_text(&text, None, 24, 1.0);
            let x = screen_width() - text_dimensions.width - 10.0;
            draw_text_ex(
                &text,
                x,
                30.0,
                TextParams {
                    font_size: 24,
                    color: Color::new(1.0, 1.0, 1.0, 1.0),
                    ..Default::default()
                },
            );


            set_camera(&state.camera);
        }
    }

    fn handle_input(&mut self, _state: &mut State) {
        if _state.ui_captures_keyboard {
            return;
        }

        if is_key_pressed(KeyCode::F) {
            self.visible = !self.visible;
        }
    }

    fn update(&mut self, _dt: f32, _state: &mut State) {
        if get_time() - self.last_update_time > 1.0 {
            self.last_update_time = get_time();
            self.fps = get_fps();
        }
    }
}

