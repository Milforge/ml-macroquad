use crate::{
    math::Rect,
    texture::{Image, Texture2D},
    Color,
};

use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct Sprite {
    pub rect: Rect,
    pub texture_index: usize,
}

struct AtlasPage {
    texture: Texture2D,
    image: Image,
    cursor_x: u16,
    cursor_y: u16,
    max_line_height: u16,
    dirty: bool,
}

impl AtlasPage {
    const SIZE: u16 = 1024;
    const GAP: u16 = 2;

    fn new(ctx: &mut miniquad::Context, filter: miniquad::FilterMode) -> Self {
        let image = Image::gen_image_color(Self::SIZE, Self::SIZE, Color::new(0.0, 0.0, 0.0, 0.0));
        let texture = Texture2D::from_rgba8(Self::SIZE, Self::SIZE, &image.bytes);
        texture.set_filter(filter);

        AtlasPage {
            texture,
            image,
            cursor_x: Self::GAP,
            cursor_y: Self::GAP,
            max_line_height: 0,
            dirty: false,
        }
    }

    fn try_cache_sprite(
        &mut self,
        ctx: &mut miniquad::Context,
        sprite_image: &Image,
        texture_index: usize,
        filter: miniquad::FilterMode,
    ) -> Option<Sprite> {
        let (width, height) = (sprite_image.width as usize, sprite_image.height as usize);
        if width > Self::SIZE as usize || height > Self::SIZE as usize {
            return None;
        }

        let w = width as u16;
        let h = height as u16;

        let (x, y, new_cursor_x, new_cursor_y, new_max_line_height) =
            if self.cursor_x + w <= Self::SIZE && self.cursor_y + h <= Self::SIZE {
                let x = self.cursor_x;
                let y = self.cursor_y;
                let new_cursor_x = x + w + Self::GAP;
                let new_max_line_height = self.max_line_height.max(h);
                (x, y, new_cursor_x, self.cursor_y, new_max_line_height)
            } else {
                let new_y = self.cursor_y + self.max_line_height + Self::GAP;
                if new_y + h > Self::SIZE {
                    return None;
                }
                let x = Self::GAP;
                let new_cursor_x = x + w + Self::GAP;
                (x, new_y, new_cursor_x, new_y, h)
            };

        self.cursor_x = new_cursor_x;
        self.cursor_y = new_cursor_y;
        self.max_line_height = new_max_line_height;

        self.dirty = true;
        for j in 0..height {
            for i in 0..width {
                self.image.set_pixel(
                    (x + i as u16) as u32,
                    (y + j as u16) as u32,
                    sprite_image.get_pixel(i as u32, j as u32),
                );
            }
        }

        Some(Sprite {
            rect: Rect::new(x as f32, y as f32, width as f32, height as f32),
            texture_index,
        })
    }

    fn flush(&mut self, ctx: &mut miniquad::Context) {
        if self.dirty {
            self.dirty = false;
            self.texture.update(&self.image);
        }
    }
}

pub struct Atlas {
    pages: Vec<AtlasPage>,
    pub sprites: HashMap<u64, Sprite>,
    filter: miniquad::FilterMode,
    unique_id: u64,
    initialized: bool,
}

impl Atlas {
    const UNIQUENESS_OFFSET: u64 = 100000;
    const PAGE_SIZE: u16 = 1024;

    pub fn new(ctx: &mut miniquad::Context, filter: miniquad::FilterMode) -> Self {
        Atlas {
            pages: Vec::new(),
            sprites: HashMap::new(),
            filter,
            unique_id: Self::UNIQUENESS_OFFSET,
            initialized: false
        }
    }

    fn ensure_initialized(&mut self, ctx: &mut miniquad::Context) {
        if !self.initialized {
            let first_page = AtlasPage::new(ctx, self.filter);
            self.pages.push(first_page);
            self.initialized = true;
        }
    }

    pub fn new_unique_id(&mut self) -> u64 {
        self.unique_id += 1;
        self.unique_id
    }

    pub fn get(&self, key: u64) -> Option<Sprite> {
        self.sprites.get(&key).cloned()
    }

    pub fn width(&self) -> u16 {
        Self::PAGE_SIZE
    }

    pub fn height(&self) -> u16 {
        Self::PAGE_SIZE
    }

    pub fn texture(&mut self, ctx: &mut miniquad::Context, key: u64) -> Option<Texture2D> {
        self.ensure_initialized(ctx);
        let sprite = self.sprites.get(&key)?;
        let idx = sprite.texture_index;
        if idx >= self.pages.len() {
            return None;
        }
        self.pages[idx].flush(ctx);
        Some(self.pages[idx].texture.clone())
    }

    pub fn get_uv_rect(&self, key: u64) -> Option<Rect> {
        let sprite = self.get(key)?;
        let page_size = Self::PAGE_SIZE as f32;
        Some(Rect::new(
            sprite.rect.x / page_size,
            sprite.rect.y / page_size,
            sprite.rect.w / page_size,
            sprite.rect.h / page_size,
        ))
    }

    pub fn cache_sprite(&mut self, ctx: &mut miniquad::Context, key: u64, sprite: Image) {
        self.ensure_initialized(ctx);
        if sprite.width > Self::PAGE_SIZE || sprite.height > Self::PAGE_SIZE {
            return;
        }

        for (idx, page) in self.pages.iter_mut().enumerate() {
            if let Some(sprite_meta) = page.try_cache_sprite(ctx, &sprite, idx, self.filter) {
                self.sprites.insert(key, sprite_meta);
                return;
            }
        }

        let mut new_page = AtlasPage::new(ctx, self.filter);
        let new_idx = self.pages.len();
        if let Some(sprite_meta) = new_page.try_cache_sprite(ctx, &sprite, new_idx, self.filter) {
            self.pages.push(new_page);
            self.sprites.insert(key, sprite_meta);
        }
    }

    // pub fn flush_all(&mut self, ctx: &mut miniquad::Context) {
    //     for page in &mut self.pages {
    //         page.flush(ctx);
    //     }
    // }
}
