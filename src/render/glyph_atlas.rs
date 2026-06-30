use std::collections::{HashMap, VecDeque};

use cosmic_text::CacheKey;

use crate::{
    graphics::Gpu,
    render::{
        alloc::AllocatorKind,
        texture::{Atlas, TextureHandle, TextureRegistry},
    },
};

/// Side length of each GPU atlas page in pixels.
pub(crate) const GLYPH_PAGE_SIZE: u32 = 1024;

/// Upper bounds of bucket 0 and bucket 1 (inclusive), in glyph pixels.
///
/// | Bucket | Height range | Typical content                  |
/// |--------|-------------|----------------------------------|
/// | 0      | ≤ 16 px     | Small UI text, most ASCII        |
/// | 1      | 17 – 32 px  | Medium headings, larger labels   |
/// | 2      | > 32 px     | Large display text, emoji        |
const BUCKET_THRESHOLDS: [u32; 2] = [16, 32];
const NUM_BUCKETS: usize = 3;

/// Maximum pages per bucket before LRU eviction triggers.
const BUCKET_CAPS: [usize; NUM_BUCKETS] = [
    2, // small glyphs get two pages; they dominate traffic
    1, 1,
];

const fn bucket_for_height(h: u32) -> usize {
    if h <= BUCKET_THRESHOLDS[0] {
        0
    } else if h <= BUCKET_THRESHOLDS[1] {
        1
    } else {
        2
    }
}

struct Page {
    id: usize,
    atlas: Atlas,
}

struct PagePool {
    pages: VecDeque<Page>,
    cap: usize,
    /// The page we tried to allocate into most recently.
    current_page_id: Option<usize>,
}

impl PagePool {
    fn new(cap: usize) -> Self {
        Self {
            pages: VecDeque::new(),
            cap,
            current_page_id: None,
        }
    }

    fn is_full(&self) -> bool {
        self.pages.len() >= self.cap
    }
}

/// GPU atlas storage for rasterized glyphs.
///
/// Manages multiple 1024×1024 atlas pages split into three height-based
/// buckets. When a bucket fills beyond its page cap, the least-recently-used
/// page is recycled and its glyphs evicted from the map.
///
/// Call [`tick`](GlyphAtlas::tick) once per render pass (before prepare) to
/// advance the frame counter used for LRU tracking.
pub(crate) struct GlyphAtlas {
    buckets: [PagePool; NUM_BUCKETS],
    next_page_id: usize,
    page_allocator: AllocatorKind,

    /// Maps a `CacheKey` to its `TextureHandle` and the `page_id` that owns it.
    glyph_map: HashMap<CacheKey, (TextureHandle, usize)>,
    /// Maps `page_id` to the last frame it was touched.
    page_last_used: HashMap<usize, u64>,
    frame: u64,
}
impl Default for GlyphAtlas {
    fn default() -> Self {
        Self::new(AllocatorKind::Shelf)
    }
}
impl GlyphAtlas {
    pub(crate) fn new(page_allocator: AllocatorKind) -> Self {
        Self {
            buckets: [
                PagePool::new(BUCKET_CAPS[0]),
                PagePool::new(BUCKET_CAPS[1]),
                PagePool::new(BUCKET_CAPS[2]),
            ],
            next_page_id: 0,
            page_allocator,
            glyph_map: HashMap::new(),
            page_last_used: HashMap::new(),
            frame: 0,
        }
    }

    /// Advance the frame counter. Must be called once per render pass,
    /// before the prepare phase, so that LRU timestamps are meaningful.
    pub(crate) fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }

    /// Non-mutating lookup. Used during the paint phase where `TextSystem`
    /// is only available by shared reference.
    pub(crate) fn lookup(&self, key: &CacheKey) -> Option<TextureHandle> {
        self.glyph_map.get(key).map(|&(handle, _)| handle)
    }

    /// Mark the page that owns `key` as accessed this frame.
    /// Called on cache hits in `TextSystem::upload_glyph` so that frequently
    /// re-rendered glyphs are never treated as cold.
    pub(crate) fn touch(&mut self, key: CacheKey) {
        if let Some(&(_, page_id)) = self.glyph_map.get(&key) {
            self.page_last_used.insert(page_id, self.frame);
        }
    }

    /// Upload `rgba` pixels for `key` into the appropriate bucket page.
    ///
    /// The caller is responsible for:
    /// - Checking `lookup` first and short-circuiting on a cache hit.
    /// - Providing correctly pre-multiplied RGBA data of size `w × h × 4`.
    ///
    /// Returns the handle on success. Returns `None` only if the glyph is
    /// wider/taller than a full atlas page — which should never happen for
    /// real glyph data.
    pub(crate) fn upload(
        &mut self,
        gpu: &Gpu,
        texture_reg: &mut TextureRegistry,
        key: CacheKey,
        w: u32,
        h: u32,
        rgba: &[u8],
    ) -> Option<TextureHandle> {
        let bucket_idx = bucket_for_height(h);

        // Fast path: there's room in an existing page.
        if let Some(handle) = self.try_insert(gpu, texture_reg, bucket_idx, key, w, h, rgba) {
            return Some(handle);
        }

        // Slow path: all pages in this bucket are full.
        if !self.buckets[bucket_idx].is_full() {
            self.alloc_page_in_bucket(bucket_idx, gpu, texture_reg);
        } else {
            // Evict the coldest page and replace it with a fresh one.
            self.recycle_lru_in_bucket(bucket_idx, gpu, texture_reg);
        }

        // One more attempt on the freshly created page.
        self.try_insert(gpu, texture_reg, bucket_idx, key, w, h, rgba)
    }

    /// Try to insert `rgba` into any existing page in `bucket_idx`,
    /// preferring `current_page_id`. Returns the handle on success.
    #[allow(clippy::too_many_arguments)]
    fn try_insert(
        &mut self,
        gpu: &Gpu,
        texture_reg: &mut TextureRegistry,
        bucket_idx: usize,
        key: CacheKey,
        w: u32,
        h: u32,
        rgba: &[u8],
    ) -> Option<TextureHandle> {
        if self.buckets[bucket_idx].pages.is_empty() {
            return None;
        }

        let current_id = self.buckets[bucket_idx].current_page_id;
        let n = self.buckets[bucket_idx].pages.len();

        // Build a visit order: current page first, then the rest.
        let mut order: Vec<usize> = Vec::with_capacity(n);
        if let Some(cid) = current_id
            && let Some(pos) = self.buckets[bucket_idx]
                .pages
                .iter()
                .position(|p| p.id == cid)
        {
            order.push(pos);
        }
        for i in 0..n {
            if !order.contains(&i) {
                order.push(i);
            }
        }

        for &pos in &order {
            // Copy id before the mutable borrow of atlas.
            let id = self.buckets[bucket_idx].pages[pos].id;
            let handle = texture_reg.load_into_atlas(
                gpu,
                &mut self.buckets[bucket_idx].pages[pos].atlas,
                w,
                h,
                rgba,
            );
            if let Some(handle) = handle {
                self.buckets[bucket_idx].current_page_id = Some(id);
                self.glyph_map.insert(key, (handle, id));
                self.page_last_used.insert(id, self.frame);
                return Some(handle);
            }
        }

        None
    }

    fn alloc_page_in_bucket(
        &mut self,
        bucket_idx: usize,
        gpu: &Gpu,
        texture_reg: &mut TextureRegistry,
    ) {
        let id = self.next_page_id;
        self.next_page_id = self.next_page_id.wrapping_add(1);

        let atlas =
            texture_reg.create_atlas(gpu, GLYPH_PAGE_SIZE, GLYPH_PAGE_SIZE, self.page_allocator);
        let pool = &mut self.buckets[bucket_idx];
        pool.pages.push_back(Page { id, atlas });
        pool.current_page_id = Some(id);
        self.page_last_used.insert(id, self.frame);

        #[cfg(feature = "tracing")]
        tracing::debug!(
            bucket = bucket_idx,
            page_id = id,
            "glyph atlas: allocated new page"
        );
    }

    fn recycle_lru_in_bucket(
        &mut self,
        bucket_idx: usize,
        gpu: &Gpu,
        texture_reg: &mut TextureRegistry,
    ) {
        // Compute LRU page id using disjoint field borrows.
        let lru_id = {
            let page_last_used = &self.page_last_used;
            self.buckets[bucket_idx]
                .pages
                .iter()
                .min_by_key(|p| *page_last_used.get(&p.id).unwrap_or(&0))
                .map(|p| p.id)
        };

        if let Some(id) = lru_id {
            let pool = &mut self.buckets[bucket_idx];
            if let Some(pos) = pool.pages.iter().position(|p| p.id == id) {
                let Page { id, mut atlas } = pool.pages.remove(pos).unwrap();
                texture_reg.destroy_atlas(gpu, &mut atlas);
                self.glyph_map.retain(|_, (_, pid)| *pid != id);
                self.page_last_used.remove(&id);

                if self.buckets[bucket_idx].current_page_id == Some(id) {
                    self.buckets[bucket_idx].current_page_id =
                        self.buckets[bucket_idx].pages.back().map(|p| p.id);
                }

                #[cfg(feature = "tracing")]
                tracing::debug!(
                    bucket = bucket_idx,
                    page_id = id,
                    "glyph atlas: recycled LRU page"
                );
            }
        }

        // Always create a fresh replacement page.
        self.alloc_page_in_bucket(bucket_idx, gpu, texture_reg);
    }
}
