pub mod app;

use std::sync::{Arc, Mutex};
use cen::app::component::{Component, ComponentRegistry};
use cen::app::gui::{GuiComponent, GuiHandler};
use cen::egui;
use cen::egui::Context;
use cpal::Stream;
use cpal::traits::StreamTrait;
use crate::app::cpal_wrapper::StreamFactory;

struct AudioPlayer {
    stream: Stream,
}

impl AudioPlayer {
    fn new(func: fn(f32)->(f32, f32)) -> Self {
        let sf = StreamFactory::default_factory().unwrap();

        let sample_rate = sf.config().sample_rate.0;
        let mut sample_clock = 0;
        let routin = Box::new(move |len: usize| -> Vec<f32> {
            (0..len / 2) // len is apparently left *and* right
                .flat_map(|_| {
                    sample_clock = (sample_clock + 1) % sample_rate;

                    let (l, r) = func(sample_clock as f32 / sample_rate as f32);
                    vec![l, r]
                })
                .collect()
        });

        Self {
            stream: sf.create_stream(routin).unwrap() // creates stream from function "routin"
        }
    }

    fn play(&self) {
        StreamTrait::play(&self.stream).unwrap();
    }
}

struct App
{

}

impl GuiComponent for App {
    fn gui(&mut self, gui: &mut GuiHandler, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Test");
        });

    }
}

fn main() {
    let cen_conf = cen::app::app::AppConfig::default()
        .width(1000)
        .height(1000)
        .vsync(true)
        .fullscreen(false)
        .resizable(true)
        .log_fps(false);

    cen::app::Cen::run(cen_conf, Box::new(move |ctx| {
        // audio program (not synced to render time like audio file?)
        // let player = AudioPlayer::new(program);
        // let player = None;
        // if let Some(p) = &player {
        //     p.play();
        // }
        let app = App {};

        ComponentRegistry::new()
            .register(Component::Gui(Arc::new(Mutex::new(app))))
    }));
}