use crate::component::particles::Particle;
use macroquad::prelude::*;
use std::fmt::Display;

#[derive(Clone, Copy)]
pub enum Demo {
    Gravity,
    Explosion,
    ExplosionGravity,
    Vortex,
    None,
}

impl Default for Demo {
    fn default() -> Self {
        Demo::None
    }
}

impl Display for Demo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Demo::Gravity => "Gravity",
            Demo::Explosion => "Explosion",
            Demo::ExplosionGravity => "Explosion + Gravity",
            Demo::Vortex => "Vortex",
            Demo::None => "None",
        };
        write!(f, "{}", name)
    }
}

pub fn set_particles(particles: &mut Vec<Particle>, n: u32, demo: Demo) {
    *particles = vec![];

    let spacing = 1.0;
    let offset = spacing * 5.0;

    // Precompute grid points sorted by Chebyshev distance (fills cube from inside out)
    let n3 = (n as f32).cbrt().ceil() as u32;
    let mut grid: Vec<(u32, u32, u32)> = (0..n3)
        .flat_map(|x| (0..n3).flat_map(move |y| (0..n3).map(move |z| (x, y, z))))
        .collect();
    grid.sort_by_key(|&(x, y, z)| x.max(y).max(z));

    for i in 0..n {
        let (x, y, z) = grid[i as usize];

        let pos = vec3(
            x as f32 * spacing - offset,
            y as f32 * spacing - offset,
            z as f32 * spacing - offset,
        );
        let color = Color::new(
            rand::gen_range(0.5, 1.0),
            rand::gen_range(0.5, 1.0),
            rand::gen_range(0.5, 1.0),
            1.0,
        );
        let particle = match demo {
            Demo::Gravity => Particle {
                pos,
                vel: vec3(0.0, 0.0, 0.0),
                mass: 1e8,
                color,
                friction: 0.5,
                ..Default::default()
            },
            Demo::Explosion => Particle {
                pos,
                vel: vec3(
                    rand::gen_range(-25.0, 25.0),
                    rand::gen_range(-25.0, 25.0),
                    rand::gen_range(-25.0, 25.0),
                ),
                mass: 0.0,
                color,
                friction: 0.5,
                ..Default::default()
            },
            Demo::ExplosionGravity => Particle {
                pos,
                vel: vec3(
                    rand::gen_range(-25.0, 25.0),
                    rand::gen_range(-25.0, 25.0),
                    rand::gen_range(-25.0, 25.0),
                ),
                mass: 3e10,
                color,
                friction: 0.5,
                ..Default::default()
            },
            Demo::Vortex => Particle {
                pos,
                vel: vec3(-pos.z, 0.0, pos.x).normalize_or_zero() * 4.0,
                mass: 2e9,
                color,
                friction: 0.5,
                ..Default::default()
            },
            Demo::None => Particle {
                pos,
                vel: vec3(0.0, 0.0, 0.0),
                mass: 0.0,
                color,
                friction: 0.5,
                ..Default::default()
            },
        };

        particles.push(particle);
    }
}
