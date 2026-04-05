use macroquad::color::Color;
use serde::{Deserialize, Serialize};

pub mod serde_color {
    use super::*;

    pub fn serialize<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let (r, g, b, a) = (color.r, color.g, color.b, color.a);
        (r, g, b, a).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (r, g, b, a) = <(f32, f32, f32, f32)>::deserialize(deserializer)?;
        Ok(Color { r, g, b, a })
    }
}
