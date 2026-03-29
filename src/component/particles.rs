use std::collections::VecDeque;

use meval::Expr;
use macroquad::prelude::*;
use crate::force::{resolve_collisions, n_body_update};
use crate::component::{Component, Event};
use crate::State;

type ParametricFn = Box<dyn Fn(f64, f64, f64, f64) -> f64>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainLoopDirection {
    Wrap,
    PingPong,
}

pub struct ParametricEquations {
    pub x_fn: ParametricFn,
    pub y_fn: ParametricFn,
    pub z_fn: ParametricFn,
    pub spread: f64,
    pub domain: Option<(f64, f64)>,
    pub domain_direction: DomainLoopDirection,
    pub particle_indices: Vec<usize>,
    pub running: bool,
}

impl ParametricEquations {
    pub fn apply_to_particles(&self, particles: &mut [Particle], t: f32) {
        if !self.running {
            return;
        }
        for (i, &idx) in self.particle_indices.iter().enumerate() {
            if idx < particles.len() {
                let p = &mut particles[idx];
                let raw_t = t as f64 + i as f64 * self.spread;
                let t = match self.domain {
                    None => raw_t,
                    Some((domain_min, domain_max)) => {
                        let domain_size = domain_max - domain_min;
                        match self.domain_direction {
                            DomainLoopDirection::Wrap => {
                                (raw_t - domain_min).rem_euclid(domain_size) + domain_min
                            }
                            DomainLoopDirection::PingPong => {
                                let cycle = domain_size * 2.0;
                                let phase = (raw_t - domain_min).rem_euclid(cycle);
                                if phase <= domain_size {
                                    domain_min + phase
                                } else {
                                    domain_max - (phase - domain_size)
                                }
                            }
                        }
                    }
                };
                let x = p.pos.x as f64;
                let y = p.pos.y as f64;
                let z = p.pos.z as f64;

                p.pos.x = (self.x_fn)(t, x, y, z) as f32;
                p.pos.y = (self.y_fn)(t, x, y, z) as f32;
                p.pos.z = (self.z_fn)(t, x, y, z) as f32;
            }
        }
    }
}

pub fn compile_parametric_fn(src: &str) -> Result<ParametricFn, String> {
    let expr = src.parse::<Expr>().map_err(|e| e.to_string())?;
    let func = expr.bind4("t", "x", "y", "z").map_err(|e| e.to_string())?;
    Ok(Box::new(move |t: f64, x: f64, y: f64, z: f64| func(t, x, y, z)))
}

#[derive(Debug, Clone)]
pub struct Particle {
    pub pos: Vec3,
    pub vel: Vec3,
    pub acc: Vec3,
    pub mass: f32,
    pub friction: f32,
    pub radius: f32,
    pub color: Color,
    pub trail: VecDeque<Vec3>,
    pub hidden: bool,
}

impl Particle {
    pub fn verlet_drift(&mut self, dt: f32) {
        self.pos += self.vel * dt + self.acc * (0.5 * dt * dt);
    }

    pub fn verlet_kick(&mut self, old_acc: Vec3, dt: f32) {
        self.vel += (old_acc + self.acc) * (0.5 * dt);
    }

    pub fn update_trail(&mut self) {
        // Update trail
        self.trail.push_back(self.pos);
        if self.trail.len() > 50 {
            self.trail.pop_front();
        }
    }
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            pos: Vec3::ZERO,
            vel: Vec3::ZERO,
            acc: Vec3::ZERO,
            mass: 0.0,
            friction: 0.0,
            radius: 0.1,
            color: RED,
            trail: VecDeque::new(),
            hidden: false,
        }
    }
}

pub struct Particles {
    pub show_trail: bool,
    pub use_cubes: bool,
    pub min_merge_mass: f32,
    pub restitution: f32,
    pub g: f32,
    pub use_parametric: bool,
    pub time: f32,
    pub parametric_equations: Vec<ParametricEquations>,
    pub particles: Vec<Particle>,
}

impl Particles {
    pub fn update_particles_verlet(
        &mut self,
        dt: f32
    ) {
        let old_acc: Vec<Vec3> = self.particles.iter().map(|p| p.acc).collect();

        for p in self.particles.iter_mut() {
            if !p.hidden {
                p.verlet_drift(dt);
            }
        }

        resolve_collisions(&mut self.particles, self.restitution, self.min_merge_mass, self.g);
        n_body_update(&mut self.particles, self.g);

        for (p, &prev_acc) in self.particles.iter_mut().zip(old_acc.iter()) {
            if !p.hidden {
                p.verlet_kick(prev_acc, dt);
            }
        }
    }
}

impl Component for Particles {
    fn draw(&self, _state: &State) {
        for p in &self.particles {
            if p.hidden {
                continue;
            }
            if self.use_cubes {
                draw_cube(p.pos, vec3(p.radius * 2.0, p.radius * 2.0, p.radius * 2.0), None, p.color);
            } else {
                draw_sphere_ex(p.pos, p.radius, None, p.color, sphere_lod_params(p.radius));
            }
        }

        if self.show_trail {
            for p in &self.particles {
                if p.hidden {
                    continue;
                }
                for i in 1..p.trail.len() {
                    let a = p.trail[i - 1];
                    let b = p.trail[i];

                    let alpha = i as f32 / p.trail.len() as f32;

                    draw_line_3d(
                        a,
                        b,
                        Color::new(p.color.r, p.color.g, p.color.b, alpha)
                    );
                }
            }
        }
    }

    fn handle_input(&mut self, state: &mut State) {
        if state.ui_captures_keyboard {
            return;
        }

        let mut alert_msg: String = String::new();
        if is_key_pressed(KeyCode::T) {
            self.show_trail = !self.show_trail;
            alert_msg = format!("Trails: {}", if self.show_trail { "On" } else { "Off" });
        }
        if is_key_pressed(KeyCode::C) {
            self.use_cubes = !self.use_cubes;
            alert_msg = format!("Render Mode: {}", if self.use_cubes { "Cubes" } else { "Spheres" });
        }
        if is_key_pressed(KeyCode::Slash) {
            self.use_parametric = !self.use_parametric;
            // Always unhide everything first
            for p in &mut self.particles {
                p.hidden = false;
            }
            if self.use_parametric {
                // Hide particles not claimed by any equation
                let used: usize = self.parametric_equations.iter().map(|eq| eq.particle_indices.len()).sum();
                for i in used..self.particles.len() {
                    self.particles[i].hidden = true;
                }
            }
            alert_msg = format!("Parametric Mode: {}", if self.use_parametric { "On" } else { "Off" });
        }

        if !alert_msg.is_empty() {
            state.events.push(Event::Alert(alert_msg));
        }
    }

    fn update(&mut self, dt: f32, state: &mut State) {
        if !state.clock_running { return; }
        
        self.time += dt * state.time_warp;

        let sim_dt = dt * state.time_warp;

        if self.use_parametric {
            for parametric in &self.parametric_equations {
                parametric.apply_to_particles(&mut self.particles, self.time);
            }
        } else {
            self.update_particles_verlet(
                sim_dt
            );
        }

        for p in self.particles.iter_mut() {
            if !p.hidden {
                p.update_trail();
            }
        }
    }
}

fn sphere_lod_params(radius: f32) -> DrawSphereParams {
    let clamped_radius = radius.max(0.1);
    let slices = (12.0 + clamped_radius * 0.8).round() as usize;
    let slices = slices.clamp(12, 96);
    let rings = (slices / 2).clamp(8, 64);

    DrawSphereParams {
        rings,
        slices,
        ..Default::default()
    }
}