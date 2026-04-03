use std::collections::VecDeque;
use serde::{Serialize, Deserialize};

use meval::Expr;
use macroquad::prelude::*;
use crate::force::{resolve_collisions, n_body_update};
use crate::component::{Component, Event};
use crate::State;
use crate::serde_helper::serde_color;

type ParametricFn = Box<dyn Fn(f64, f64, f64, f64, f64) -> f64>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

                let new_x = (self.x_fn)(t, i as f64, x, y, z) as f32;
                let new_y = (self.y_fn)(t, i as f64, x, y, z) as f32;
                let new_z = (self.z_fn)(t, i as f64, x, y, z) as f32;

                p.pos.x = new_x;
                p.pos.y = new_y;
                p.pos.z = new_z;
            }
        }
    }
}

pub fn insert_implicit_mul(expr: &str) -> String {
    let chars: Vec<char> = expr.chars().filter(|c| !c.is_whitespace()).collect();
    let mut out = String::with_capacity(chars.len() * 2);

    for pair in chars.windows(2) {
        let (cur, next) = (pair[0], pair[1]);
        out.push(cur);

        let needs_mul = match (cur, next) {
            // 5t, 5(, t5, t(, )5, )t, )(
            ('0'..='9', 'a'..='z' | 'A'..='Z' | '(') => true,
            ('a'..='z' | 'A'..='Z' | ')', '0'..='9') => true,
            (')', 'a'..='z' | 'A'..='Z' | '(') => true,
            _ => false,
        };

        if needs_mul {
            out.push('*');
        }
    }

    if let Some(&last) = chars.last() {
        out.push(last);
    }

    out
}

pub fn compile_parametric_fn(src: &str) -> Result<ParametricFn, String> {
    let src = insert_implicit_mul(src);

    let expr = src.parse::<Expr>().map_err(|e| e.to_string())?;
    let func = expr.bind5("t", "i", "x", "y", "z").map_err(|e| e.to_string())?;
    Ok(Box::new(move |t: f64, i: f64, x: f64, y: f64, z: f64| func(t, i, x, y, z)))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Particle {
    pub pos: Vec3,
    pub vel: Vec3,
    pub acc: Vec3,
    pub mass: f32,
    pub friction: f32,
    pub restitution: f32,
    pub radius: f32,
    #[serde(with = "serde_color")]
    pub color: Color,
    #[serde(skip_serializing, skip_deserializing, default)]
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
            restitution: 0.5,
            radius: 0.1,
            color: RED,
            trail: VecDeque::new(),
            hidden: false,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Particles {
    pub show_trail: bool,
    pub use_cubes: bool,
    pub min_merge_mass: f32,
    pub g: f32,
    pub use_parametric: bool,
    pub time: f32,
    #[serde(skip_serializing, skip_deserializing, default)]
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

        resolve_collisions(&mut self.particles, self.min_merge_mass, self.g);
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
        if self.use_cubes {
            for p in &self.particles {
                if p.hidden {
                    continue;
                }
                draw_cube(p.pos, vec3(p.radius * 2.0, p.radius * 2.0, p.radius * 2.0), None, p.color);
            }
        } else {
            let (base_sphere_vertices, base_sphere_indices) = match self.particles.len() {
                0..=1000 => create_base_sphere(1.0, 16, 16),
                1001..=5000 => create_base_sphere(1.0, 12, 12),
                _ => create_base_sphere(1.0, 8, 8),
            };

            let verts_per_sphere = base_sphere_vertices.len() as u16;

            const MAX_SPHERES_PER_BATCH: usize = 25;
            for batch in self.particles.chunks(MAX_SPHERES_PER_BATCH) {
                let mut vertices = Vec::with_capacity(batch.len() * base_sphere_vertices.len());
                let mut indices = Vec::with_capacity(batch.len() * base_sphere_indices.len());

                for (i, sphere) in batch.iter().enumerate() {
                    if sphere.hidden {
                        continue;
                    }

                    for v in &base_sphere_vertices {
                        let mut new_v = *v;
                        new_v.position = new_v.position * sphere.radius + sphere.pos;
                        new_v.color = sphere.color.into();
                        vertices.push(new_v);
                    }

                    for idx in &base_sphere_indices {
                        indices.push(idx + i as u16 * verts_per_sphere);
                    }
                }

                let mesh = Mesh {
                    vertices,
                    indices,
                    texture: None,
                };

                draw_mesh(&mesh);
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
        if is_key_pressed(KeyCode::M) {
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

                for parametric in &self.parametric_equations {
                    parametric.apply_to_particles(&mut self.particles, self.time);
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

fn create_base_sphere(radius: f32, rings: u32, slices: u32) -> (Vec<Vertex>, Vec<u16>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Generate vertices
    for ring in 0..=rings {
        let v = ring as f32 / rings as f32;
        let theta = v * std::f32::consts::PI;

        for slice in 0..=slices {
            let u = slice as f32 / slices as f32;
            let phi = u * std::f32::consts::TAU;

            let x = phi.cos() * theta.sin();
            let y = theta.cos();
            let z = phi.sin() * theta.sin();

            vertices.push(Vertex {
                position: vec3(x * radius, y * radius, z * radius),
                uv: vec2(u, v),
                color: WHITE.into(),
                normal: Vec4::ZERO
            });
        }
    }

    // Generate indices
    let verts_per_row = slices + 1;

    for ring in 0..rings {
        for slice in 0..slices {
            let i0 = ring * verts_per_row + slice;
            let i1 = i0 + 1;
            let i2 = i0 + verts_per_row;
            let i3 = i2 + 1;

            // Two triangles per quad
            indices.push(i0 as u16);
            indices.push(i2 as u16);
            indices.push(i1 as u16);

            indices.push(i1 as u16);
            indices.push(i2 as u16);
            indices.push(i3 as u16);
        }
    }

    (vertices, indices)
}