use macroquad::prelude::*;
use crate::particle::Particle;

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
                    vel: vec3(
                        rand::gen_range(-25.0, 25.0),
                        rand::gen_range(-25.0, 25.0),
                        rand::gen_range(-25.0, 25.0),
                    ),
                    mass: 0.0,
                    color,
                    ..Default::default()
                });
            }
        }
    }

    // let ball1 = Particle {
    //     pos: vec3(0.0, -50.0, 0.0),
    //     vel: vec3(0.0, 0.0, 0.0),
    //     mass: 3.67e14,
    //     friction: 0.35,
    //     radius: 50.0,
    //     color: BLUE,
    //     ..Default::default()
    // };

    // let ball2 = Particle {
    //     pos: vec3(0.0, 10.0, 0.0),
    //     vel: vec3(0.0, 0.0, 10.0),
    //     mass: 100.0,
    //     friction: 0.35,
    //     color: GREEN,
    //     ..Default::default()
    // };

    // particles.push(ball1);
    // particles.push(ball2);
}

pub fn time_function(particles: &mut [Particle], t: f32) {
    let a = 3.0; // amplitude x
    let b = 2.0; // amplitude y
    let c = 1.0; // amplitude z

    let ax = 1.0; // frequency x
    let by = 2.0; // frequency y
    let cz = 3.0; // frequency z

    for (i, p) in particles.iter_mut().enumerate() {
        let k = i as f32 * 0.1; // small offset per particle so they don’t overlap exactly
        p.pos.x = a * ((ax * t + k).sin());
        p.pos.y = b * ((by * t + k).sin());
        p.pos.z = c * ((cz * t + k).sin());
    }
}