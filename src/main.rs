use macroquad::prelude::*;

mod input;
mod particle;
mod grid;
mod force;
mod draw;
mod setup;

use input::handle_input;
use particle::{Particle, update_particles_verlet};
use grid::draw_grid;
use draw::draw_particles;
use setup::{set_particles, time_function};

struct State {
    show_grid: bool,
    use_cubes: bool,
    show_trail: bool,
    use_time_function: bool,
    min_merge_mass: f32,
    restitution: f32,
    g: f32,
    yaw: f32,
    pitch: f32,
    pos: Vec3,
    camera: Camera3D,
    last_mouse: Option<(f32, f32)>,
    is_fullscreen: bool,
    time: f32,
    time_warp: f32,
    speed: f32,
    alert_flash: f32,
    alert_flash_duration: f32,
    alert_text: String,
    clock_running: bool,
}

fn window_conf() -> Conf {
    Conf {
        window_title: "MyGame".to_owned(),
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut state = State {
        show_grid: false,
        use_cubes: true,
        show_trail: true,
        use_time_function: false,
        min_merge_mass: f32::INFINITY,
        restitution: 0.5,
        g: 6.67430e-11,
        yaw: -135.0f32.to_radians(),
        pitch: -45.0f32.to_radians(),
        pos: vec3(15.0, 15.0, 15.0),
        camera: Camera3D::default(),
        last_mouse: None,
        is_fullscreen: false,
        time: 0.0,
        time_warp: 1.0,
        speed: 1.0,
        alert_flash: 0.0,
        alert_flash_duration: 2.0,
        alert_text: String::new(),
        clock_running: false,
    };

    let mut particles: Vec<Particle> = Vec::new();
    set_particles(&mut particles);

    loop {
        let dt = get_frame_time();
        handle_input(&mut state, &mut particles, dt);

        if state.alert_flash > 0.0 {
            state.alert_flash = (state.alert_flash - dt).max(0.0);
        }

        clear_background(Color::new(0.1, 0.1, 0.15, 1.0));
        set_camera(&state.camera);

        if state.show_grid {
            draw_grid(&state.camera);
        }

        if state.clock_running {
            state.time += dt * state.time_warp;

            let sim_dt = dt * state.time_warp;

            if state.use_time_function {
                time_function(&mut particles, state.time);
            } else {
                update_particles_verlet(
                    &mut particles,
                    sim_dt,
                    state.restitution,
                    state.min_merge_mass,
                    state.g,
                );
            }
        }

        draw_particles(&particles, state.use_cubes, state.show_trail);

        if state.alert_flash > 0.0 {
            set_default_camera();

            let alpha = (state.alert_flash / state.alert_flash_duration).clamp(0.0, 1.0);

            draw_text_ex(
                &state.alert_text,
                25.0,
                49.0,
                TextParams {
                    font_size: 36,
                    color: Color::new(0.0, 0.0, 0.0, alpha),
                    ..Default::default()
                },
            );

            draw_text_ex(
                &state.alert_text,
                24.0,
                48.0,
                TextParams {
                    font_size: 36,
                    color: Color::new(1.0, 1.0, 1.0, alpha),
                    ..Default::default()
                },
            );
        }

        next_frame().await
    }
}
