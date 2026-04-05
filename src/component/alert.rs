use macroquad::prelude::*;

use crate::State;
use crate::component::Component;

pub struct Alert {
    alert_flash: f32,
    alert_flash_duration: f32,
    alert_text: String,
}

impl Alert {
    pub fn new() -> Self {
        Self {
            alert_flash: 0.0,
            alert_flash_duration: 2.0,
            alert_text: String::new(),
        }
    }

    pub fn alert(&mut self, text: &str) {
        self.alert_text = text.to_string();
        self.alert_flash = self.alert_flash_duration;
    }
}

impl Component for Alert {
    fn draw(&self, state: &State) {
        if self.alert_flash > 0.0 {
            set_default_camera();

            let alpha = (self.alert_flash / self.alert_flash_duration).clamp(0.0, 1.0);
            let x = (state.editor_panel_width + 24.0).max(24.0);
            let y = 48.0;
            let font_size = 36;
            let outline_offsets = [
                (-2.0, 0.0),
                (2.0, 0.0),
                (0.0, -2.0),
                (0.0, 2.0),
                (-1.5, -1.5),
                (-1.5, 1.5),
                (1.5, -1.5),
                (1.5, 1.5),
            ];
            let text_metrics = measure_text(&self.alert_text, None, font_size, 1.0);
            let padding_x = 14.0;
            let padding_y = 10.0;
            let panel_x = x - padding_x;
            let panel_y = y - text_metrics.offset_y - padding_y;
            let panel_w = text_metrics.width + padding_x * 2.0;
            let panel_h = text_metrics.height + padding_y * 2.0;

            draw_rectangle(
                panel_x,
                panel_y,
                panel_w,
                panel_h,
                Color::new(0.02, 0.03, 0.05, alpha * 0.8),
            );
            draw_rectangle_lines(
                panel_x,
                panel_y,
                panel_w,
                panel_h,
                2.0,
                Color::new(1.0, 1.0, 1.0, alpha * 0.35),
            );

            for (dx, dy) in outline_offsets {
                draw_text_ex(
                    &self.alert_text,
                    x + dx,
                    y + dy,
                    TextParams {
                        font_size,
                        color: Color::new(0.0, 0.0, 0.0, alpha),
                        ..Default::default()
                    },
                );
            }

            draw_text_ex(
                &self.alert_text,
                x,
                y,
                TextParams {
                    font_size,
                    color: Color::new(1.0, 1.0, 1.0, alpha),
                    ..Default::default()
                },
            );

            set_camera(&state.camera);
        }
    }

    fn update(&mut self, dt: f32, _state: &mut State) {
        if self.alert_flash > 0.0 {
            self.alert_flash = (self.alert_flash - dt).max(0.0);
        }
    }

    fn handle_input(&mut self, _state: &mut State) {}
}
