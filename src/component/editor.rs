use egui_macroquad::egui;
use macroquad::prelude::*;
use serde::{Serialize, Deserialize};

use serde_json::json;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

#[cfg(target_arch = "wasm32")]
use quad_storage::STORAGE;

use crate::component::{Component, Event, particles::{DomainLoopDirection, ParametricEquations, Particle, Particles, compile_parametric_fn, insert_implicit_mul}};
use crate::State;

#[derive(Serialize, Deserialize)]
struct Save {
    state: State,
    particles: Particles,
    editor: Editor,
}

#[derive(Serialize, Deserialize)]
struct ParametricEquationEditor {
    x_expr: String,
    y_expr: String,
    z_expr: String,
    spread_expr: String,
    #[serde(skip_serializing, skip_deserializing, default)]
    x_expr_error: bool,
    #[serde(skip_serializing, skip_deserializing, default)]
    y_expr_error: bool,
    #[serde(skip_serializing, skip_deserializing, default)]
    z_expr_error: bool,
    #[serde(skip_serializing, skip_deserializing, default)]
    spread_expr_error: bool,
    use_domain: bool,
    domain_lower_expr: String,
    domain_upper_expr: String,
    #[serde(skip_serializing, skip_deserializing, default)]
    domain_lower_expr_error: bool,
    #[serde(skip_serializing, skip_deserializing, default)]
    domain_upper_expr_error: bool,
    domain_direction: DomainLoopDirection,
    #[serde(skip_serializing, skip_deserializing, default)]
    error: Option<String>,
    num_particles: usize,
    running: bool,
    hidden: bool,
}

impl Default for ParametricEquationEditor {
    fn default() -> Self {
        Self {
            x_expr: "0".to_string(),
            y_expr: "0".to_string(),
            z_expr: "0".to_string(),
            spread_expr: "0.01".to_string(),
            x_expr_error: false,
            y_expr_error: false,
            z_expr_error: false,
            spread_expr_error: false,
            use_domain: true,
            error: None,
            domain_lower_expr: "0.0".to_string(),
            domain_upper_expr: "10.0".to_string(),
            domain_lower_expr_error: false,
            domain_upper_expr_error: false,
            domain_direction: DomainLoopDirection::Wrap,
            num_particles: 10,
            running: true,
            hidden: false,
        }
    }
}

impl ParametricEquationEditor {
    fn example() -> Self {
        Self {
            x_expr: "3sin(t)".to_string(),
            y_expr: "2sin(2t)".to_string(),
            z_expr: "sin(3t)".to_string(),
            spread_expr: "0.01".to_string(),
            x_expr_error: false,
            y_expr_error: false,
            z_expr_error: false,
            spread_expr_error: false,
            use_domain: false,
            error: None,
            domain_lower_expr: "0.0".to_string(),
            domain_upper_expr: "10.0".to_string(),
            domain_lower_expr_error: false,
            domain_upper_expr_error: false,
            domain_direction: DomainLoopDirection::Wrap,
            num_particles: 1000,
            running: true,
            hidden: false,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Editor {
    pub visible: bool,
    merge_enabled: bool,
    merge_mass_threshold: f32,
    parametric_equations: Vec<ParametricEquationEditor>,
    
    #[serde(skip_serializing, skip_deserializing, default)]
    expanded_particles: Vec<bool>,
    #[serde(skip_serializing, skip_deserializing, default)]
    expanded_parametric: Vec<bool>,
    #[serde(skip_serializing, skip_deserializing, default)]
    #[cfg(not(target_arch = "wasm32"))]
    config_dir: Option<PathBuf>,
    #[serde(skip_serializing, skip_deserializing, default)]
    saves_list: Vec<String>,
    #[serde(skip_serializing, skip_deserializing, default)]
    parametric_error: Option<String>,
}

impl Editor {
    pub fn new(visible: bool) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let config_dir: Option<PathBuf>;

        let saves_list: Vec<String>;

        #[cfg(not(target_arch = "wasm32"))]
        {
            config_dir = directories::ProjectDirs::from("", "", "ParticleSimulator").map(|dirs| dirs.config_dir().to_path_buf());

            if let Some(dir) = &config_dir {
                std::fs::create_dir_all(dir).unwrap_or_else(|err| {
                    eprintln!("Failed to create config directory {:?}: {}", dir, err);
                });
            }

            saves_list = if let Some(config_dir) = &config_dir {
                std::fs::read_dir(config_dir)
                    .map(|entries| {
                        entries.filter_map(|entry| {
                            entry.ok().and_then(|e| {
                                let path = e.path();
                                if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                                    path.file_stem().and_then(|stem| stem.to_str()).map(|s| s.to_string())
                                } else {
                                    None
                                }
                            })
                        }).collect()
                    })
                    .unwrap_or_else(|_| Vec::new())
            } else {
                Vec::new()
            };
        }
        #[cfg(target_arch = "wasm32")]
        {
            let mut storage = STORAGE.lock().unwrap();
            saves_list = serde_json::from_str(&storage.get("saves_list")
                .unwrap_or_else(|| { storage.set("saves_list", "[]"); "[]".to_string() }))
                .unwrap_or_else(|_| Vec::new());
        }

        Self {
            visible,
            expanded_particles: Vec::new(),
            expanded_parametric: vec![false],
            parametric_equations: vec![ParametricEquationEditor::example()],
            parametric_error: None,
            merge_enabled: false,
            merge_mass_threshold: 1e10,

            #[cfg(not(target_arch = "wasm32"))]
            config_dir,

            saves_list
        }
    }

    pub fn apply_editor_save(&mut self, loaded_editor: Editor) {
        self.parametric_equations = loaded_editor.parametric_equations;
        self.merge_enabled = loaded_editor.merge_enabled;
        self.merge_mass_threshold = loaded_editor.merge_mass_threshold;
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

        let n_parametric = self.parametric_equations.len();
        if self.expanded_parametric.len() < n_parametric {
            self.expanded_parametric.resize(n_parametric, false);
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

                    egui::CollapsingHeader::new("Saves")
                        .default_open(false)
                        .show(ui, |ui| {
                            self.draw_saves(ui, particles, state);
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

                ui.label("Show Grid");
                ui.checkbox(&mut state.show_grid, "");
                ui.end_row();
            });

        ui.separator();

        ui.label("Camera");
        egui::Grid::new("options_camera")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("Position");
                ui.add(egui::DragValue::new(&mut state.pos.x).speed(0.1));
                ui.add(egui::DragValue::new(&mut state.pos.y).speed(0.1));
                ui.add(egui::DragValue::new(&mut state.pos.z).speed(0.1));
                ui.end_row();

                ui.label("Yaw");
                ui.add(egui::DragValue::new(&mut state.yaw).speed(0.1));
                ui.end_row();

                ui.label("Pitch");
                ui.add(egui::DragValue::new(&mut state.pitch).speed(0.1));
                ui.end_row();

                ui.label("Speed");
                ui.add(
                    egui::DragValue::new(&mut state.speed)
                        .speed(0.01)
                        .range(0.001..=1000.0_f32),
                );
                ui.end_row();
            });

        ui.separator();

        ui.label("Simulation");
        egui::Grid::new("options_sim")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("Clock Running");
                ui.checkbox(&mut state.clock_running, "");
                ui.end_row();
                
                ui.label("Elapsed Time");
                ui.add(egui::DragValue::new(&mut particles.time).speed(0.1));
                ui.end_row();

                ui.label("Time Warp");
                ui.add(
                    egui::DragValue::new(&mut state.time_warp)
                        .speed(0.01)
                        .range(0.001..=1000.0_f32),
                );
                ui.end_row();

                ui.label("Use Parametric");
                let was_parametric = particles.use_parametric;
                if ui.checkbox(&mut particles.use_parametric, "").changed() {
                    // Unhide everything on mode switch; re-hide unused if entering parametric
                    for p in &mut particles.particles {
                        p.hidden = false;
                    }
                    if particles.use_parametric {
                        let used: usize = particles.parametric_equations.iter().map(|eq| eq.particle_indices.len()).sum();
                        for i in used..particles.particles.len() {
                            particles.particles[i].hidden = true;
                        }
                    }
                }
                let _ = was_parametric;
                ui.end_row();

                if ui.button("Reset").clicked() {
                    state.events.push(Event::ResetSimulation);
                }
            });

        ui.separator();

        ui.label("Physics");
        egui::Grid::new("options_physics")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
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
                        -1.0
                    };
                }
                ui.end_row();

                if self.merge_enabled {
                    ui.label("Min Merge Mass");
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
                ui.label("Show Trails");
                ui.checkbox(&mut particles.show_trail, "");
                ui.end_row();

                ui.label("Use Cubes");
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
            let is_hidden = particles.particles[i].hidden;

            ui.horizontal(|ui| {
                let row_label = format!("Particle {}", i + 1);
                let text_color = if is_hidden {
                    egui::Color32::from_gray(128)
                } else {
                    egui::Color32::LIGHT_GRAY
                };
                if ui.selectable_label(expanded, egui::RichText::new(&row_label).color(text_color)).clicked() {
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

                    ui.label("Restitution");
                    ui.add(
                        egui::DragValue::new(&mut p.restitution)
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

                    ui.horizontal(|ui| {
                        ui.label("Hidden");
                        ui.checkbox(&mut p.hidden, "");
                    });
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

        ui.horizontal(|ui| {
            if ui.button("+ Add Equation").clicked() {
                self.parametric_equations.push(ParametricEquationEditor::default());
                self.expanded_parametric.push(false);
                changed = true;
            }
            if !self.parametric_equations.is_empty() && ui.button("Delete All").clicked() {
                self.parametric_equations.clear();
                self.expanded_parametric.clear();
                changed = true;
            }
        });

        ui.separator();

        let mut to_delete: Option<usize> = None;

        for i in 0..self.parametric_equations.len() {
            let expanded = *self.expanded_parametric.get(i).unwrap_or(&false);
            let is_running = self.parametric_equations[i].running;
            let is_hidden = self.parametric_equations[i].hidden;

            ui.horizontal(|ui| {
                let label_color = if is_hidden {
                    egui::Color32::from_gray(110)
                } else {
                    egui::Color32::LIGHT_GRAY
                };
                let row_label = egui::RichText::new(format!("Equation {}", i + 1)).color(label_color);
                if ui.selectable_label(expanded, row_label).clicked() {
                    if let Some(slot) = self.expanded_parametric.get_mut(i) {
                        *slot = !*slot;
                    }
                }

                let play_label = if is_running { "⏸" } else { "▶" };
                if ui.small_button(play_label).clicked() {
                    self.parametric_equations[i].running = !self.parametric_equations[i].running;
                    changed = true;
                }

                let hide_label = if is_hidden { "👁" } else { "🚫" };
                if ui.small_button(hide_label).clicked() {
                    self.parametric_equations[i].hidden = !self.parametric_equations[i].hidden;
                    changed = true;
                }

                if ui.small_button("X").clicked() {
                    to_delete = Some(i);
                }
            });

            if expanded {
                ui.indent(("parametric_props", i), |ui| {
                    let eq = &mut self.parametric_equations[i];

                    egui::Grid::new(("parametric_grid", i))
                        .num_columns(2)
                        .spacing([8.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("Particles");
                            if ui.add(egui::DragValue::new(&mut eq.num_particles).speed(1).range(1..=10000)).changed() {
                                changed = true;
                            }
                            ui.end_row();

                            ui.label("x =");
                            changed |= draw_parametric_row(ui, &mut eq.x_expr, eq.x_expr_error, "e.g. sin(t)");
                            ui.end_row();

                            ui.label("y =");
                            changed |= draw_parametric_row(ui, &mut eq.y_expr, eq.y_expr_error, "e.g. cos(t)");
                            ui.end_row();

                            ui.label("z =");
                            changed |= draw_parametric_row(ui, &mut eq.z_expr, eq.z_expr_error, "e.g. t");
                            ui.end_row();

                            ui.label("Spread = ");
                            changed |= draw_parametric_row(ui, &mut eq.spread_expr, eq.spread_expr_error, "e.g. 0.1");
                            ui.end_row();

                            ui.label("Use Domain");
                            changed |= ui.checkbox(&mut eq.use_domain, "").changed();
                            ui.end_row();

                            if eq.use_domain {
                                ui.label("Min = ");
                                changed |= draw_parametric_row(ui, &mut eq.domain_lower_expr, eq.domain_lower_expr_error, "0.0");
                                ui.end_row();

                                ui.label("Max = ");
                                changed |= draw_parametric_row(ui, &mut eq.domain_upper_expr, eq.domain_upper_expr_error, "10.0");
                                ui.end_row();

                                ui.label("Direction");
                                let direction_text = match eq.domain_direction {
                                    DomainLoopDirection::Wrap => "Wrap",
                                    DomainLoopDirection::PingPong => "Ping Pong",
                                };
                                egui::ComboBox::from_id_salt(("domain_direction", i))
                                    .selected_text(direction_text)
                                    .show_ui(ui, |ui| {
                                        changed |= ui
                                            .selectable_value(&mut eq.domain_direction, DomainLoopDirection::Wrap, "Wrap")
                                            .changed();
                                        changed |= ui
                                            .selectable_value(&mut eq.domain_direction, DomainLoopDirection::PingPong, "Ping Pong")
                                            .changed();
                                    });
                                ui.end_row();
                            }
                        });

                    if let Some(error) = &eq.error {
                        ui.colored_label(egui::Color32::RED, error);
                    }

                    if eq.use_domain && (eq.domain_lower_expr_error || eq.domain_upper_expr_error) {
                        ui.colored_label(egui::Color32::RED, "Invalid domain bounds");
                    }
                });

                ui.separator();
            }
        }

        if let Some(idx) = to_delete {
            self.parametric_equations.remove(idx);
            if idx < self.expanded_parametric.len() {
                self.expanded_parametric.remove(idx);
            }
            changed = true;
        }

        if changed && particles.use_parametric {
            match self.try_compile_parametric(particles) {
                Ok(()) => state.events.push(Event::Alert("Parametric equations updated".to_string())),
                Err(()) => {}
            }

            if !state.clock_running {
                for parametric in &particles.parametric_equations {
                    parametric.apply_to_particles(&mut particles.particles, particles.time);
                }
            }
        }

        if let Some(error) = &self.parametric_error {
            ui.colored_label(egui::Color32::RED, error);
        }
    }

    fn draw_saves(&mut self, ui: &mut egui::Ui, particles: &mut Particles, state: &mut State) {
        ui.horizontal(|ui| {
            if ui.button("Save Current").clicked() {
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(config_dir) = &self.config_dir {
                    let mut save_name = format!("save_{}.json", chrono::Local::now().format("%Y-%m-%d-%H-%M-%S"));
                    let mut path = config_dir.join(&save_name);
                    let mut count = 1;
                    while path.exists() {
                        save_name = format!("save_{}-{}.json", chrono::Local::now().format("%Y-%m-%d-%H-%M-%S"), count);
                        path = config_dir.join(&save_name);
                        count += 1;
                    }

                    let object = json!({
                        "state": &state,
                        "particles": &particles,
                        "editor": &self,
                    });

                    match serde_json::to_string_pretty(&object) {
                        Ok(json) => {
                            if let Err(err) = std::fs::write(&path, json) {
                                state.events.push(Event::Alert(format!("Failed to write save file: {}", err)));
                                eprintln!("Failed to write save file {:?}: {}", path, err);
                            } else {
                                self.saves_list.push(save_name.trim_end_matches(".json").to_string());
                                state.events.push(Event::Alert("Save successful".to_string()));
                            }
                        }
                        Err(err) => {
                            state.events.push(Event::Alert(format!("Failed to serialize save data: {}", err)));
                            eprintln!("Failed to serialize save data: {}", err);
                        }
                    }
                }
                #[cfg(target_arch="wasm32")]
                {
                    let mut save_name = format!("save_{}.json", quad_timestamp::timestamp_utc().unwrap());
                    let mut count = 1;

                    while !self.saves_list.iter().all(|s| *s != save_name) {
                        save_name = format!("save_{}-{}.json", quad_timestamp::timestamp_utc().unwrap(), count);
                        count += 1;
                    }

                    let object = json!({
                        "state": &state,
                        "particles": &particles,
                        "editor": &self,
                    });

                    match serde_json::to_string(&object) {
                        Ok(json) => {
                            let mut storage = STORAGE.lock().unwrap();
                            storage.set(&save_name, &json);

                            // check if it was successfully saved
                            if storage.get(&save_name).is_none() {
                                state.events.push(Event::Alert(format!("Failed to save: {}", save_name)));
                            } else {
                                self.saves_list.push(save_name);
                                storage.set("saves_list", &serde_json::to_string(&self.saves_list).unwrap_or_else(|_| "[]".to_string()));
                                state.events.push(Event::Alert("Save successful".to_string()));
                            }
                        }
                        Err(err) => {
                            state.events.push(Event::Alert(format!("Failed to serialize save data: {}", err)));
                            eprintln!("Failed to serialize save data: {}", err);
                        }
                    }
                }
            }
        });

        ui.separator();

        let mut selected_save: Option<Save> = None;
        let mut delete_save: Option<String> = None;

        if self.saves_list.is_empty() {
            ui.label("No saves found");
        } else {
            for save in &self.saves_list {
                ui.horizontal(|ui| {
                    ui.label(save);
                    if ui.button("Load").clicked() {
                        #[cfg(not(target_arch = "wasm32"))]
                        if let Some(config_dir) = &self.config_dir {
                            let path = config_dir.join(format!("{}.json", save));
                            match std::fs::read_to_string(&path) {
                                Ok(contents) => match serde_json::from_str::<Save>(&contents) {
                                    Ok(loaded_save) => {
                                        selected_save = Some(loaded_save);
                                        state.events.push(Event::Alert(format!("Loaded save: {}", save)));
                                    }
                                    Err(err) => {
                                        state.events.push(Event::Alert(format!("Failed to parse save file: {}", err)));
                                        eprintln!("Failed to parse save file {:?}: {}", path, err);
                                    }
                                },
                                Err(err) => {
                                    state.events.push(Event::Alert(format!("Failed to read save file: {}", err)));
                                    eprintln!("Failed to read save file {:?}: {}", path, err);
                                }
                            }
                        }
                        #[cfg(target_arch = "wasm32")]
                        {
                            match STORAGE.lock().unwrap().get(save) {
                                Some(contents) => match serde_json::from_str::<Save>(&contents) {
                                    Ok(loaded_save) => {
                                        selected_save = Some(loaded_save);
                                        state.events.push(Event::Alert(format!("Loaded save: {}", save)));
                                    }
                                    Err(err) => {
                                        state.events.push(Event::Alert(format!("Failed to parse save data: {}", err)));
                                        eprintln!("Failed to parse save data for {}: {}", save, err);
                                    }
                                },
                                None => {
                                    state.events.push(Event::Alert("Save data not found".to_string()));
                                    eprintln!("Save data not found for {}", save);
                                }
                            }
                        }
                    }

                    #[cfg(not(target_arch = "wasm32"))]
                    if ui.button("📄").clicked() {
                        if let Some(config_dir) = &self.config_dir {
                            let path = config_dir.join(format!("{}.json", save));
                            if let Err(err) = open::that(&path) {
                                state.events.push(Event::Alert(format!("Failed to open save file: {}", err)));
                                eprintln!("Failed to open save file {:?}: {}", path, err);
                            }
                        }
                    }

                    if ui.button("X").clicked() {
                        #[cfg(not(target_arch = "wasm32"))]
                        if let Some(config_dir) = &self.config_dir {
                            let path = config_dir.join(format!("{}.json", save));
                            if let Err(err) = std::fs::remove_file(&path) {
                                state.events.push(Event::Alert(format!("Failed to delete save file: {}", err)));
                                eprintln!("Failed to delete save file {:?}: {}", path, err);
                            } else {
                                state.events.push(Event::Alert(format!("Deleted save: {}", save)));
                                delete_save = Some(save.clone());
                            }
                        }
                        #[cfg(target_arch = "wasm32")]
                        {
                            let mut storage = STORAGE.lock().unwrap();
                            storage.remove(save);
                            state.events.push(Event::Alert(format!("Deleted save: {}", save)));
                            delete_save = Some(save.clone());
                        }
                    }
                });
            }
        }

        if let Some(loaded_save) = selected_save {
            state.apply_state_save(loaded_save.state);
            *particles = loaded_save.particles;
            self.apply_editor_save(loaded_save.editor);
            self.try_compile_parametric(particles).ok();
        }

        if let Some(save_to_delete) = delete_save {
            self.saves_list.retain(|s| s != &save_to_delete);

            #[cfg(target_arch = "wasm32")]
            STORAGE.lock().unwrap().set("saves_list", &serde_json::to_string(&self.saves_list).unwrap_or_else(|_| "[]".to_string()));
        }
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
        ui.label("Toggle simulation/parametric mode: m");
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
        ui.separator();
        ui.add_space(10.0);
        ui.label("Parametric equations:");
        ui.label("Particles: the number of particles assigned to each parametric equation");
        ui.label("X, Y, Z: the parametric equations for the particle positions. These equations are functions of time t and the particle's current position (x, y, z).");
        ui.label("Spread: the time offset between particles in the same equation");
        ui.label("Use Domain: enables/disables domain looping.");
        ui.label("Domain [min, max]: the range of the parameter t over which the equation is evaluated.");
        ui.label("Direction: Wrap jumps from max back to min. Ping Pong goes back and forth between min and max.");
        ui.label("Equation variables: t (time), i (particle index), x, y, z (current particle position)");
        ui.add_space(10.0);
    }

    pub fn try_compile_parametric(&mut self, particles: &mut Particles) -> Result<(), ()> {
        // First pass: validate num_particles and check total doesn't exceed available
        let total_particles_requested: usize = self.parametric_equations.iter().map(|eq| eq.num_particles).sum();
        
        if total_particles_requested > particles.particles.len() {
            self.parametric_error = Some(format!("Total particles requested ({}) exceeds available ({})", total_particles_requested, particles.particles.len()));
            return Err(());
        }

        // Unhide all particles first so that removed/resized equations don't leave particles stuck hidden.
        // Only do this in parametric mode — in simulation mode the user controls hidden state manually.
        if particles.use_parametric {
            for p in &mut particles.particles {
                p.hidden = false;
            }
        }

        let mut compiled: Vec<ParametricEquations> = Vec::with_capacity(self.parametric_equations.len());
        let mut particle_offset = 0;
        let mut has_errors = false;

        for eq in &mut self.parametric_equations {
            let x_fn_result = compile_parametric_fn(&eq.x_expr);
            eq.x_expr_error = x_fn_result.is_err();

            let y_fn_result = compile_parametric_fn(&eq.y_expr);
            eq.y_expr_error = y_fn_result.is_err();

            let z_fn_result = compile_parametric_fn(&eq.z_expr);
            eq.z_expr_error = z_fn_result.is_err();

            let (spread, spread_parse_failed) = match eval_constant_expr(&eq.spread_expr) {
                Ok(value) => (value, false),
                Err(_) => (0.0, true),
            };
            eq.spread_expr_error = spread_parse_failed || spread.is_nan() || spread.is_infinite() || spread < 0.0;

            let mut invalid_domain_bounds = false;
            let domain = if eq.use_domain {
                let (domain_lower, domain_lower_parse_failed) = match eval_constant_expr(&eq.domain_lower_expr) {
                    Ok(value) => (value, false),
                    Err(_) => (0.0, true),
                };

                let (domain_upper, domain_upper_parse_failed) = match eval_constant_expr(&eq.domain_upper_expr) {
                    Ok(value) => (value, false),
                    Err(_) => (0.0, true),
                };

                eq.domain_lower_expr_error = domain_lower_parse_failed || domain_lower.is_nan() || domain_lower.is_infinite();
                eq.domain_upper_expr_error = domain_upper_parse_failed || domain_upper.is_nan() || domain_upper.is_infinite();
                invalid_domain_bounds = eq.domain_lower_expr_error
                    || eq.domain_upper_expr_error
                    || domain_upper <= domain_lower;

                Some((domain_lower, domain_upper))
            } else {
                eq.domain_lower_expr_error = false;
                eq.domain_upper_expr_error = false;
                None
            };


            if eq.x_expr_error {
                eq.error = Some(format!("x: {}", x_fn_result.err().unwrap()));
                has_errors = true;
                continue;
            }
            if eq.y_expr_error {
                eq.error = Some(format!("y: {}", y_fn_result.err().unwrap()));
                has_errors = true;
                continue;
            }
            if eq.z_expr_error {
                eq.error = Some(format!("z: {}", z_fn_result.err().unwrap()));
                has_errors = true;
                continue;
            }
            if eq.spread_expr_error {
                eq.error = Some("Spread must be a non-negative number".to_string());
                has_errors = true;
                continue;
            }
            if invalid_domain_bounds {
                eq.error = Some("Domain must be finite and satisfy max > min".to_string());
                has_errors = true;
                continue;
            }

            // Allocate particle indices sequentially
            let particle_indices: Vec<usize> = (particle_offset..particle_offset + eq.num_particles).collect();
            particle_offset += eq.num_particles;

            // Apply hidden state to the assigned particles
            if eq.hidden {
                for &idx in &particle_indices {
                    if idx < particles.particles.len() {
                        particles.particles[idx].hidden = true;
                    }
                }
            }

            eq.error = None;
            compiled.push(ParametricEquations {
                x_fn: x_fn_result.unwrap(),
                y_fn: y_fn_result.unwrap(),
                z_fn: z_fn_result.unwrap(),
                spread,
                particle_indices,
                running: eq.running,
                domain,
                domain_direction: eq.domain_direction,
            });
        }

        if has_errors {
            self.parametric_error = Some("Fix equation errors above".to_string());
            return Err(());
        }

        self.parametric_error = None;
        particles.parametric_equations = compiled;

        // Hide particles not claimed by any equation, but only when actively in parametric mode
        if particles.use_parametric {
            for i in particle_offset..particles.particles.len() {
                particles.particles[i].hidden = true;
            }
        }

        Ok(())
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

fn eval_constant_expr(expr: &str) -> Result<f64, String> {
    let expr = insert_implicit_mul(expr);
    meval::eval_str(expr).map_err(|err| err.to_string())
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