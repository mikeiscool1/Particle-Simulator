use std::collections::VecDeque;
use serde::{Serialize, Deserialize};

use meval::Expr;
use macroquad::{Error, prelude::*};
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
            p.verlet_drift(dt);
        }

        resolve_collisions(&mut self.particles, self.min_merge_mass, self.g);
        n_body_update(&mut self.particles, self.g);

        for (p, &prev_acc) in self.particles.iter_mut().zip(old_acc.iter()) {
            p.verlet_kick(prev_acc, dt);
        }
    }
}

// shaders
const SPHERE_FRAGMENT: &str = r#"#version 300 es
precision highp float;

in vec3 frag_pos;
in vec4 vert_color;
in vec3 v_sphere_center;
in float v_sphere_radius;

out vec4 fragColor;

uniform vec3 camera_pos;
uniform mat4 ViewProj;

void main() {
    vec3 ray_dir = normalize(frag_pos - camera_pos);

    vec3 oc = camera_pos - v_sphere_center;
    float a = dot(ray_dir, ray_dir);
    float b = 2.0 * dot(oc, ray_dir);
    float c = dot(oc, oc) - v_sphere_radius * v_sphere_radius;
    float discriminant = b * b - 4.0 * a * c;

    if (discriminant < 0.0) discard;

    float t = (-b - sqrt(discriminant)) / (2.0 * a);
    if (t < 0.0) t = (-b + sqrt(discriminant)) / (2.0 * a);
    if (t < 0.0) discard;

    vec3 hit = camera_pos + t * ray_dir;

    vec4 clip = ViewProj * vec4(hit, 1.0);
    gl_FragDepth = (clip.z / clip.w) * 0.5 + 0.5;

    vec3 normal = normalize(hit - v_sphere_center);
    float diffuse = max(dot(normal, normalize(vec3(1.0, 1.0, 1.0))), 0.0);
    float light = 0.6 + 0.4 * diffuse;

    fragColor = vec4(vert_color.rgb * light, vert_color.a);
}
"#;

const SPHERE_VERTEX: &str = r#"#version 300 es
in vec3 position;
in vec4 color0;
in vec4 normal;

out vec3 frag_pos;
out vec4 vert_color;
out vec3 v_sphere_center;
out float v_sphere_radius;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    vec4 world_pos = Model * vec4(position, 1.0);
    frag_pos = world_pos.xyz;
    vert_color = color0 / 255.0;
    v_sphere_center = normal.xyz;
    v_sphere_radius = normal.w;
    gl_Position = Projection * world_pos;
}
"#;

pub fn create_sphere_material() -> Result<Material, Error> {
    let a = load_material(
        ShaderSource::Glsl {
            vertex: SPHERE_VERTEX,
            fragment: SPHERE_FRAGMENT,
        },
        MaterialParams {
            uniforms: vec![
                UniformDesc::new("camera_pos", UniformType::Float3),
                UniformDesc::new("ViewProj",   UniformType::Mat4),
            ],
            pipeline_params: PipelineParams {
                depth_write: true,
                depth_test: Comparison::LessOrEqual,
                ..Default::default()
            },
            ..Default::default()
        },
    )?;

    Ok(a)
}

pub static SPHERE_MATERIAL: std::sync::LazyLock<Material> = std::sync::LazyLock::new(|| create_sphere_material().unwrap());

pub fn draw_spheres_batched(
    particles: &[&Particle],
    camera_pos: Vec3,
    vp: Mat4,
) {
    let mut vertices = Vec::with_capacity(particles.len() * 4);
    let mut indices = Vec::with_capacity(particles.len() * 6);

    let to_y = Vec3::Y;

    for (i, p) in particles.iter().enumerate() {
        let to_cam = (camera_pos - p.pos).normalize();
        let right = if to_cam.dot(to_y).abs() < 0.99 {
            to_cam.cross(to_y).normalize()
        } else {
            to_cam.cross(Vec3::Z).normalize()
        };
        let up = to_cam.cross(right).normalize();

        let dist = (camera_pos - p.pos).length();

        if dist < p.radius {
            continue;
        }

        let scale = if dist > p.radius {
            // analytical minimum quad size to always contain the silhouette
            let sin_alpha = p.radius / dist;
            let cos_alpha = (1.0 - sin_alpha * sin_alpha).sqrt();
            1.0 / cos_alpha
        } else {
            1.0 / 10000.0 // inside the sphere, just make it big
        };

        let r = p.radius * scale;
        
        let positions = [
            p.pos + (-right - up) * r,
            p.pos + ( right - up) * r,
            p.pos + ( right + up) * r,
            p.pos + (-right + up) * r,
        ];

        // pack sphere_center into normal.xyz, radius into normal.w
        let packed_normal = Vec4::new(p.pos.x, p.pos.y, p.pos.z, p.radius);

        for pos in positions {
            vertices.push(Vertex {
                position: pos,
                uv: Vec2::ZERO,
                color: p.color.into(),
                normal: packed_normal,
            });
        }

        let base = (i * 4) as u16;
        indices.extend_from_slice(&[
            base, base+1, base+2,
            base, base+2, base+3,
        ]);
    }

    let material = &SPHERE_MATERIAL;
    material.set_uniform("camera_pos", camera_pos);
    material.set_uniform("ViewProj",   vp);

    gl_use_material(material);
    draw_mesh(&Mesh { vertices, indices, texture: None });
    gl_use_default_material();
}

impl Component for Particles {
    fn draw(&self, state: &State) {
        let view = Mat4::look_at_rh(state.camera.position, state.camera.target, state.camera.up);
        let proj = Mat4::perspective_rh_gl(
            state.camera.fovy,
            screen_width() / screen_height(),
            0.01,
            1000.0,
        );
        let vp = proj * view;

        if self.show_trail {
            set_default_camera();
            let sw = screen_width();
            let sh = screen_height();
            let line_width = 1.0;

            let mut vertices: Vec<Vertex> = Vec::new();
            let mut indices: Vec<u16> = Vec::new();

            for p in &self.particles {
                if p.hidden || p.trail.len() < 2 { continue; }

                let mut last_drawn = 0;
                for i in 1..p.trail.len() {
                    let a = p.trail[last_drawn];
                    let b = p.trail[i];

                    let clip_a = vp * vec4(a.x, a.y, a.z, 1.0);
                    let clip_b = vp * vec4(b.x, b.y, b.z, 1.0);
                    if clip_a.w <= 0.0 || clip_b.w <= 0.0 { continue; }

                    let ndc_a = clip_a.xyz() / clip_a.w;
                    let ndc_b = clip_b.xyz() / clip_b.w;

                    if ndc_a.x < -1.0 && ndc_b.x < -1.0 { continue; }
                    if ndc_a.x > 1.0 && ndc_b.x > 1.0 { continue; }
                    if ndc_a.y < -1.0 && ndc_b.y < -1.0 { continue; }
                    if ndc_a.y > 1.0 && ndc_b.y > 1.0 { continue; }

                    let sa = vec2((ndc_a.x + 1.0) * 0.5 * sw, (1.0 - ndc_a.y) * 0.5 * sh);
                    let sb = vec2((ndc_b.x + 1.0) * 0.5 * sw, (1.0 - ndc_b.y) * 0.5 * sh);

                    let dx = sb.x - sa.x;
                    let dy = sb.y - sa.y;
                    let len = (dx * dx + dy * dy).sqrt();
                    if len < 2.0 { continue; }

                    // perpendicular for line thickness
                    let nx = -dy / len * line_width * 0.5;
                    let ny =  dx / len * line_width * 0.5;

                    let alpha = i as f32 / p.trail.len() as f32;
                    let color: [u8; 4] = Color::new(p.color.r, p.color.g, p.color.b, alpha).into();

                    let base = vertices.len() as u16;
                    vertices.extend_from_slice(&[
                        Vertex { position: vec3(sa.x + nx, sa.y + ny, 0.0), uv: Vec2::ZERO, color, normal: Vec4::ZERO },
                        Vertex { position: vec3(sa.x - nx, sa.y - ny, 0.0), uv: Vec2::ZERO, color, normal: Vec4::ZERO },
                        Vertex { position: vec3(sb.x + nx, sb.y + ny, 0.0), uv: Vec2::ZERO, color, normal: Vec4::ZERO },
                        Vertex { position: vec3(sb.x - nx, sb.y - ny, 0.0), uv: Vec2::ZERO, color, normal: Vec4::ZERO },
                    ]);
                    indices.extend_from_slice(&[base, base+1, base+2, base+1, base+3, base+2]);

                    last_drawn = i;

                    // flush if approaching u16 limit
                    if indices.len() > 60000 {
                        draw_mesh(&Mesh { vertices: std::mem::take(&mut vertices), indices: std::mem::take(&mut indices), texture: None });
                    }
                }
            }

            if !vertices.is_empty() {
                draw_mesh(&Mesh { vertices, indices, texture: None });
            }

            set_camera(&state.camera);
        }

        if !self.use_cubes {
            let visible: Vec<&Particle> = self.particles.iter()
                .filter(|p| !p.hidden)
                .collect();
            draw_spheres_batched(&visible, state.camera.position, vp);

            // after draw_spheres_batched
            let closest_inside = visible.iter()
                .filter(|p| (state.camera.position - p.pos).length() <= p.radius)
                .min_by(|a, b| {
                    let da = (state.camera.position - a.pos).length();
                    let db = (state.camera.position - b.pos).length();
                    da.partial_cmp(&db).unwrap()
                });

            if let Some(p) = closest_inside {
                set_default_camera();
                draw_rectangle(0.0, 0.0, screen_width(), screen_height(),
                    Color::new(p.color.r, p.color.g, p.color.b, 0.8));
                set_camera(&state.camera);
            }
        } else {
            for p in &self.particles {
                if p.hidden { continue; }
                draw_cube(p.pos, vec3(p.radius * 2.0, p.radius * 2.0, p.radius * 2.0), None, p.color);
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