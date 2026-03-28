use core::f32;

use macroquad::prelude::*;

mod component;
mod input;
mod force;
mod setup;

use component::{Component, Grid, Particles, Alert, Editor};

use input::handle_input;
use setup::set_particles;

struct State {
    yaw: f32, // Start camera yaw (rotation around the Y axis)
    pitch: f32, // Start camera pitch (rotation around the X axis)
    pos: Vec3, // Start camera position
    camera: Camera3D,
    last_mouse: Option<(f32, f32)>,
    is_fullscreen: bool, // Whether the window is currently in fullscreen mode
    speed: f32, // Camera speed
    time_warp: f32,
    clock_running: bool, // Whether the simulation clock is running
    editor_panel_width: f32,
    ui_captures_keyboard: bool,
    ui_captures_pointer: bool,
    bg_color: Color,
    events: Vec<component::Event>, // Events that components can trigger to communicate with each other
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
        yaw: -135.0f32.to_radians(),
        pitch: -45.0f32.to_radians(),
        pos: vec3(15.0, 15.0, 15.0),
        camera: Camera3D::default(),
        last_mouse: None,
        is_fullscreen: false,
        speed: 1.0,
        time_warp: 1.0,
        clock_running: false,
        editor_panel_width: 0.0,
        ui_captures_keyboard: false,
        ui_captures_pointer: false,
        bg_color: Color::new(0.1, 0.1, 0.15, 1.0),
        events: Vec::new(),
    };

    let mut particles = Particles { 
        particles: Vec::new(), 
        show_trail: true, 
        use_cubes: true,
        min_merge_mass: f32::INFINITY,
        restitution: 0.5,
        g: 6.67430e-11,
        use_parametric: false,
        time: 0.0,
        parametric_equations: None,
    };
    set_particles(&mut particles.particles);

    let mut grid = Grid { visible: false };
    let mut alert = Alert::new();
    let mut fps = component::FPS { visible: false, last_update_time: 0.0, fps: get_fps() };
    let mut editor = Editor::new(true, component::editor::EditorMode::Particles);

    editor.try_compile_parametric(&mut particles, &mut state);

    loop {
        let dt = get_frame_time();

        state.ui_captures_keyboard = false;
        state.ui_captures_pointer = false;

        egui_macroquad::ui(|ctx| {
            editor.draw_egui(ctx, &mut particles, &mut state);
            state.ui_captures_keyboard = ctx.wants_keyboard_input();
            state.ui_captures_pointer = ctx.wants_pointer_input();
        });

        // Inputs
        handle_input(&mut state, &mut particles.particles, dt);

        particles.handle_input(&mut state);
        grid.handle_input(&mut state);
        fps.handle_input(&mut state);
        editor.handle_input(&mut state);

        // Handle events
        for event in state.events.drain(..) {
            match event {
                component::Event::Alert(message) => alert.alert(&message),
                component::Event::ResetSimulation => particles.time = 0.0
            }
        }

        // Updates, Animations
        alert.update(dt, &mut state);
        particles.update(dt, &mut state);
        fps.update(dt, &mut state);

        // Draw
        clear_background(state.bg_color);
        set_camera(&state.camera);
        
        grid.draw(&state);
        particles.draw(&state);
        fps.draw(&state);
        alert.draw(&state);

        // Draw egui on top of everything
        egui_macroquad::draw();

        next_frame().await
    }
}
