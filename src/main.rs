use macroquad::prelude::*;
use std::collections::VecDeque;

mod particle;
use particle::Particle;

struct State {
    show_grid: bool,
    use_cubes: bool,
    show_trail: bool,
    use_time_function: bool,
    min_merge_mass: f32,
    restitution: f32,
    g: f32,
    yaw: f32,
    pitch: f32,
    pos: Vec3,
    camera: Camera3D,
    last_mouse: Option<(f32, f32)>,
    is_fullscreen: bool,
    time: f32,
    time_warp: f32,
    speed: f32,
    alert_flash: f32,
    alert_flash_duration: f32,
    alert_text: String,
    clock_running: bool,
}

fn window_conf() -> Conf {
    Conf {
        window_title: "MyGame".to_owned(),
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut state = State {
        show_grid: false,
        use_cubes: true,
        show_trail: true,
        use_time_function: false,
        min_merge_mass: f32::INFINITY,
        restitution: 0.5,
        g: 6.67430e-11,
        yaw: -135.0f32.to_radians(),
        pitch: -45.0f32.to_radians(),
        pos: vec3(15.0, 15.0, 15.0),
        camera: Camera3D::default(),
        last_mouse: None,
        is_fullscreen: false,
        time: 0.0,
        time_warp: 1.0,
        speed: 1.0,
        alert_flash: 0.0,
        alert_flash_duration: 2.0,
        alert_text: String::new(),
        clock_running: false,
    };

    let mut particles: Vec<Particle> = Vec::new();
    set_particles(&mut particles);

    loop {
        let dt = get_frame_time();
        handle_input(&mut state, &mut particles, dt);

        if state.alert_flash > 0.0 {
            state.alert_flash = (state.alert_flash - dt).max(0.0);
        }

        clear_background(Color::new(0.1, 0.1, 0.15, 1.0));
        set_camera(&state.camera);

        if state.show_grid {
            draw_grid(&state.camera);
        }

        if state.clock_running {
            state.time += dt * state.time_warp;

            let sim_dt = dt * state.time_warp;

            if state.use_time_function {
                time_function(&mut particles, state.time);
            } else {
                update_particles_verlet(
                    &mut particles,
                    sim_dt,
                    state.restitution,
                    state.min_merge_mass,
                    state.g,
                );
            }
        }

        draw_particles(&particles, state.use_cubes, state.show_trail);

        if state.alert_flash > 0.0 {
            set_default_camera();

            let alpha = (state.alert_flash / state.alert_flash_duration).clamp(0.0, 1.0);

            draw_text_ex(
                &state.alert_text,
                25.0,
                49.0,
                TextParams {
                    font_size: 36,
                    color: Color::new(0.0, 0.0, 0.0, alpha),
                    ..Default::default()
                },
            );

            draw_text_ex(
                &state.alert_text,
                24.0,
                48.0,
                TextParams {
                    font_size: 36,
                    color: Color::new(1.0, 1.0, 1.0, alpha),
                    ..Default::default()
                },
            );
        }

        next_frame().await
    }
}

fn handle_input(state: &mut State, particles: &mut Vec<Particle>, dt: f32) {
// Full screen check
    if is_key_pressed(KeyCode::F11) {
        if !state.is_fullscreen {
            // Enter fullscreen
            set_fullscreen(true);
            state.is_fullscreen = true;
        } else {
            // Exit fullscreen
            set_fullscreen(false);
            state.is_fullscreen = false;
        }
    }

    if is_key_pressed(KeyCode::Space) {
        state.clock_running = !state.clock_running;

        if state.clock_running {
            state.alert_text = "Simulation Started".to_string();
        } else {
            state.alert_text = "Simulation Paused".to_string();
        }

        state.alert_flash = state.alert_flash_duration;
    }

    let sensitivity = 0.003;

    // Compute forward and right from yaw and pitch
    let forward = vec3(
        state.pitch.cos() * state.yaw.cos(),
        state.pitch.sin(),
        state.pitch.cos() * state.yaw.sin(),
    )
    .normalize();
    let right = forward.cross(vec3(0.0, 1.0, 0.0)).normalize();

    // Right click mouse look
    if is_mouse_button_down(MouseButton::Right) {
        let mouse = mouse_position();
        if let Some((lx, ly)) = state.last_mouse {
            let dx = mouse.0 - lx;
            let dy = mouse.1 - ly;
            state.yaw += dx * sensitivity;
            state.pitch -= dy * sensitivity;
            state.pitch = state.pitch.clamp(-89.0f32.to_radians(), 89.0f32.to_radians());
        }
        state.last_mouse = Some(mouse);
    } else {
        state.last_mouse = None;
    }

    // WASDEQ movement
    if is_key_down(KeyCode::W) { state.pos += forward * state.speed * 10.0 * dt; }
    if is_key_down(KeyCode::S) { state.pos -= forward * state.speed * 10.0 * dt; }
    if is_key_down(KeyCode::A) { state.pos -= right * state.speed * 10.0 * dt; }
    if is_key_down(KeyCode::D) { state.pos += right * state.speed * 10.0 * dt; }
    if is_key_down(KeyCode::E) { state.pos.y += state.speed * 10.0 * dt; }
    if is_key_down(KeyCode::Q) { state.pos.y -= state.speed * 10.0 * dt; }

    state.camera = Camera3D {
        position: state.pos,
        target: state.pos + forward,
        up: vec3(0.0, 1.0, 0.0),
        ..Default::default()
    };

    // Reset
    if is_key_pressed(KeyCode::R) {
        state.clock_running = false;
        state.time = 0.0;
        set_particles(particles);
        n_body_update(particles, state.g);

        state.alert_flash = state.alert_flash_duration;
        state.alert_text = "Simulation Reset".to_string();
    }

    // Setting toggles
    if is_key_pressed(KeyCode::F1) {
        state.time_warp *= 0.5;
        state.alert_flash = state.alert_flash_duration;
        state.alert_text = format!("Time Warp: {}x", format_dec(state.time_warp));
    }
    if is_key_pressed(KeyCode::F2) {
        state.time_warp *= 2.0;
        state.alert_flash = state.alert_flash_duration;
        state.alert_text = format!("Time Warp: {}x", format_dec(state.time_warp));
    }
    if is_key_pressed(KeyCode::G) {
        state.show_grid = !state.show_grid;
        state.alert_flash = state.alert_flash_duration;
        state.alert_text = format!("Grid: {}", if state.show_grid { "On" } else { "Off" });
    }
    if is_key_pressed(KeyCode::T) {
        state.show_trail = !state.show_trail;
        state.alert_flash = state.alert_flash_duration;
        state.alert_text = format!("Trails: {}", if state.show_trail { "On" } else { "Off" });
    }
    if is_key_pressed(KeyCode::C) {
        state.use_cubes = !state.use_cubes;
        state.alert_flash = state.alert_flash_duration;
        state.alert_text = format!("Render Mode: {}", if state.use_cubes { "Cubes" } else { "Spheres" });
    }

    if is_key_pressed(KeyCode::F3) {
        state.speed *= 0.5;
        state.alert_flash = state.alert_flash_duration;
        state.alert_text = format!("Speed: {}x", format_dec(state.speed));
    }
    if is_key_pressed(KeyCode::F4) {
        state.speed *= 2.0;
        state.alert_flash = state.alert_flash_duration;
        state.alert_text = format!("Speed: {}x", format_dec(state.speed));
    }
    if is_key_pressed(KeyCode::O) {
        state.yaw = -135.0f32.to_radians();
        state.pitch = -45.0f32.to_radians();
        state.pos = vec3(15.0, 15.0, 15.0);
        state.alert_flash = state.alert_flash_duration;
        state.alert_text = "Camera Reset".to_string();
    }
}

fn format_dec(value: f32) -> String {
    let formatted = format!("{:.6}", value);
    formatted.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn set_particles(particles: &mut Vec<Particle>) {
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

fn draw_grid(camera: &Camera3D) {
    let camera_pos = camera.position;
    let grid_spacing = 1.0;

    // Get the VP matrix to compute frustum reach
    let view = Mat4::look_at_rh(camera.position, camera.target, camera.up);
    let proj = Mat4::perspective_rh_gl(60f32.to_radians(), screen_width() / screen_height(), 0.1, 1000.0);
    let vp = proj * view;
    let inv_vp = vp.inverse();

    // Unproject the four far-plane corners to find max world extent
    let ndc_corners = [
        vec4(-1.0, -1.0, 1.0, 1.0),
        vec4( 1.0, -1.0, 1.0, 1.0),
        vec4(-1.0,  1.0, 1.0, 1.0),
        vec4( 1.0,  1.0, 1.0, 1.0),
    ];
    let grid_size = ndc_corners.iter().map(|&c| {
        let world = inv_vp * c;
        let world = world.xyz() / world.w;
        (world - camera_pos).length()
    }).fold(0.0f32, f32::max);

    let steps = (grid_size * 2.0 / grid_spacing) as i32;

    let ox = (camera_pos.x / grid_spacing).floor() * grid_spacing;
    let oy = (camera_pos.y / grid_spacing).floor() * grid_spacing;
    let oz = (camera_pos.z / grid_spacing).floor() * grid_spacing;

    let max_dist = grid_size * 1.5;
    let fade = |dist: f32| -> f32 { 1.0 - (dist / max_dist).clamp(0.0, 1.0) };

    for i in 0..=steps {
        let t = -grid_size + i as f32 * grid_spacing;

        let xz_color = Color::new(0.5, 0.5, 0.5, fade((vec3(0.0, 0.0, oz + t) - camera_pos).length()) * 0.8 + 0.1);
        let xy_color = Color::new(0.4, 0.4, 0.5, fade((vec3(0.0, oy + t, 0.0) - camera_pos).length()) * 0.7 + 0.1);
        let yz_color = Color::new(0.5, 0.4, 0.4, fade((vec3(ox + t, 0.0, 0.0) - camera_pos).length()) * 0.7 + 0.1);

        // XZ plane
        draw_line_3d(vec3(ox - grid_size, 0.0, oz + t), vec3(ox + grid_size, 0.0, oz + t), xz_color);
        draw_line_3d(vec3(ox + t, 0.0, oz - grid_size), vec3(ox + t, 0.0, oz + grid_size), xz_color);

        // XY plane
        draw_line_3d(vec3(ox - grid_size, oy + t, 0.0), vec3(ox + grid_size, oy + t, 0.0), xy_color);
        draw_line_3d(vec3(ox + t, oy - grid_size, 0.0), vec3(ox + t, oy + grid_size, 0.0), xy_color);

        // YZ plane
        draw_line_3d(vec3(0.0, oy - grid_size, oz + t), vec3(0.0, oy + grid_size, oz + t), yz_color);
        draw_line_3d(vec3(0.0, oy + t, oz - grid_size), vec3(0.0, oy + t, oz + grid_size), yz_color);
    }

    draw_line_3d(vec3(ox - grid_size, 0.0, 0.0), vec3(ox + grid_size, 0.0, 0.0), BLUE);
    draw_line_3d(vec3(0.0, oy - grid_size, 0.0), vec3(0.0, oy + grid_size, 0.0), BLUE);
    draw_line_3d(vec3(0.0, 0.0, oz - grid_size), vec3(0.0, 0.0, oz + grid_size), BLUE);
}

fn update_particles_verlet(
    particles: &mut Vec<Particle>,
    dt: f32,
    restitution: f32,
    min_merge_mass: f32,
    g: f32,
) {
    let old_acc: Vec<Vec3> = particles.iter().map(|p| p.acc).collect();

    for p in particles.iter_mut() {
        p.verlet_drift(dt);
    }

    resolve_collisions(particles, restitution, min_merge_mass, g);
    n_body_update(particles, g);

    for (p, &prev_acc) in particles.iter_mut().zip(old_acc.iter()) {
        p.verlet_kick(prev_acc, dt);
    }

    for p in particles.iter_mut() { p.update_trail(); }
}

fn resolve_collisions(particles: &mut Vec<Particle>, restitution: f32, min_merge_mass: f32, g: f32) {
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

                let should_merge = particles[i].mass >= min_merge_mass && particles[j].mass >= min_merge_mass && approach_speed < escape_velocity;

                if should_merge {
                    merge_particles(particles, i, j);
                    continue;
                }

                if relative_speed_normal < 0.0 {
                    let m1 = particles[i].mass;
                    let m2 = particles[j].mass;
                    let inv_mass_sum = 1.0 / m1 + 1.0 / m2;
                    let normal_impulse = -(1.0 + restitution) * relative_speed_normal / inv_mass_sum;

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
    };

    particles.swap_remove(j);
}

fn n_body_update(particles: &mut [Particle], g: f32) {
    let n = particles.len();

    for i in 0..n {
        if particles[i].mass <= 0.0 {
            continue; // skip massless particles
        }

        let mut force = vec3(0.0, 0.0, 0.0);
        for j in 0..n {
            if i != j && particles[j].mass > 0.0 {
                let dir = particles[j].pos - particles[i].pos;
                let min_dist = particles[i].radius + particles[j].radius;

                let dist_sqr = dir.length_squared().max(min_dist * min_dist); // prevent singularity
                let f = g * particles[i].mass * particles[j].mass / dist_sqr;
                force += dir.normalize() * f;
            }
        }
        particles[i].acc = force / particles[i].mass;
    }
}

fn time_function(particles: &mut [Particle], t: f32) {
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

fn draw_particles(particles: &[Particle], use_cubes: bool, show_trail: bool) {
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