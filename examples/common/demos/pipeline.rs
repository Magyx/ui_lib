use std::sync::{Arc, OnceLock};

use ui::render::pipeline::mesh::{Mesh, math::Camera, math::Mat4};

use super::super::pipeline::PlanetPipeline;
use super::*;

static CUBE: OnceLock<Arc<Mesh>> = OnceLock::new();
static SPHERE: OnceLock<Arc<Mesh>> = OnceLock::new();
static TORUS: OnceLock<Arc<Mesh>> = OnceLock::new();

pub fn view(tid: &TargetId, state: &State) -> Element {
    use Length::{Fill, Fit};

    let cube = CUBE.get_or_init(|| Arc::new(Mesh::cube()));
    let sphere = SPHERE.get_or_init(|| Arc::new(Mesh::uv_sphere(0.33, 64, 32)));
    let torus = TORUS.get_or_init(|| Arc::new(Mesh::torus(0.33, 0.25, 64, 32)));

    let target = match state.per_target.get(tid) {
        Some(t) => t,
        None => return Rectangle::placeholder().into(),
    };
    let t = &state.theme;

    Stack::new(el![
        Row::new(el![
            SimpleCanvas::<PlanetPipeline>::new(Size::splat(Fill(1.0)),).with_handle(|cx| {
                cx.ui.request_redraw();
            },),
            MeshCanvas::new(Size::splat(Fill(1.0)))
                .push(
                    MeshItem::shared("sphere", sphere.clone())
                        .model(Mat4::translation([0.0, 0.66, 0.0]))
                )
                .push(MeshItem::shared("cube", cube.clone()).model(Mat4::uniform_scale(0.66)))
                .push(
                    MeshItem::shared("torus", torus.clone())
                        .model(Mat4::translation([0.0, -0.66, 0.0]))
                        .tint([0.9, 0.5, 0.3, 1.0])
                )
                .camera(Camera::default())
                .spin(0.6)
        ])
        .size(Size::splat(Fill(1.0))),
        Text::h3(format!(
            "{:.0}",
            target.fps.iter().sum::<f32>() / target.fps.len().max(1) as f32
        ))
        .size(Size::splat(Fit))
        .color(t.error)
        .offset(-space::SM, space::SM)
        .pinned(Align2::TOP_RIGHT),
    ])
    .color(t.surface)
    .padding(Vec4::splat(0))
    .size(Size::splat(Fill(1.0)))
    .into()
}
