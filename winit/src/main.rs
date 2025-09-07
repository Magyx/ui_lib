use std::sync::Arc;
use ui::{
    event::Event,
    graphics::Engine,
    model::*,
    widget::{Column, Element, Length, Rectangle, Row, Widget},
};

use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

#[derive(Clone, Debug)]
enum Message {}

struct State {}

struct App<'a> {
    window: Option<Arc<Window>>,
    engine: Option<Engine<'a, Message>>,
    state: State,
}

fn update<'a>(
    _engine: &mut Engine<'a, Message>,
    event: &Event<Message, WindowEvent>,
    _state: &mut State,
    event_loop: &ActiveEventLoop,
) -> bool {
    if let Event::Platform(window_event) = event {
        match window_event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            }
            | WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::PinchGesture { .. } => todo!(),
            _ => (),
        }
    };

    false
}

fn view(_state: &State) -> Element<Message> {
    use Length::{Fit, Fixed, Grow};

    let small_block = |r: u8, g: u8, b: u8| {
        Rectangle::new(Size::new(Fixed(24), Fixed(24)), Color::from_rgb(r, g, b)).einto()
    };

    Column::new(vec![
        /* 1) Fixed + Fixed, zero padding baseline */
        Row::new(vec![
            Rectangle::new(Size::new(Fixed(80), Fixed(40)), Color::RED).einto(),
            Rectangle::new(Size::new(Fixed(120), Fixed(40)), Color::GREEN).einto(),
        ])
        .spacing(8)
        .padding(Vec4::splat(0))
        .color(Color::from_rgb(240, 240, 240))
        .size(Size::new(Grow, Fixed(70)))
        .einto(),

        /* 2) Fixed + Grow + Fixed; height fixed, width grow (checks single-grow distribution) */
        Row::new(vec![
            Rectangle::new(Size::new(Fixed(60), Grow), Color::from_rgb(255, 200, 0)).einto(),
            Rectangle::new(Size::new(Grow, Grow), Color::from_rgb(0, 180, 180)).einto(),
            Rectangle::new(Size::new(Fixed(60), Grow), Color::from_rgb(255, 200, 0)).einto(),
        ])
        .spacing(10)
        .padding(Vec4::splat(10))
        .color(Color::from_rgb(220, 220, 240))
        .size(Size::new(Grow, Fixed(80)))
        .einto(),

        /* 3) Multiple Grow children in a Row (checks equalization logic) */
        Row::new(vec![
            Rectangle::new(Size::new(Grow, Fixed(50)), Color::from_rgb(200, 50, 50)).einto(),
            Rectangle::new(Size::new(Grow, Fixed(50)), Color::from_rgb(50, 200, 50)).einto(),
            Rectangle::new(Size::new(Grow, Fixed(50)), Color::from_rgb(50, 50, 200)).einto(),
        ])
        .spacing(6)
        .padding(Vec4::splat(10))
        .color(Color::from_rgb(240, 220, 220))
        .size(Size::new(Grow, Fixed(70)))
        .einto(),

        /* 4) Column with Grow height distribution and fixed caps at top/bottom */
        Column::new(vec![
            Rectangle::new(Size::new(Grow, Fixed(20)), Color::from_rgb(80, 80, 80)).einto(),
            Rectangle::new(Size::new(Grow, Grow), Color::from_rgb(100, 200, 100)).einto(),
            Rectangle::new(Size::new(Grow, Fixed(20)), Color::from_rgb(80, 80, 150)).einto(),
        ])
        .spacing(8)
        .padding(Vec4::splat(10))
        .color(Color::from_rgb(240, 240, 220))
        .size(Size::new(Grow, Fixed(100)))
        .einto(),

        /* 5) Fit sizing demo: Column(Fit,Fit) measured by fixed children, next to a Grow rectangle */
        Row::new(vec![
            Column::new(vec![
                Rectangle::new(Size::new(Fixed(70), Fixed(20)), Color::from_rgb(100, 0, 100))
                    .einto(),
                Rectangle::new(Size::new(Fixed(40), Fixed(30)), Color::from_rgb(140, 0, 140))
                    .einto(),
            ])
            .spacing(4)
            .padding(Vec4::splat(4))
            .size(Size::new(Fit, Fit))
            .color(Color::from_rgb(230, 200, 230))
            .einto(),
            Rectangle::new(Size::new(Grow, Fixed(60)), Color::from_rgb(180, 180, 180)).einto(),
        ])
        .spacing(10)
        .padding(Vec4::splat(10))
        .color(Color::from_rgb(220, 240, 240))
        .size(Size::new(Grow, Fixed(80)))
        .einto(),

        /* 6) Nested grow: Row of two Columns; left fixed width, right flexible */
        Row::new(vec![
            Column::new(vec![
                Rectangle::new(Size::new(Grow, Fixed(18)), Color::from_rgb(160, 160, 0)).einto(),
                Rectangle::new(Size::new(Grow, Grow), Color::from_rgb(160, 100, 0)).einto(),
            ])
            .spacing(6)
            .padding(Vec4::splat(6))
            .size(Size::new(Fixed(200), Grow))
            .color(Color::from_rgb(250, 240, 200))
            .einto(),
            Column::new(vec![
                Rectangle::new(Size::new(Grow, Grow), Color::from_rgb(0, 120, 160)).einto(),
                Rectangle::new(Size::new(Grow, Fixed(24)), Color::from_rgb(0, 80, 120)).einto(),
            ])
            .spacing(6)
            .padding(Vec4::splat(6))
            .size(Size::new(Grow, Grow))
            .color(Color::from_rgb(200, 240, 250))
            .einto(),
        ])
        .spacing(10)
        .padding(Vec4::splat(10))
        .color(Color::from_rgb(240, 230, 230))
        .size(Size::new(Grow, Fixed(100)))
        .einto(),

        /* 7) Spacing extremes: zero vs nonzero, plus a Grow filler */
        Row::new(vec![
            Row::new(vec![
                Rectangle::new(Size::new(Fixed(40), Fixed(40)), Color::from_rgb(0, 0, 0)).einto(),
                Rectangle::new(Size::new(Fixed(40), Fixed(40)), Color::from_rgb(80, 80, 80))
                    .einto(),
            ])
            .spacing(0)
            .padding(Vec4::splat(0))
            .size(Size::new(Fixed(100), Fixed(40)))
            .color(Color::from_rgb(220, 220, 220))
            .einto(),
            Row::new(vec![
                Rectangle::new(Size::new(Fixed(40), Fixed(40)), Color::from_rgb(0, 0, 0)).einto(),
                Rectangle::new(Size::new(Fixed(40), Fixed(40)), Color::from_rgb(80, 80, 80))
                    .einto(),
            ])
            .spacing(12)
            .padding(Vec4::splat(0))
            .size(Size::new(Fixed(120), Fixed(40)))
            .color(Color::from_rgb(220, 220, 220))
            .einto(),
            Rectangle::new(Size::new(Grow, Fixed(40)), Color::from_rgb(200, 200, 200)).einto(),
        ])
        .spacing(10)
        .padding(Vec4::splat(10))
        .color(Color::from_rgb(220, 220, 240))
        .size(Size::new(Grow, Fixed(60)))
        .einto(),

        /* 8) Many children + padding stress */
        Row::new((0..8).map(|i| {
            let c = (i * 30 + 40) as u8;
            small_block(c, 30, 200u8.saturating_sub(c))
        }).collect())
        .spacing(6)
        .padding(Vec4::splat(16))
        .color(Color::from_rgb(245, 245, 220))
        .size(Size::new(Grow, Fixed(56)))
        .einto(),

        /* 9) Transparent container background over content below */
        Column::new(vec![
            Rectangle::new(Size::new(Grow, Fixed(20)), Color::from_rgb(30, 200, 30)).einto(),
            Rectangle::new(Size::new(Grow, Fixed(20)), Color::from_rgb(30, 30, 200)).einto(),
        ])
        .spacing(6)
        .padding(Vec4::splat(10))
        .color(Color::TRANSPARENT)
        .size(Size::new(Grow, Fixed(60)))
        .einto(),
    ])
    .color(Color::from_rgb(100, 80, 100))
    .padding(Vec4::splat(16))
    .spacing(14)
    .size(Size::new(Grow, Grow))
    .einto()
}

impl<'a> ApplicationHandler for App<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(WindowAttributes::default().with_title("My Test GUI lib"))
                    .expect("Failed to create window"),
            );

            let engine = Engine::new(window.clone(), window.inner_size().into());

            self.engine = Some(engine);
            self.window = Some(window);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let engine = self.engine.as_mut().unwrap();

        engine.handle_event(&event, view, &mut update, &mut self.state, event_loop);
    }
}

async fn run() -> Result<(), EventLoopError> {
    env_logger::init();
    let event_loop = EventLoop::new()?;

    let mut app = App {
        window: None,
        engine: None,
        state: State {},
    };
    event_loop.run_app(&mut app)
}

fn main() {
    _ = pollster::block_on(run());
}
