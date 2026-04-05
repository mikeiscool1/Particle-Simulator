use crate::component::Particle;
use macroquad::{Error, prelude::*};
use std::sync::LazyLock;

pub mod sphere {
    use super::*;

    pub const SPHERE_FRAGMENT: &str = r#"#version 300 es
precision highp float;

in vec3 frag_pos;
in vec4 vert_color;
in vec3 v_sphere_center;
in float v_sphere_radius;

out vec4 fragColor;

uniform vec3 camera_pos;
uniform mat4 ViewProj;

void main() {
    vec3 ro = camera_pos;
    vec3 rd = normalize(frag_pos - camera_pos);
    vec3 oc = ro - v_sphere_center;
    
    float b = dot(oc, rd);
    float c = dot(oc, oc) - v_sphere_radius * v_sphere_radius;
    float h = b * b - c;

    // Ray misses the sphere entirely
    if (h < 0.0) discard;
    h = sqrt(h);

    float tN = -b - h; // Near hit
    float tF = -b + h; // Far hit

    // If the far hit is behind us, the sphere is not visible
    if (tF < 0.0) discard;

    float t = max(tN, 0.0);

    vec3 hit = ro + t * rd;

    // Update depth buffer to match the ray-marched surface
    vec4 clip = ViewProj * vec4(hit, 1.0);
    gl_FragDepth = (clip.z / clip.w) * 0.5 + 0.5;

    // Calculate normal
    vec3 normal = normalize(hit - v_sphere_center);

    // Shading matching your cube's "light" style
    float diffuse = max(dot(normal, normalize(vec3(1.0, 1.0, 1.0))), 0.0);
    float light = 0.6 + 0.4 * diffuse;

    vec3 view_dir = normalize(camera_pos - hit);
    vec3 light_dir = normalize(vec3(1.0, 1.0, 1.0));
    vec3 reflect_dir = reflect(-light_dir, normal);
    float spec = pow(max(dot(view_dir, reflect_dir), 0.0), 4.0);
    float specular = 0.15 * spec;

    fragColor = vec4(vert_color.rgb * light + specular, vert_color.a);
}
"#;

    pub const SPHERE_VERTEX: &str = r#"#version 300 es
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
                    UniformDesc::new("ViewProj", UniformType::Mat4),
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

    pub static SPHERE_MATERIAL: LazyLock<Material> =
        LazyLock::new(|| create_sphere_material().unwrap());

    pub fn draw_spheres_batched(particles: &[&Particle], camera_pos: Vec3, vp: Mat4) {
        let mut vertices = Vec::with_capacity(particles.len() * 4);
        let mut indices = Vec::with_capacity(particles.len() * 6);

        let to_y = Vec3::Y;
        let mut vertex_count = 0;

        for p in particles.iter() {
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

            let sin_alpha = p.radius / dist;
            let cos_alpha = (1.0 - sin_alpha * sin_alpha).sqrt();

            let r = p.radius / cos_alpha;

            let positions = [
                p.pos + (-right - up) * r,
                p.pos + (right - up) * r,
                p.pos + (right + up) * r,
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

            indices.extend_from_slice(&[
                vertex_count,
                vertex_count + 1,
                vertex_count + 2,
                vertex_count,
                vertex_count + 2,
                vertex_count + 3,
            ]);

            vertex_count += 4;
        }

        let material = &SPHERE_MATERIAL;
        material.set_uniform("camera_pos", camera_pos);
        material.set_uniform("ViewProj", vp);

        gl_use_material(material);
        draw_mesh(&Mesh {
            vertices,
            indices,
            texture: None,
        });
        gl_use_default_material();
    }
}

pub mod cube {
    use super::*;

    pub const CUBE_FRAGMENT: &str = r#"#version 300 es
precision highp float;

in vec3 frag_pos;
in vec4 vert_color;
in vec3 v_cube_center;
in float v_half_size;

out vec4 fragColor;

uniform vec3 camera_pos;
uniform mat4 ViewProj;

void main() {
    vec3 ro = camera_pos;
    vec3 rd = normalize(frag_pos - camera_pos);
    vec3 center = v_cube_center;
    float hs = v_half_size;

    vec3 m = 1.0 / rd;
    vec3 n = m * (ro - center);
    vec3 k = abs(m) * hs;
    vec3 t1 = -n - k;
    vec3 t2 = -n + k;
    float tN = max(max(t1.x, t1.y), t1.z);
    float tF = min(min(t2.x, t2.y), t2.z);

    if (tN > tF || tF < 0.0) discard;

    float t = max(tN, 0.0);
    vec3 hit = ro + t * rd;

    vec4 clip = ViewProj * vec4(hit, 1.0);
    gl_FragDepth = (clip.z / clip.w) * 0.5 + 0.5;

    // compute face normal for shading
    vec3 local = hit - center;
    vec3 a = abs(local) / hs;
    vec3 normal = sign(local) * step(a.yzx, a.xyz) * step(a.zxy, a.xyz);

    vec3 normal_abs = abs(normal);
    float face_shade;
    if (normal_abs.x > normal_abs.y && normal_abs.x > normal_abs.z) {
        face_shade = normal.x > 0.0 ? 1.0 : 0.7;  // right/left
    } else if (normal_abs.y > normal_abs.z) {
        face_shade = normal.y > 0.0 ? 0.9 : 0.5;  // top/bottom
    } else {
        face_shade = normal.z > 0.0 ? 0.8 : 0.6;  // front/back
    }
    float light = face_shade;

    fragColor = vec4(vert_color.rgb * light, vert_color.a);
}
"#;

    pub const CUBE_VERTEX: &str = r#"#version 300 es
in vec3 position;
in vec4 color0;
in vec4 normal;

out vec3 frag_pos;
out vec4 vert_color;
out vec3 v_cube_center;
out float v_half_size;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    vec4 world_pos = Model * vec4(position, 1.0);
    frag_pos = world_pos.xyz;
    vert_color = color0 / 255.0;
    v_cube_center = normal.xyz;
    v_half_size = normal.w;
    gl_Position = Projection * world_pos;
}
"#;

    pub fn create_cube_material() -> Result<Material, Error> {
        load_material(
            ShaderSource::Glsl {
                vertex: CUBE_VERTEX,
                fragment: CUBE_FRAGMENT,
            },
            MaterialParams {
                uniforms: vec![
                    UniformDesc::new("camera_pos", UniformType::Float3),
                    UniformDesc::new("ViewProj", UniformType::Mat4),
                ],
                pipeline_params: PipelineParams {
                    depth_write: true,
                    depth_test: Comparison::LessOrEqual,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    }

    pub static CUBE_MATERIAL: LazyLock<Material> =
        LazyLock::new(|| create_cube_material().unwrap());

    pub fn draw_cubes_batched(particles: &[&Particle], camera_pos: Vec3, vp: Mat4) {
        let mut vertices: Vec<Vertex> = Vec::with_capacity(particles.len() * 4);
        let mut indices: Vec<u16> = Vec::with_capacity(particles.len() * 6);

        let mut vertex_count = 0;

        for p in particles.iter() {
            let half_size = p.radius;
            let dist = (camera_pos - p.pos).length();

            let to_cam = (camera_pos - p.pos).normalize();
            let right = if to_cam.dot(Vec3::Y).abs() < 0.99 {
                to_cam.cross(Vec3::Y).normalize()
            } else {
                to_cam.cross(Vec3::Z).normalize()
            };
            let up = to_cam.cross(right).normalize();

            // Hide if we're inside of the cube
            if (camera_pos.x - p.pos.x).abs() < half_size
                && (camera_pos.y - p.pos.y).abs() < half_size
                && (camera_pos.z - p.pos.z).abs() < half_size
            {
                continue;
            }

            let max_radius = half_size * 1.732; // sqrt(3), worst case corner

            let safe_dist = dist.max(max_radius * 1.001);

            let sin_alpha = max_radius / safe_dist;
            let cos_alpha = (1.0 - sin_alpha * sin_alpha).sqrt();

            let r = max_radius / cos_alpha;

            let positions = [
                p.pos + (-right - up) * r,
                p.pos + (right - up) * r,
                p.pos + (right + up) * r,
                p.pos + (-right + up) * r,
            ];

            let packed_normal = Vec4::new(p.pos.x, p.pos.y, p.pos.z, half_size);

            for pos in positions {
                vertices.push(Vertex {
                    position: pos,
                    uv: Vec2::ZERO,
                    color: p.color.into(),
                    normal: packed_normal,
                });
            }

            indices.extend_from_slice(&[
                vertex_count,
                vertex_count + 1,
                vertex_count + 2,
                vertex_count,
                vertex_count + 2,
                vertex_count + 3,
            ]);

            vertex_count += 4;
        }

        let material = &CUBE_MATERIAL;
        material.set_uniform("camera_pos", camera_pos);
        material.set_uniform("ViewProj", vp);
        gl_use_material(material);
        draw_mesh(&Mesh {
            vertices,
            indices,
            texture: None,
        });
        gl_use_default_material();
    }
}
