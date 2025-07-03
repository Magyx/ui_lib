use crate::Size;
use crate::widget::Text;
use crate::widget::TextStyle;
use wgpu::SurfaceConfiguration;

pub struct TextBundle {
    font_system: glyphon::FontSystem,
    swash_cache: glyphon::SwashCache,
    atlas: glyphon::TextAtlas,
    viewport: glyphon::Viewport,
    text_renderer: glyphon::TextRenderer,

    buffers: Vec<glyphon::Buffer>,
    is_ready: bool,
}

impl<'a> TextBundle {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &SurfaceConfiguration,
    ) -> Self {
        let mut font_system = glyphon::FontSystem::new();
        font_system.db_mut().load_system_fonts();

        let swash_cache = glyphon::SwashCache::new();
        let cache = glyphon::Cache::new(&device);
        let mut atlas = glyphon::TextAtlas::new(&device, &queue, &cache, config.format);
        let viewport = glyphon::Viewport::new(device, &cache);
        let text_renderer = glyphon::TextRenderer::new(
            &mut atlas,
            &device,
            wgpu::MultisampleState::default(),
            None,
        );

        Self {
            font_system,
            swash_cache,
            atlas,
            viewport,
            text_renderer,
            buffers: Vec::new(),
            is_ready: true,
        }
    }

    pub(crate) fn update(&mut self, texts: &[Text]) {
        self.buffers.clear();

        for t in texts {
            let metrics = glyphon::Metrics::new(t.style.font_size, t.style.font_size * 1.4);
            let mut buf = glyphon::Buffer::new(&mut self.font_system, metrics);
            buf.set_size(
                &mut self.font_system,
                Some(t.size.width as f32),
                Some(t.size.height as f32),
            );

            let mut attrs = glyphon::Attrs::new().family(glyphon::Family::SansSerif);
            attrs = attrs.weight(t.style.weight);
            if t.style.italic {
                attrs.style(glyphon::Style::Italic);
            }
            buf.set_text(
                &mut self.font_system,
                &t.content,
                &glyphon::Attrs::new(),
                glyphon::Shaping::Advanced,
            );

            self.buffers.push(buf);
        }

        self.is_ready = false;
    }

    pub(crate) fn resize(&mut self, queue: &wgpu::Queue, size: &Size<u32>) {
        self.viewport.update(
            queue,
            glyphon::Resolution {
                width: size.width,
                height: size.height,
            },
        );
        self.is_ready = false;
    }

    pub(crate) fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, texts: &[Text]) {
        if self.is_ready {
            return;
        }

        let text_areas: Vec<_> = texts
            .iter()
            .zip(&self.buffers)
            .map(|(text, buffer)| crate::utils::glyphon::to_text(text, buffer))
            .collect();

        self.text_renderer
            .prepare(
                device,
                queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .expect("glyphon prepare failed");

        self.is_ready = true;
    }

    pub(crate) fn render(
        &self,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        clear_color: &mut Option<wgpu::Color>,
    ) -> Result<(), glyphon::RenderError> {
        if self.buffers.is_empty() {
            return Ok(());
        }

        let load = if let Some(clear_color) = clear_color.take() {
            wgpu::LoadOp::Clear(clear_color)
        } else {
            wgpu::LoadOp::Load
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Text Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        self.text_renderer
            .render(&self.atlas, &self.viewport, &mut render_pass)
    }

    pub(crate) fn get_min_size(&mut self, style: &TextStyle, content: &str) -> Size<f32> {
        let metrics = glyphon::Metrics::new(style.font_size, style.font_size * 1.4);
        let mut buf = glyphon::Buffer::new(&mut self.font_system, metrics);

        let mut attrs = glyphon::Attrs::new().family(glyphon::Family::SansSerif);
        attrs = attrs.weight(style.weight);
        if style.italic {
            attrs = attrs.style(glyphon::Style::Italic);
        }
        buf.set_text(
            &mut self.font_system,
            &content,
            &attrs,
            glyphon::Shaping::Advanced,
        );

        let width = buf.layout_runs().map(|run| run.line_w).fold(0.0, f32::max);
        let height = buf.metrics().line_height;

        Size { width, height }
    }
}
