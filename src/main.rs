use core::f32;
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

mod component;
mod input;
mod force;
mod setup;
mod serde_helper;

use component::{Component, Grid, Particles, Alert, Editor};
use input::handle_input;
use setup::set_particles;
use serde_helper::serde_color;

#[derive(Serialize, Deserialize)]
struct State {
    yaw: f32,
    pitch: f32,
    pos: Vec3,
    time_warp: f32,
    #[serde(with = "serde_color")]
    bg_color: Color,
    show_grid: bool,

    #[serde(skip_serializing, skip_deserializing, default)]
    camera: Camera3D,
    #[serde(skip_serializing, skip_deserializing, default)]
    last_mouse: Option<(f32, f32)>,
    #[serde(skip_serializing, skip_deserializing, default)]
    is_fullscreen: bool,
    #[serde(skip_serializing, skip_deserializing, default)]
    speed: f32, // Camera speed
    #[serde(skip_serializing, skip_deserializing, default)]
    clock_running: bool, // Whether the simulation clock is running
    #[serde(skip_serializing, skip_deserializing, default)]
    editor_panel_width: f32,
    #[serde(skip_serializing, skip_deserializing, default)]
    ui_captures_keyboard: bool,
    #[serde(skip_serializing, skip_deserializing, default)]
    ui_captures_pointer: bool,
    #[serde(skip_serializing, skip_deserializing, default)]
    events: Vec<component::Event>, // Events that components can trigger to communicate with each other
}

impl State {
    fn apply_state_save(&mut self, loaded_state: State) {
        self.yaw = loaded_state.yaw;
        self.pitch = loaded_state.pitch;
        self.pos = loaded_state.pos;
        self.bg_color = loaded_state.bg_color;
        self.show_grid = loaded_state.show_grid;
        self.time_warp = loaded_state.time_warp;
        self.clock_running = false;
    }
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
        show_grid: false,
        events: Vec::new(),
    };

    let mut particles = Particles { 
        particles: Vec::new(), 
        show_trail: true, 
        use_cubes: true,
        min_merge_mass: -1.0,
        g: 6.67430e-11,
        use_parametric: false,
        time: 0.0,
        parametric_equations: Vec::new(),
    };
    set_particles(&mut particles.particles);

    let mut grid = Grid {};
    let mut alert = Alert::new();
    let mut fps = component::FPS { visible: false, last_update_time: 0.0, fps: get_fps() };
    let mut editor = Editor::new(true);

    let _ = editor.try_compile_parametric(&mut particles);

    loop {
        let dt = get_frame_time().min(0.1);

        state.ui_captures_keyboard = false;
        state.ui_captures_pointer = false;

        egui_macroquad::ui(|ctx| {
            editor.draw_egui(ctx, &mut particles, &mut state);
            state.ui_captures_keyboard = ctx.wants_keyboard_input();
            state.ui_captures_pointer = ctx.wants_pointer_input();
        });

        // Inputs
        handle_input(&mut state, dt);

        particles.handle_input(&mut state);
        grid.handle_input(&mut state);
        fps.handle_input(&mut state);
        editor.handle_input(&mut state);

        // Handle events
        for event in state.events.drain(..) {
            match event {
                component::Event::Alert(message) => alert.alert(&message),
                component::Event::ResetSimulation => {
                    state.clock_running = false;
                    particles.time = 0.0;
                    if !particles.use_parametric {
                        set_particles(&mut particles.particles);
                    } else {
                        for parametric in &particles.parametric_equations {
                            parametric.apply_to_particles(&mut particles.particles, particles.time);
                        }

                    }
                    alert.alert("Simulation Reset");
                }
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
