use macroquad::prelude::*;
use crate::State;
use crate::component::{Component, Event};

pub struct Grid {
}

impl Component for Grid {
    fn draw(&self, state: &State) {
        if !state.show_grid { return; }

        let camera_pos = state.camera.position;
        let grid_spacing = 1.0;
        let grid_size = 200.0;

        let steps = (grid_size * 2.0 / grid_spacing) as i32;

        let ox = (camera_pos.x / grid_spacing).floor() * grid_spacing;
        let oy = (camera_pos.y / grid_spacing).floor() * grid_spacing;
        let oz = (camera_pos.z / grid_spacing).floor() * grid_spacing;

        let max_dist = grid_size * 1.5;
        let fade = |dist: f32| -> f32 { 1.0 - (dist / max_dist).clamp(0.0, 1.0) };

        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u16> = Vec::new();
        let line_width = 0.02;

        let push_line = |a: Vec3, b: Vec3, color: Color, vertices: &mut Vec<Vertex>, indices: &mut Vec<u16>| {
            let dir = (b - a).normalize();
            let perp = dir.cross(vec3(0.0, 1.0, 0.0));
            let perp = if perp.length_squared() > 0.001 {
                perp.normalize() * line_width
            } else {
                dir.cross(vec3(1.0, 0.0, 0.0)).normalize() * line_width
            };

            let c: [u8; 4] = color.into();
            let base = vertices.len() as u16;
            vertices.extend_from_slice(&[
                Vertex { position: a + perp, uv: Vec2::ZERO, color: c, normal: Vec4::ZERO },
                Vertex { position: a - perp, uv: Vec2::ZERO, color: c, normal: Vec4::ZERO },
                Vertex { position: b + perp, uv: Vec2::ZERO, color: c, normal: Vec4::ZERO },
                Vertex { position: b - perp, uv: Vec2::ZERO, color: c, normal: Vec4::ZERO },
            ]);
            indices.extend_from_slice(&[base, base+1, base+2, base+1, base+3, base+2]);

            if indices.len() > 63000 {
                draw_mesh(&Mesh { vertices: std::mem::take(vertices), indices: std::mem::take(indices), texture: None });
            }
        };

        for i in 0..=steps {
            let t = -grid_size + i as f32 * grid_spacing;

            let xz_color = Color::new(0.5, 0.5, 0.5, fade((vec3(0.0, 0.0, oz + t) - camera_pos).length()) * 0.8 + 0.1);
            let xy_color = Color::new(0.4, 0.4, 0.5, fade((vec3(0.0, oy + t, 0.0) - camera_pos).length()) * 0.7 + 0.1);
            let yz_color = Color::new(0.5, 0.4, 0.4, fade((vec3(ox + t, 0.0, 0.0) - camera_pos).length()) * 0.7 + 0.1);

            push_line(vec3(ox - grid_size, 0.0, oz + t), vec3(ox + grid_size, 0.0, oz + t), xz_color, &mut vertices, &mut indices);
            push_line(vec3(ox + t, 0.0, oz - grid_size), vec3(ox + t, 0.0, oz + grid_size), xz_color, &mut vertices, &mut indices);

            push_line(vec3(ox - grid_size, oy + t, 0.0), vec3(ox + grid_size, oy + t, 0.0), xy_color, &mut vertices, &mut indices);
            push_line(vec3(ox + t, oy - grid_size, 0.0), vec3(ox + t, oy + grid_size, 0.0), xy_color, &mut vertices, &mut indices);

            push_line(vec3(0.0, oy - grid_size, oz + t), vec3(0.0, oy + grid_size, oz + t), yz_color, &mut vertices, &mut indices);
            push_line(vec3(0.0, oy + t, oz - grid_size), vec3(0.0, oy + t, oz + grid_size), yz_color, &mut vertices, &mut indices);
        }

        push_line(vec3(ox - grid_size, 0.0, 0.0), vec3(ox + grid_size, 0.0, 0.0), BLUE, &mut vertices, &mut indices);
        push_line(vec3(0.0, oy - grid_size, 0.0), vec3(0.0, oy + grid_size, 0.0), BLUE, &mut vertices, &mut indices);
        push_line(vec3(0.0, 0.0, oz - grid_size), vec3(0.0, 0.0, oz + grid_size), BLUE, &mut vertices, &mut indices);

        if !vertices.is_empty() {
            draw_mesh(&Mesh { vertices, indices, texture: None });
        }
    }

    fn handle_input(&mut self, state: &mut State) {
        if state.ui_captures_keyboard {
            return;
        }

        if is_key_pressed(KeyCode::G) {
            state.show_grid = !state.show_grid;
            state.events.push(Event::Alert(format!("Grid: {}", if state.show_grid { "On" } else { "Off" })));
        }
    }

    fn update(&mut self, _dt: f32, _state: &mut State) {
        
    }
}