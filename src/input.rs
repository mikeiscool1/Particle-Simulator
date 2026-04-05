use crate::State;
use crate::component::Event;
use macroquad::prelude::*;

pub fn handle_input(state: &mut State, dt: f32) {
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

    let forward = vec3(
        state.pitch.cos() * state.yaw.cos(),
        state.pitch.sin(),
        state.pitch.cos() * state.yaw.sin(),
    )
    .normalize();

    state.camera = Camera3D {
        position: state.pos,
        target: state.pos + forward,
        up: vec3(0.0, 1.0, 0.0),
        ..Default::default()
    };

    if state.ui_captures_keyboard {
        return;
    }

    if is_key_pressed(KeyCode::Space) {
        state.clock_running = !state.clock_running;

        if state.clock_running {
            state
                .events
                .push(Event::Alert("Simulation Running".to_string()));
        } else {
            state
                .events
                .push(Event::Alert("Simulation Paused".to_string()));
        }
    }

    if state.ui_captures_keyboard {
        state.camera = Camera3D {
            position: state.pos,
            target: state.pos
                + vec3(
                    state.pitch.cos() * state.yaw.cos(),
                    state.pitch.sin(),
                    state.pitch.cos() * state.yaw.sin(),
                )
                .normalize(),
            up: vec3(0.0, 1.0, 0.0),
            ..Default::default()
        };
        return;
    }

    let sensitivity = 0.003;

    // Compute forward and right from yaw and pitch
    let right = forward.cross(vec3(0.0, 1.0, 0.0)).normalize();

    // Right click mouse look
    if !state.ui_captures_pointer && is_mouse_button_down(MouseButton::Right) {
        let mouse = mouse_position();
        if let Some((lx, ly)) = state.last_mouse {
            let dx = mouse.0 - lx;
            let dy = mouse.1 - ly;
            state.yaw += dx * sensitivity;
            state.pitch -= dy * sensitivity;
            state.pitch = state
                .pitch
                .clamp(-89.0f32.to_radians(), 89.0f32.to_radians());
        }
        state.last_mouse = Some(mouse);
    } else {
        state.last_mouse = None;
    }

    // Arrow key look
    if is_key_down(KeyCode::Up) {
        state.pitch += 2.0 * sensitivity;
    }
    if is_key_down(KeyCode::Down) {
        state.pitch -= 2.0 * sensitivity;
    }
    if is_key_down(KeyCode::Left) {
        state.yaw -= 2.0 * sensitivity;
    }
    if is_key_down(KeyCode::Right) {
        state.yaw += 2.0 * sensitivity;
    }

    // WASDEQ movement
    if is_key_down(KeyCode::W) {
        state.pos += forward * state.speed * 10.0 * dt;
    }
    if is_key_down(KeyCode::S) {
        state.pos -= forward * state.speed * 10.0 * dt;
    }
    if is_key_down(KeyCode::A) {
        state.pos -= right * state.speed * 10.0 * dt;
    }
    if is_key_down(KeyCode::D) {
        state.pos += right * state.speed * 10.0 * dt;
    }
    if is_key_down(KeyCode::E) {
        state.pos.y += state.speed * 10.0 * dt;
    }
    if is_key_down(KeyCode::Q) {
        state.pos.y -= state.speed * 10.0 * dt;
    }
    // Reset
    if is_key_pressed(KeyCode::R) {
        state.events.push(Event::ResetSimulation);
    }

    // Setting toggles
    if is_key_pressed(KeyCode::F1) {
        state.time_warp *= 0.5;
        state.events.push(Event::Alert(format!(
            "Time Warp: {}x",
            format_dec(state.time_warp)
        )));
    }
    if is_key_pressed(KeyCode::F2) {
        state.time_warp *= 2.0;
        state.events.push(Event::Alert(format!(
            "Time Warp: {}x",
            format_dec(state.time_warp)
        )));
    }
    if is_key_pressed(KeyCode::F3) {
        state.speed *= 0.5;
        state
            .events
            .push(Event::Alert(format!("Speed: {}x", format_dec(state.speed))));
    }
    if is_key_pressed(KeyCode::F4) {
        state.speed *= 2.0;
        state
            .events
            .push(Event::Alert(format!("Speed: {}x", format_dec(state.speed))));
    }
    if is_key_pressed(KeyCode::O) {
        state.yaw = -135.0f32.to_radians();
        state.pitch = -45.0f32.to_radians();
        state.pos = vec3(15.0, 15.0, 15.0);
        state.events.push(Event::Alert("Camera Reset".to_string()));
    }
}

fn format_dec(value: f32) -> String {
    let formatted = format!("{:.6}", value);
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}
