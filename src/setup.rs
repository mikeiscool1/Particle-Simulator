use macroquad::prelude::*;
use crate::component::particles::Particle;

pub fn set_particles(particles: &mut Vec<Particle>) {
    *particles = vec![];

    let spacing = 1.0;
    let offset = spacing * 5.0;
    for x in 0..10 {
        for y in 0..10 {
            for z in 0..10 {
                let pos = vec3(x as f32 * spacing - offset, y as f32
                    * spacing - offset, z as f32 * spacing - offset);
                let color = Color::new(
                    rand::gen_range(0.5, 1.0),
                    rand::gen_range(0.5, 1.0),
                    rand::gen_range(0.5, 1.0),
                    1.0,
                );
                particles.push(Particle {
                    pos,
                    // vel: vec3(
                    //     rand::gen_range(-25.0, 25.0),
                    //     rand::gen_range(-25.0, 25.0),
                    //     rand::gen_range(-25.0, 25.0),
                    // ),
                    vel: vec3(0.0, 0.0, 0.0),
                    mass: 1e8,
                    color,
                    friction: 0.5,
                    ..Default::default()
                });
            }
        }
    }
}