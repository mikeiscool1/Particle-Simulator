use std::collections::VecDeque;

use macroquad::prelude::*;

#[derive(Debug, Clone)]
pub struct Particle {
    pub pos: Vec3,
    pub vel: Vec3,
    pub acc: Vec3,
    pub mass: f32,
    pub friction: f32,
    pub radius: f32,
    pub color: Color,
    pub trail: VecDeque<Vec3>
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
        }
    }
}