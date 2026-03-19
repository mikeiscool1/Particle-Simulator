use macroquad::prelude::*;
use crate::State;
use crate::Particle;
use crate::force::n_body_update;
use crate::setup::set_particles;

pub fn handle_input(state: &mut State, particles: &mut Vec<Particle>, dt: f32) {
// Full screen check
    if is_key_pressed(KeyCode::F11) {
        if !state.is_fullscreen {
            // Enter fullscreen
            set_fullscreen(true);
            state.is_fullscreen = true;
        } else {
            // Exit fullscreen
            set_fullscreen(false);
            state.is_fullscreen = false;
        }
    }

    if is_key_pressed(KeyCode::Space) {
        state.clock_running = !state.clock_running;

        if state.clock_running {
            state.alert_text = "Simulation Started".to_string();
        } else {
            state.alert_text = "Simulation Paused".to_string();
        }

        state.alert_flash = state.alert_flash_duration;
    }

    let sensitivity = 0.003;

    // Compute forward and right from yaw and pitch
    let forward = vec3(
        state.pitch.cos() * state.yaw.cos(),
        state.pitch.sin(),
        state.pitch.cos() * state.yaw.sin(),
    )
    .normalize();
    let right = forward.cross(vec3(0.0, 1.0, 0.0)).normalize();

    // Right click mouse look
    if is_mouse_button_down(MouseButton::Right) {
        let mouse = mouse_position();
        if let Some((lx, ly)) = state.last_mouse {
            let dx = mouse.0 - lx;
            let dy = mouse.1 - ly;
            state.yaw += dx * sensitivity;
            state.pitch -= dy * sensitivity;
            state.pitch = state.pitch.clamp(-89.0f32.to_radians(), 89.0f32.to_radians());
        }
        state.last_mouse = Some(mouse);
    } else {
        state.last_mouse = None;
    }

    // WASDEQ movement
    if is_key_down(KeyCode::W) { state.pos += forward * state.speed * 10.0 * dt; }
    if is_key_down(KeyCode::S) { state.pos -= forward * state.speed * 10.0 * dt; }
    if is_key_down(KeyCode::A) { state.pos -= right * state.speed * 10.0 * dt; }
    if is_key_down(KeyCode::D) { state.pos += right * state.speed * 10.0 * dt; }
    if is_key_down(KeyCode::E) { state.pos.y += state.speed * 10.0 * dt; }
    if is_key_down(KeyCode::Q) { state.pos.y -= state.speed * 10.0 * dt; }

    state.camera = Camera3D {
        position: state.pos,
        target: state.pos + forward,
        up: vec3(0.0, 1.0, 0.0),
        ..Default::default()
    };

    // Reset
    if is_key_pressed(KeyCode::R) {
        state.clock_running = false;
        state.time = 0.0;
        set_particles(particles);
        n_body_update(particles, state.g);

        state.alert_flash = state.alert_flash_duration;
        state.alert_text = "Simulation Reset".to_string();
    }

    // Setting toggles
    if is_key_pressed(KeyCode::F1) {
        state.time_warp *= 0.5;
        state.alert_flash = state.alert_flash_duration;
        state.alert_text = format!("Time Warp: {}x", format_dec(state.time_warp));
    }
    if is_key_pressed(KeyCode::F2) {
        state.time_warp *= 2.0;
        state.alert_flash = state.alert_flash_duration;
        state.alert_text = format!("Time Warp: {}x", format_dec(state.time_warp));
    }
    if is_key_pressed(KeyCode::G) {
        state.show_grid = !state.show_grid;
        state.alert_flash = state.alert_flash_duration;
        state.alert_text = format!("Grid: {}", if state.show_grid { "On" } else { "Off" });
    }
    if is_key_pressed(KeyCode::T) {
        state.show_trail = !state.show_trail;
        state.alert_flash = state.alert_flash_duration;
        state.alert_text = format!("Trails: {}", if state.show_trail { "On" } else { "Off" });
    }
    if is_key_pressed(KeyCode::C) {
        state.use_cubes = !state.use_cubes;
        state.alert_flash = state.alert_flash_duration;
        state.alert_text = format!("Render Mode: {}", if state.use_cubes { "Cubes" } else { "Spheres" });
    }

    if is_key_pressed(KeyCode::F3) {
        state.speed *= 0.5;
        state.alert_flash = state.alert_flash_duration;
        state.alert_text = format!("Speed: {}x", format_dec(state.speed));
    }
    if is_key_pressed(KeyCode::F4) {
        state.speed *= 2.0;
        state.alert_flash = state.alert_flash_duration;
        state.alert_text = format!("Speed: {}x", format_dec(state.speed));
    }
    if is_key_pressed(KeyCode::O) {
        state.yaw = -135.0f32.to_radians();
        state.pitch = -45.0f32.to_radians();
        state.pos = vec3(15.0, 15.0, 15.0);
        state.alert_flash = state.alert_flash_duration;
        state.alert_text = "Camera Reset".to_string();
    }
}

fn format_dec(value: f32) -> String {
    let formatted = format!("{:.6}", value);
    formatted.trim_end_matches('0').trim_end_matches('.').to_string()
}