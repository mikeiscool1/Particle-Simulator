use macroquad::prelude::*;
use crate::State;
use crate::component::{Component, Event};

pub struct Grid {
    pub visible: bool
}

impl Component for Grid {
    fn draw(&self, state: &State) {
        if !self.visible {
            return;
        }

        let camera_pos = state.camera.position;
        let grid_spacing = 1.0;

        // Get the VP matrix to compute frustum reach
        let view = Mat4::look_at_rh(state.camera.position, state.camera.target, state.camera.up);
        let proj = Mat4::perspective_rh_gl(60f32.to_radians(), screen_width() / screen_height(), 0.1, 1000.0);
        let vp = proj * view;
        let inv_vp = vp.inverse();

        // Unproject the four far-plane corners to find max world extent
        let ndc_corners = [
            vec4(-1.0, -1.0, 1.0, 1.0),
            vec4( 1.0, -1.0, 1.0, 1.0),
            vec4(-1.0,  1.0, 1.0, 1.0),
            vec4( 1.0,  1.0, 1.0, 1.0),
        ];
        let grid_size = ndc_corners.iter().map(|&c| {
            let world = inv_vp * c;
            let world = world.xyz() / world.w;
            (world - camera_pos).length()
        }).fold(0.0f32, f32::max);

        let steps = (grid_size * 2.0 / grid_spacing) as i32;

        let ox = (camera_pos.x / grid_spacing).floor() * grid_spacing;
        let oy = (camera_pos.y / grid_spacing).floor() * grid_spacing;
        let oz = (camera_pos.z / grid_spacing).floor() * grid_spacing;

        let max_dist = grid_size * 1.5;
        let fade = |dist: f32| -> f32 { 1.0 - (dist / max_dist).clamp(0.0, 1.0) };

        for i in 0..=steps {
            let t = -grid_size + i as f32 * grid_spacing;

            let xz_color = Color::new(0.5, 0.5, 0.5, fade((vec3(0.0, 0.0, oz + t) - camera_pos).length()) * 0.8 + 0.1);
            let xy_color = Color::new(0.4, 0.4, 0.5, fade((vec3(0.0, oy + t, 0.0) - camera_pos).length()) * 0.7 + 0.1);
            let yz_color = Color::new(0.5, 0.4, 0.4, fade((vec3(ox + t, 0.0, 0.0) - camera_pos).length()) * 0.7 + 0.1);

            // XZ plane
            draw_line_3d(vec3(ox - grid_size, 0.0, oz + t), vec3(ox + grid_size, 0.0, oz + t), xz_color);
            draw_line_3d(vec3(ox + t, 0.0, oz - grid_size), vec3(ox + t, 0.0, oz + grid_size), xz_color);

            // XY plane
            draw_line_3d(vec3(ox - grid_size, oy + t, 0.0), vec3(ox + grid_size, oy + t, 0.0), xy_color);
            draw_line_3d(vec3(ox + t, oy - grid_size, 0.0), vec3(ox + t, oy + grid_size, 0.0), xy_color);

            // YZ plane
            draw_line_3d(vec3(0.0, oy - grid_size, oz + t), vec3(0.0, oy + grid_size, oz + t), yz_color);
            draw_line_3d(vec3(0.0, oy + t, oz - grid_size), vec3(0.0, oy + t, oz + grid_size), yz_color);
        }

        draw_line_3d(vec3(ox - grid_size, 0.0, 0.0), vec3(ox + grid_size, 0.0, 0.0), BLUE);
        draw_line_3d(vec3(0.0, oy - grid_size, 0.0), vec3(0.0, oy + grid_size, 0.0), BLUE);
        draw_line_3d(vec3(0.0, 0.0, oz - grid_size), vec3(0.0, 0.0, oz + grid_size), BLUE);
    }

    fn handle_input(&mut self, state: &mut State) {
        if state.ui_captures_keyboard {
            return;
        }

        if is_key_pressed(KeyCode::G) {
            self.visible = !self.visible;
            state.events.push(Event::Alert(format!("Grid: {}", if self.visible { "On" } else { "Off" })));
        }
    }

    fn update(&mut self, _dt: f32, _state: &mut State) {
        
    }
}