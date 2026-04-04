use macroquad::prelude::*;
use crate::component::Particle;
use std::collections::VecDeque;

pub fn resolve_collisions(particles: &mut Vec<Particle>, min_merge_mass: f32, g: f32) {
    let mut i = 0;
    while i < particles.len() {
        if particles[i].mass <= 0.0 {
            i += 1;
            continue; // skip massless particles
        }

        let mut j = i + 1;

        while j < particles.len() {
            if particles[j].mass <= 0.0 {
                j += 1;
                continue; // skip massless particles
            }

            let pi = particles[i].pos;
            let pj = particles[j].pos;
            let delta = pj - pi;
            let distance = delta.length();
            let radius_sum = particles[i].radius + particles[j].radius;

            if distance <= radius_sum {
                let normal = if distance > 1e-6 {
                    delta / distance
                } else {
                    vec3(1.0, 0.0, 0.0)
                };

                let overlap = (radius_sum - distance).max(0.0);
                if overlap > 0.0 {
                    let total_mass = particles[i].mass + particles[j].mass;
                    let move_i = overlap * (particles[j].mass / total_mass);
                    let move_j = overlap * (particles[i].mass / total_mass);

                    particles[i].pos -= normal * move_i;
                    particles[j].pos += normal * move_j;
                }

                let relative_velocity = particles[j].vel - particles[i].vel;
                let relative_speed_normal = relative_velocity.dot(normal);

                let total_mass = particles[i].mass + particles[j].mass;
                let escape_velocity = (2.0 * g * total_mass / distance.max(1e-6)).sqrt();
                let approach_speed = (-relative_speed_normal).max(0.0);

                let should_merge = min_merge_mass != -1.0 && particles[i].mass >= min_merge_mass && particles[j].mass >= min_merge_mass && approach_speed < escape_velocity;

                if should_merge {
                    merge_particles(particles, i, j);
                    continue;
                }

                if relative_speed_normal < 0.0 {
                    let m1 = particles[i].mass;
                    let m2 = particles[j].mass;
                    let inv_mass_sum = 1.0 / m1 + 1.0 / m2;
                    let e = (particles[i].restitution + particles[j].restitution) / 2.0;
                    let normal_impulse = -(1.0 + e) * relative_speed_normal / inv_mass_sum;

                    particles[i].vel -= normal * (normal_impulse / m1);
                    particles[j].vel += normal * (normal_impulse / m2);

                    let post_normal_relative_velocity = particles[j].vel - particles[i].vel;
                    let tangential_velocity = post_normal_relative_velocity
                        - normal * post_normal_relative_velocity.dot(normal);
                    let tangential_speed = tangential_velocity.length();

                    if tangential_speed > 1e-6 {
                        let tangent = tangential_velocity / tangential_speed;
                        let desired_friction_impulse = -tangential_speed / inv_mass_sum; // use tangential_speed directly
                        let friction_coefficient = (particles[i].friction * particles[j].friction).sqrt();
                        let max_friction_impulse = friction_coefficient * normal_impulse.abs();
                        let friction_impulse = desired_friction_impulse
                            .clamp(-max_friction_impulse, max_friction_impulse);

                        particles[i].vel -= tangent * (friction_impulse / m1);
                        particles[j].vel += tangent * (friction_impulse / m2);
                    }
                }
            }

            j += 1;
        }

        i += 1;
    }
}

fn merge_particles(particles: &mut Vec<Particle>, i: usize, j: usize) {
    let p1 = &particles[i];
    let p2 = &particles[j];

    let total_mass = p1.mass + p2.mass;
    let pos = (p1.pos * p1.mass + p2.pos * p2.mass) / total_mass;
    let vel = (p1.vel * p1.mass + p2.vel * p2.mass) / total_mass;
    let friction = (p1.friction * p1.mass + p2.friction * p2.mass) / total_mass;
    let restitution = (p1.restitution * p1.mass + p2.restitution * p2.mass) / total_mass;
    let radius = (p1.radius.powi(3) + p2.radius.powi(3)).cbrt();

    let r_total = (p1.radius + p2.radius).max(1e-6);
    let w1 = p1.radius / r_total;
    let w2 = p2.radius / r_total;
    let color = Color::new(
        p1.color.r * w1 + p2.color.r * w2,
        p1.color.g * w1 + p2.color.g * w2,
        p1.color.b * w1 + p2.color.b * w2,
        p1.color.a * w1 + p2.color.a * w2,
    );

    particles[i] = Particle {
        pos,
        vel,
        acc: Vec3::ZERO,
        mass: total_mass,
        friction,
        radius,
        color,
        trail: VecDeque::new(),
        hidden: false,
        restitution,
    };

    particles.swap_remove(j);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn n_body_update(particles: &mut [Particle], g: f32) {
    use rayon::prelude::*;
    
    let n = particles.len();

    // Snapshot positions/masses to avoid borrow conflicts across threads
    let snapshot: Vec<(Vec3, f32, f32)> = particles
        .iter()
        .map(|p| (p.pos, p.mass, p.radius))
        .collect();

    particles
        .par_iter_mut()
        .enumerate()
        .for_each(|(i, particle)| {
            if particle.mass <= 0.0 {
                return;
            }

            let mut force = vec3(0.0, 0.0, 0.0);
            for j in 0..n {
                if i != j {
                    let (pos_j, mass_j, radius_j) = snapshot[j];
                    if mass_j <= 0.0 {
                        continue;
                    }

                    let dir = pos_j - particle.pos;
                    let min_dist = particle.radius + radius_j;
                    let dist_sqr = dir.length_squared().max(min_dist * min_dist);
                    let f = g * particle.mass * mass_j / dist_sqr;
                    force += dir.normalize() * f;
                }
            }

            particle.acc = force / particle.mass;
        });
}

#[cfg(target_arch = "wasm32")]
pub fn n_body_update(particles: &mut [Particle], g: f32) {
    let n = particles.len();

    let snapshot: Vec<(Vec3, f32, f32)> = particles
        .iter()
        .map(|p| (p.pos, p.mass, p.radius))
        .collect();

    let mut forces = vec![Vec3::ZERO; n];

    for i in 0..n {
        let (pos_i, mass_i, radius_i) = snapshot[i];
        if mass_i <= 0.0 { continue; }

        for j in (i + 1)..n {
            let (pos_j, mass_j, radius_j) = snapshot[j];
            if mass_j <= 0.0 { continue; }

            let dir = pos_j - pos_i;
            let min_dist = radius_i + radius_j;
            let dist_sqr = dir.length_squared().max(min_dist * min_dist);
            let f = g * mass_i * mass_j / dist_sqr;
            let force = dir.normalize() * f;

            forces[i] += force;
            forces[j] -= force;
        }
    }

    particles.iter_mut().zip(forces.iter()).for_each(|(p, f)| {
        if p.mass > 0.0 {
            p.acc = *f / p.mass;
        }
    });
}