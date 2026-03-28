use egui_macroquad::egui;
use macroquad::prelude::*;

use crate::component::{Component, Event, particles::{ParametricEquations, Particle, Particles, compile_parametric_fn}};
use crate::State;

pub struct Editor {
    pub visible: bool,
    expanded_particles: Vec<bool>,
    pub x_expr: String,
    pub y_expr: String,
    pub z_expr: String,
    pub spread_expr: String,
    x_expr_error: bool,
    y_expr_error: bool,
    z_expr_error: bool,
    spread_expr_error: bool,
    parametric_error: Option<String>,
    merge_enabled: bool,
    merge_mass_threshold: f32,
}

impl Editor {
    pub fn new(visible: bool) -> Self {
        Self {
            visible,
            expanded_particles: Vec::new(),
            x_expr: "3 * sin(t)".to_string(),
            y_expr: "2 * sin(2 * t)".to_string(),
            z_expr: "sin(3 * t)".to_string(),
            spread_expr: "0.01".to_string(),
            x_expr_error: false,
            y_expr_error: false,
            z_expr_error: false,
            spread_expr_error: false,
            parametric_error: None,
            merge_enabled: false,
            merge_mass_threshold: 1e10,
        }
    }

    pub fn draw_egui(&mut self, ctx: &egui::Context, particles: &mut Particles, state: &mut State) {
        if !self.visible {
            state.editor_panel_width = 0.0;
            return;
        }

        let n = particles.particles.len();
        if self.expanded_particles.len() < n {
            self.expanded_particles.resize(n, false);
        }

        let panel_response = egui::SidePanel::left("editor_panel")
            .resizable(true)
            .default_width(280.0)
            .min_width(160.0)
            .show(ctx, |ui| {
                ui.add_space(5.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::CollapsingHeader::new("Options")
                        .default_open(false)
                        .show(ui, |ui| {
                            self.draw_options(ui, particles, state);
                        });

                    ui.separator();

                    egui::CollapsingHeader::new("Parametric")
                        .default_open(false)
                        .show(ui, |ui| {
                            self.draw_parametric(ui, particles, state);
                        });

                    ui.separator();

                    egui::CollapsingHeader::new(format!("Particles ({})", particles.particles.len()))
                        .id_salt("particles_list_header")
                        .default_open(false)
                        .show(ui, |ui| {
                            self.draw_particles(ui, particles);
                        });

                    ui.separator();

                    egui::CollapsingHeader::new("Help")
                        .default_open(false)
                        .show(ui, |ui| {
                            self.draw_help(ui);
                        });
                });
            });

        state.editor_panel_width = panel_response.response.rect.width();
    }

    fn draw_options(&mut self, ui: &mut egui::Ui, particles: &mut Particles, state: &mut State) {
        ui.label("Scene");
        egui::Grid::new("options_scene")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("Background");
                let mut rgba = [
                    state.bg_color.r,
                    state.bg_color.g,
                    state.bg_color.b,
                    state.bg_color.a,
                ];
                if ui.color_edit_button_rgba_unmultiplied(&mut rgba).changed() {
                    state.bg_color = Color::new(rgba[0], rgba[1], rgba[2], rgba[3]);
                }
                ui.end_row();
            });

        ui.separator();

        ui.label("Simulation");
        egui::Grid::new("options_sim")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("Clock running");
                ui.checkbox(&mut state.clock_running, "");
                ui.end_row();

                ui.label("Time warp");
                ui.add(
                    egui::DragValue::new(&mut state.time_warp)
                        .speed(0.01)
                        .range(0.001..=1000.0_f32),
                );
                ui.end_row();

                ui.label("Camera speed");
                ui.add(
                    egui::DragValue::new(&mut state.speed)
                        .speed(0.01)
                        .range(0.001..=1000.0_f32),
                );
                ui.end_row();

                ui.label("Elapsed time");
                ui.add(egui::DragValue::new(&mut particles.time).speed(0.1));
                ui.end_row();

                ui.label("Use parametric");
                ui.checkbox(&mut particles.use_parametric, "");
                ui.end_row();
            });

        ui.separator();

        ui.label("Physics");
        egui::Grid::new("options_physics")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("Restitution");
                ui.add(
                    egui::DragValue::new(&mut particles.restitution)
                        .speed(0.005)
                        .range(0.0..=1.0_f32),
                );
                ui.end_row();

                ui.label("Gravity (G)");
                ui.add(
                    egui::DragValue::new(&mut particles.g)
                        .speed(1e-13)
                        .range(0.0..=1e-6_f32)
                        .custom_formatter(|v, _| format!("{:.3e}", v))
                        .custom_parser(|s| s.parse::<f64>().ok()),
                );
                ui.end_row();

                ui.label("Merging");
                if ui.checkbox(&mut self.merge_enabled, "").changed() {
                    particles.min_merge_mass = if self.merge_enabled {
                        self.merge_mass_threshold
                    } else {
                        f32::INFINITY
                    };
                }
                ui.end_row();

                if self.merge_enabled {
                    ui.label("Min merge mass");
                    if ui
                        .add(
                            egui::DragValue::new(&mut self.merge_mass_threshold)
                                .speed(1e6)
                                .range(0.0..=1e30_f32),
                        )
                        .changed()
                    {
                        particles.min_merge_mass = self.merge_mass_threshold;
                    }
                    ui.end_row();
                }
            });

        ui.separator();

        ui.label("Rendering");
        egui::Grid::new("options_render")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("Show trails");
                ui.checkbox(&mut particles.show_trail, "");
                ui.end_row();

                ui.label("Use cubes");
                ui.checkbox(&mut particles.use_cubes, "");
                ui.end_row();
            });
    }

    fn draw_particles(&mut self, ui: &mut egui::Ui, particles: &mut Particles) {
        ui.horizontal(|ui| {
            if ui.button("+ Add Particle").clicked() {
                particles.particles.push(Particle::default());
                self.expanded_particles.push(false);
            }
            if !particles.particles.is_empty() && ui.button("Delete All").clicked() {
                particles.particles.clear();
                self.expanded_particles.clear();
            }
        });

        ui.separator();

        let mut to_delete: Option<usize> = None;

        for i in 0..particles.particles.len() {
            let expanded = *self.expanded_particles.get(i).unwrap_or(&false);

            ui.horizontal(|ui| {
                let row_label = format!("Particle {}", i + 1);
                if ui.selectable_label(expanded, row_label).clicked() {
                    if let Some(slot) = self.expanded_particles.get_mut(i) {
                        *slot = !*slot;
                    }
                }
                if ui.small_button("X").clicked() {
                    to_delete = Some(i);
                }
            });

            if expanded {
                ui.indent(("particle_props", i), |ui| {
                    let p = &mut particles.particles[i];

                    ui.label("Position");
                    ui.horizontal(|ui| {
                        ui.label("x:");
                        ui.add(egui::DragValue::new(&mut p.pos.x).speed(0.1));
                        ui.label("y:");
                        ui.add(egui::DragValue::new(&mut p.pos.y).speed(0.1));
                        ui.label("z:");
                        ui.add(egui::DragValue::new(&mut p.pos.z).speed(0.1));
                    });

                    ui.label("Velocity");
                    ui.horizontal(|ui| {
                        ui.label("x:");
                        ui.add(egui::DragValue::new(&mut p.vel.x).speed(0.1));
                        ui.label("y:");
                        ui.add(egui::DragValue::new(&mut p.vel.y).speed(0.1));
                        ui.label("z:");
                        ui.add(egui::DragValue::new(&mut p.vel.z).speed(0.1));
                    });

                    ui.label("Mass");
                    ui.add(
                        egui::DragValue::new(&mut p.mass)
                            .speed(1e6)
                            .range(0.0..=1e30_f32),
                    );

                    ui.label("Radius");
                    ui.add(
                        egui::DragValue::new(&mut p.radius)
                            .speed(0.01)
                            .range(0.001..=1e6_f32),
                    );

                    ui.label("Friction");
                    ui.add(
                        egui::DragValue::new(&mut p.friction)
                            .speed(0.005)
                            .range(0.0..=1.0_f32),
                    );

                    ui.label("Color");
                    let mut rgba = [p.color.r, p.color.g, p.color.b, p.color.a];
                    if ui
                        .color_edit_button_rgba_unmultiplied(&mut rgba)
                        .changed()
                    {
                        p.color = Color::new(rgba[0], rgba[1], rgba[2], rgba[3]);
                    }
                });
                ui.separator();
            }
        }

        if let Some(idx) = to_delete {
            particles.particles.remove(idx);
            if idx < self.expanded_particles.len() {
                self.expanded_particles.remove(idx);
            }
        }
    }

    fn draw_parametric(&mut self, ui: &mut egui::Ui, particles: &mut Particles, state: &mut State) {
        let mut changed = false;

        egui::Grid::new("parametric_grid")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.label("x =");
                changed |= draw_parametric_row(ui, &mut self.x_expr, self.x_expr_error, "e.g. sin(t)");
                ui.end_row();

                ui.label("y =");
                changed |= draw_parametric_row(ui, &mut self.y_expr, self.y_expr_error, "e.g. cos(t)");
                ui.end_row();

                ui.label("z =");
                changed |= draw_parametric_row(ui, &mut self.z_expr, self.z_expr_error, "e.g. t");
                ui.end_row();

                ui.label("Spread = ");
                changed |= draw_parametric_row(ui, &mut self.spread_expr, self.spread_expr_error, "e.g. 0.1");
                ui.end_row();
            });

        if changed {
            self.try_compile_parametric(particles, state);
        }

        if let Some(error) = &self.parametric_error {
            ui.colored_label(egui::Color32::RED, error);
        }
        ui.label("Variables:\n- t: elapsed time\n- x: current x position\n- y: current y position\n- z: current z position");
    }

    fn draw_help(&self, ui: &mut egui::Ui) {
        ui.label("Move: WASD, Right click + drag, E/Q");
        ui.separator();
        ui.label("Fullscreen: F11");
        ui.separator();
        ui.label("Start/Stop simulation: Space");
        ui.separator();
        ui.label("Reset simulation: r");
        ui.separator();
        ui.label("Reset camera: o");
        ui.separator();
        ui.label("Show/Hide particle trails: t");
        ui.separator();
        ui.label("Show/Hide grid: g");
        ui.separator();
        ui.label("Toggle sphere/cube rendering: c");
        ui.label("Tip: use cube rendering for many particles to reduce lag");
        ui.separator();
        ui.label("Toggle simulation/parametric mode: /");
        ui.separator();
        ui.label("Hide editor: p");
        ui.separator();
        ui.label("Slow time: F1");
        ui.separator();
        ui.label("Speed up time: F2");
        ui.separator();
        ui.label("Slow camera speed: F3");
        ui.separator();
        ui.label("Speed up camera speed: F4");
    }

    pub fn try_compile_parametric(&mut self, particles: &mut Particles, state: &mut State) {
        let x_fn_result = compile_parametric_fn(&self.x_expr);
        self.x_expr_error = x_fn_result.is_err();
        let y_fn_result = compile_parametric_fn(&self.y_expr);
        self.y_expr_error = y_fn_result.is_err();
        let z_fn_result = compile_parametric_fn(&self.z_expr);
        self.z_expr_error = z_fn_result.is_err();

        let spread = self.spread_expr.parse::<f64>().unwrap_or(0.0);
        self.spread_expr_error = spread.is_nan() || spread.is_infinite() || spread < 0.0;
        if self.spread_expr_error {
            self.parametric_error = Some("Spread must be a non-negative number".to_string());
            return;
        }

        if self.x_expr_error {
            self.parametric_error = Some(format!("x: {}", x_fn_result.err().unwrap()));
        } else if self.y_expr_error {
            self.parametric_error = Some(format!("y: {}", y_fn_result.err().unwrap()));
        } else if self.z_expr_error {
            self.parametric_error = Some(format!("z: {}", z_fn_result.err().unwrap()));
        } else if self.spread_expr_error {
            self.parametric_error = Some("Spread: Invalid number".to_string());
        } else {
            self.parametric_error = None;
            let was_none = particles.parametric_equations.is_none();
            particles.parametric_equations = Some(ParametricEquations {
                x_fn: x_fn_result.unwrap(),
                y_fn: y_fn_result.unwrap(),
                z_fn: z_fn_result.unwrap(),
                spread,
            });
            
            if !was_none {
                state.events.push(Event::Alert("Parametric equations updated".to_string()));
            }
        }
    }
}

fn draw_parametric_row(ui: &mut egui::Ui, expr: &mut String, has_error: bool, hint: &str) -> bool {
    let frame = if has_error {
        egui::Frame::default()
            .fill(egui::Color32::from_rgb(70, 20, 20))
            .stroke(egui::Stroke::new(1.0, egui::Color32::RED))
    } else {
        egui::Frame::default()
    };

    frame
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(expr)
                    .hint_text(hint)
                    .desired_width(f32::INFINITY),
            )
        })
        .inner
        .changed()
}


impl Component for Editor {
    fn draw(&self, _state: &State) {

    }

    fn handle_input(&mut self, _state: &mut State) {
        if _state.ui_captures_keyboard {
            return;
        }

        if is_key_pressed(KeyCode::P) {
            self.visible = !self.visible;
        }
    }

    fn update(&mut self, _dt: f32, _state: &mut State) {
        // No dynamic behavior to update
    }
}