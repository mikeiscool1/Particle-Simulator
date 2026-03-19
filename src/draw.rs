use macroquad::prelude::*;
use crate::particle::Particle;

pub fn draw_particles(particles: &[Particle], use_cubes: bool, show_trail: bool) {
    for p in particles {
        if use_cubes {
            draw_cube(p.pos, vec3(p.radius * 2.0, p.radius * 2.0, p.radius * 2.0), None, p.color);
        } else {
            draw_sphere_ex(p.pos, p.radius, None, p.color, sphere_lod_params(p.radius));
        }
    }

    if show_trail {
        for p in particles {
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