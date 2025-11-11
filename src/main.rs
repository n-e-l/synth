pub mod app;

use std::sync::{Arc, Mutex};
use cen::app::component::{Component, ComponentRegistry};
use cen::app::gui::{GuiComponent, GuiHandler};
use cen::egui;
use cen::egui::Context;
use cpal::Stream;
use cpal::traits::StreamTrait;
use log::info;
use crate::app::cpal_wrapper::StreamFactory;

struct AudioControls {
    play: bool
}

struct AudioPlayer {
    stream: Stream,
}

fn audio_shader(t:f32) -> (f32, f32) {
    let tau = 2.0 * std::f32::consts::PI;
    let n = f32::sin(tau * 440.0 * t);
    let m = n*f32::powf(1.0-t,3.0);
    let a = (f32::sin(t*tau)/2.0-0.5)*m;
    let b = (f32::sin(t*tau + tau*0.5)/2.0-0.5)*m;

    (a, b)
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
    player: AudioPlayer,
}

impl GuiComponent for App {
    fn gui(&mut self, gui: &mut GuiHandler, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Test");
        });

        // The gui isn't the correct call for this, but there's no other place right now
        self.player.play();
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
        let player = AudioPlayer::new(audio_shader);
        let app = App {
            player
        };

        ComponentRegistry::new()
            .register(Component::Gui(Arc::new(Mutex::new(app))))
    }));
}