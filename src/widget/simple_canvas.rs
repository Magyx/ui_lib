use super::*;
use crate::render::pipeline::PipelineKey;

pub struct SimpleCanvas<M> {
    layout: Option<Layout>,

    id: Id,
    key: &'static str,
    with_handle: Option<fn(&mut EventCtx<M>)>,
    position: Position<i32>,
    size: Size<Length<i32>>,
}

impl<M> SimpleCanvas<M> {
    pub fn new(
        size: Size<Length<i32>>,
        pipeline_key: &'static str,
        with_handle: Option<fn(&mut EventCtx<M>)>,
    ) -> Self {
        Self {
            layout: None,

            id: crate::context::next_id(),
            key: pipeline_key,
            with_handle,
            position: Position::splat(0),
            size,
        }
    }
}

impl<M> Widget<M> for SimpleCanvas<M> {
    fn id(&self) -> Id {
        self.id
    }
    fn position(&self) -> &Position<i32> {
        &self.position
    }
    fn layout(&self) -> &Layout {
        self.layout.as_ref().expect(LAYOUT_ERROR)
    }

    fn fit_width(&mut self, _ctx: &mut LayoutCtx<M>) -> Layout {
        let cur_w = match self.size.width {
            Length::Fixed(w) => w,
            _ => 0,
        };

        let l = Layout {
            size: self.size,
            current_size: Size::new(cur_w, 0),
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
        };
        self.layout = Some(l);
        l
    }

    fn grow_width(&mut self, _ctx: &mut LayoutCtx<M>, parent_width: i32) {
        let l = self.layout.as_mut().expect(LAYOUT_ERROR);

        let target_w = match self.size.width {
            Length::Grow => parent_width,
            Length::Fixed(w) => w,
            Length::Fit => l.current_size.width,
        };

        let final_w = target_w.min(parent_width);

        l.current_size.width = final_w;
        self.size.width = Length::Fixed(final_w);
    }

    fn fit_height(&mut self, _ctx: &mut LayoutCtx<M>) -> Layout {
        let cur_h = match self.size.height {
            Length::Fixed(h) => h,
            _ => 0,
        };

        let cur_w = self.layout.map(|l| l.current_size.width).unwrap_or(0);

        let l = Layout {
            size: self.size,
            current_size: Size::new(cur_w, cur_h),
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
        };
        self.layout = Some(l);
        l
    }

    fn grow_height(&mut self, _ctx: &mut LayoutCtx<M>, parent_height: i32) {
        let l = self.layout.as_mut().expect(LAYOUT_ERROR);
        let target_h = match self.size.height {
            Length::Grow => parent_height,
            Length::Fixed(h) => h,
            Length::Fit => l.current_size.height,
        };

        let final_h = target_h.min(parent_height);

        l.current_size.height = final_h;
        self.size.height = Length::Fixed(final_h);
    }

    fn place(&mut self, _ctx: &mut LayoutCtx<M>, position: Position<i32>) -> Size<i32> {
        self.position = position;
        self.layout().current_size
    }

    fn draw_self(&self, _ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
        instances.push(Instance::new(
            PipelineKey::Other(self.key),
            self.position,
            self.layout().current_size,
            [0, 0, 0, 0],
            [0, 0, 0, 0],
        ));
    }

    fn handle(&mut self, ctx: &mut EventCtx<M>) {
        if let Some(f) = self.with_handle {
            f(ctx);
        }
    }
}
