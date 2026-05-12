use cosmic_text::{CacheKey, FontSystem, LayoutGlyph, SwashCache, SwashContent, SwashImage};

use crate::{
    graphics::Gpu,
    model::{Position, Size},
    render::{
        glyph_atlas::GlyphAtlas,
        texture::{TextureHandle, TextureRegistry},
    },
};

fn premul_rgba(img: &SwashImage) -> Vec<u8> {
    match img.content {
        SwashContent::Mask => {
            let a = &img.data;
            let mut out = Vec::with_capacity(a.len() * 4);
            for &aa in a {
                out.extend_from_slice(&[aa, aa, aa, aa]); // RGB=A, A=A
            }
            out
        }
        SwashContent::SubpixelMask => {
            let m = &img.data;
            let mut out = Vec::with_capacity(m.len() / 3 * 4);
            for px in m.chunks_exact(3) {
                let (r, g, b) = (px[0], px[1], px[2]);
                let a = r.max(g).max(b);
                out.extend_from_slice(&[r, g, b, a]); // RGB=RGB, A=max(R,G,B)
            }
            out
        }
        SwashContent::Color => {
            let p = &img.data;
            let mut out = Vec::with_capacity(p.len());
            for px in p.chunks_exact(4) {
                let (r, g, b, a) = (px[0] as u16, px[1] as u16, px[2] as u16, px[3] as u16);
                let pr = (r * a / 255) as u8;
                let pg = (g * a / 255) as u8;
                let pb = (b * a / 255) as u8;
                out.extend_from_slice(&[pr, pg, pb, a as u8]); // RGB=RGB*A, A=A
            }
            out
        }
    }
}

const GLYPH_PAGE_SIZE: u32 = 1024;

pub struct TextSystem {
    glyph_atlas: GlyphAtlas,
    swash_cache: SwashCache,
    font_system: FontSystem,
}

impl Default for TextSystem {
    fn default() -> Self {
        Self {
            glyph_atlas: GlyphAtlas::default(),
            swash_cache: SwashCache::new(),
            font_system: FontSystem::new(),
        }
    }
}

impl TextSystem {
    pub fn font_system(&self) -> &FontSystem {
        &self.font_system
    }

    pub fn font_system_mut(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }

    pub fn swash_cache(&self) -> &SwashCache {
        &self.swash_cache
    }

    pub fn swash_cache_mut(&mut self) -> &mut SwashCache {
        &mut self.swash_cache
    }

    pub(crate) fn tick(&mut self) {
        self.glyph_atlas.tick();
    }

    pub fn prepare_glyph_data(&mut self, glyph: &LayoutGlyph) -> Option<(Size<u32>, CacheKey)> {
        let phys = glyph.physical((0.0, 0.0), 1.0);
        let img = self
            .swash_cache
            .get_image(&mut self.font_system, phys.cache_key)
            .as_ref()?;

        if img.placement.width == 0 || img.placement.height == 0 {
            return None;
        }

        let gw = img.placement.width;
        let gh = img.placement.height;

        Some((Size::new(gw, gh), phys.cache_key))
    }

    pub fn get_glyph_data(
        &self,
        glyph: &LayoutGlyph,
        origin: (f32, f32),
        line_y: f32,
    ) -> Option<(Position<i32>, Size<u32>, CacheKey)> {
        let phys = glyph.physical((origin.0, origin.1 + line_y), 1.0);
        let img = self
            .swash_cache
            .image_cache
            .get(&phys.cache_key)?
            .as_ref()?;

        if img.placement.width == 0 || img.placement.height == 0 {
            return None;
        }

        Some((
            Position::new(phys.x + img.placement.left, phys.y - img.placement.top),
            Size::new(img.placement.width, img.placement.height),
            phys.cache_key,
        ))
    }

    pub fn upload_glyph(
        &mut self,
        gpu: &Gpu,
        texture_reg: &mut TextureRegistry,
        key: CacheKey,
        w: u32,
        h: u32,
    ) -> Option<TextureHandle> {
        if w == 0 || h == 0 {
            return Some(TextureHandle::default());
        }
        if w > GLYPH_PAGE_SIZE || h > GLYPH_PAGE_SIZE {
            return Some(TextureHandle::default());
        }

        if let Some(handle) = self.glyph_atlas.lookup(&key) {
            self.glyph_atlas.touch(key);
            return Some(handle);
        }

        let img = self
            .swash_cache
            .get_image(&mut self.font_system, key)
            .as_ref()?;
        let rgba = premul_rgba(img);

        self.glyph_atlas.upload(gpu, texture_reg, key, w, h, &rgba)
    }

    pub fn lookup_glyph_handle(&self, key: CacheKey) -> Option<TextureHandle> {
        self.glyph_atlas.lookup(&key)
    }
}
