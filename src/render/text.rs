use std::collections::{HashMap, VecDeque};

use cosmic_text::{CacheKey, FontSystem, LayoutGlyph, SwashCache, SwashContent, SwashImage};

use crate::{
    graphics::Gpu,
    model::{Position, Size},
    render::texture::{Atlas, TextureHandle, TextureRegistry},
};

const GLYPH_PAGE_SIZE: u32 = 1024;

struct Page {
    id: usize,
    atlas: Atlas,
}

pub struct TextSystem {
    pages: VecDeque<Page>,
    page_cap: usize,
    current_page: usize,
    glyph_map: HashMap<CacheKey, (TextureHandle, usize)>,

    swash_cache: SwashCache,
    font_system: FontSystem,
}

impl Default for TextSystem {
    fn default() -> Self {
        Self {
            pages: VecDeque::new(),
            page_cap: 4,
            current_page: 0,
            glyph_map: HashMap::new(),
            swash_cache: SwashCache::new(),
            font_system: FontSystem::new(),
        }
    }
}

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

    pub fn get_glyph_data(
        &mut self,
        glyph: &LayoutGlyph,
    ) -> Option<(Position<i32>, Size<u32>, CacheKey)> {
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

        Some((
            Position::new(img.placement.left, img.placement.top),
            Size::new(gw, gh),
            phys.cache_key,
        ))
    }

    fn create_atlas(&mut self, gpu: &Gpu, texture_reg: &mut TextureRegistry) -> bool {
        if self.pages.len() >= self.page_cap {
            return false;
        }
        let id = self.pages.back().map(|p| p.id + 1).unwrap_or(0);
        let atlas = texture_reg.create_atlas(gpu, GLYPH_PAGE_SIZE, GLYPH_PAGE_SIZE);
        self.pages.push_back(Page { id, atlas });
        self.current_page = self.pages.len() - 1;
        true
    }

    fn recycle_oldest(&mut self, gpu: &Gpu, texture_reg: &mut TextureRegistry) {
        if let Some(Page { id, mut atlas }) = self.pages.pop_front() {
            texture_reg.destroy_atlas(gpu, &mut atlas);
            self.glyph_map.retain(|_, (_, page_id)| *page_id != id);
            let _ = self.create_atlas(gpu, texture_reg);
        }
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

        if let Some(&(handle, _)) = self.glyph_map.get(&key) {
            return Some(handle);
        }

        if self.pages.is_empty() && !self.create_atlas(gpu, texture_reg) {
            return Some(TextureHandle::default());
        }

        let img = self
            .swash_cache
            .get_image(&mut self.font_system, key)
            .as_ref()?;
        let rgba = premul_rgba(img);

        // Try current page
        if let Some(handle) =
            texture_reg.load_into_atlas(gpu, &mut self.pages[self.current_page].atlas, w, h, &rgba)
        {
            let id = self.pages[self.current_page].id;
            self.glyph_map.insert(key, (handle, id));
            return Some(handle);
        }

        // Try other pages
        for idx in 0..self.pages.len() {
            if idx == self.current_page {
                continue;
            }
            if let Some(handle) =
                texture_reg.load_into_atlas(gpu, &mut self.pages[idx].atlas, w, h, &rgba)
            {
                let id = self.pages[idx].id;
                self.glyph_map.insert(key, (handle, id));
                return Some(handle);
            }
        }

        // Allocate or recycle, then place
        if !self.create_atlas(gpu, texture_reg) {
            self.recycle_oldest(gpu, texture_reg);
        }
        if let Some(handle) =
            texture_reg.load_into_atlas(gpu, &mut self.pages[self.current_page].atlas, w, h, &rgba)
        {
            let id = self.pages[self.current_page].id;
            self.glyph_map.insert(key, (handle, id));
            return Some(handle);
        }

        Some(TextureHandle::default())
    }
}
