use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use bytes::Bytes;
use freya::elements::image::{ImageHandle, image};
use freya::engine::prelude::{Paint, SkData, SkImage, raster_n32_premul};
use freya::prelude::*;

use crate::hooks::use_player_skin;

const AVATAR_SIZE: f32 = 32.;
const FACE: f32 = 8.;
const HEAD_FACE: (f32, f32) = (8., 8.);
const HEAD_OVERLAY: (f32, f32) = (40., 8.);

static AVATAR_HEAD_CACHE: OnceLock<Mutex<HashMap<usize, ImageHandle>>> = OnceLock::new();

#[derive(PartialEq, Clone)]
pub struct Avatar {
    uuid: String,
    layout: LayoutData,
}

impl Avatar {
    pub fn new(uuid: impl Into<String>) -> Self {
        Self {
            uuid: uuid.into(),
            layout: LayoutData::default(),
        }
    }
}

impl LayoutExt for Avatar {
    fn get_layout(&mut self) -> &mut LayoutData {
        &mut self.layout
    }
}

impl ContainerSizeExt for Avatar {}

impl Component for Avatar {
    fn render_key(&self) -> DiffKey {
        DiffKey::from(&self.uuid)
    }

    fn render(&self) -> impl IntoElement {
        let (skin_bytes, _is_slim) = use_player_skin(self.uuid.clone());
        let head = get_or_compose_head(&skin_bytes);

        rect()
            .width(Size::px(AVATAR_SIZE))
            .height(Size::px(AVATAR_SIZE))
            .corner_radius(CornerRadius::from(8.))
            .center()
            .maybe_child(head.map(|holder| {
                image(holder)
                    .width(Size::fill())
                    .height(Size::fill())
                    .aspect_ratio(AspectRatio::None)
                    .sampling_mode(SamplingMode::Nearest)
                    .corner_radius(CornerRadius::from(8.))
            }))
    }
}

fn get_or_compose_head(skin_bytes: &Bytes) -> Option<ImageHandle> {
    if skin_bytes.is_empty() {
        return None;
    }
    let src_ptr = skin_bytes.as_ptr() as usize;
    let cache = AVATAR_HEAD_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(handle) = cache.lock().ok().and_then(|g| g.get(&src_ptr).cloned()) {
        return Some(handle);
    }

    let composed = compose_head(skin_bytes)?;
    if let Ok(mut guard) = cache.lock() {
        guard.insert(src_ptr, composed.clone());
    }
    Some(composed)
}

fn compose_head(skin_bytes: &Bytes) -> Option<ImageHandle> {
    let data = unsafe { SkData::new_bytes(skin_bytes) };
    let skin = SkImage::from_encoded(data)?;
    let skin = skin.make_raster_image(None, None).unwrap_or(skin);

    let mut surface = raster_n32_premul((FACE as i32, FACE as i32))?;
    {
        let canvas = surface.canvas();
        let paint = Paint::default();

        canvas.draw_image(&skin, (-HEAD_FACE.0, -HEAD_FACE.1), Some(&paint));
        canvas.draw_image(&skin, (-HEAD_OVERLAY.0, -HEAD_OVERLAY.1), Some(&paint));
    }

    let head = surface.image_snapshot();
    Some(ImageHandle::new(head, skin_bytes.clone()))
}
