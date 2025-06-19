pub mod wgsl {
    pub fn load_wgsl(
        device: &wgpu::Device,
        path: &std::path::Path,
        label: &str,
    ) -> wgpu::ShaderModule {
        use std::{borrow::Cow, fs};
        let src = fs::read_to_string(path).expect("wgsl not found");
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(Cow::Owned(src)),
        })
    }
}

pub mod glyphon {
    use glyphon::{Buffer, Color, TextArea, TextBounds};

    use crate::widget::Text;

    pub fn to_text<'a>(text: &Text, buffer: &'a Buffer) -> TextArea<'a> {
        TextArea {
            buffer,
            left: text.position.x as f32,
            top: text.position.y as f32,
            scale: 1.0,
            bounds: TextBounds {
                left: text.position.x,
                top: text.position.y,
                right: text.position.x + text.size.width as i32,
                bottom: text.position.y + text.size.height as i32,
            },
            default_color: Color::rgba(
                (text.style.color.r * 255.0) as u8,
                (text.style.color.g * 255.0) as u8,
                (text.style.color.b * 255.0) as u8,
                (text.style.color.a * 255.0) as u8,
            ),
            custom_glyphs: &[],
        }
    }
}
