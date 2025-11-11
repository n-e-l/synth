pub mod app;

use std::sync::{Arc, Mutex};
use cen::app::component::{Component, ComponentRegistry};
use cen::app::gui::{GuiComponent, GuiHandler};
use cen::egui;
use cen::egui::{Context, Slider};
use egui_plot::{Line, Plot, PlotPoints};
use cpal::Stream;
use cpal::traits::StreamTrait;
use log::info;
use crate::app::cpal_wrapper::StreamFactory;

struct AudioController {
    play: bool,
    frequency: f32
}

impl AudioController {
    fn func(&self, t: f32) -> (f32, f32) {
        if !self.play {
            return (0.0, 0.0);
        }

        let tau = 2.0 * std::f32::consts::PI;
        let n = f32::sin(tau * self.frequency * t);
        let m = n*f32::powf(1.0-t,3.0);
        let a = (f32::sin(t*tau)/2.0-0.5)*m;
        let b = (f32::sin(t*tau + tau*0.5)/2.0-0.5)*m;

        (a, b)
    }
}

struct AudioPlayer {
    stream: Stream,
}

impl AudioPlayer {
    fn new(controller: Arc<Mutex<AudioController>>) -> Self {
        let sf = StreamFactory::default_factory().unwrap();

        let sample_rate = sf.config().sample_rate.0;
        let mut sample_clock = 0;
        let routin = Box::new(move |len: usize| -> Vec<f32> {
            (0..len / 2) // len is apparently left *and* right
                .flat_map(|_| {
                    sample_clock = (sample_clock + 1) % sample_rate;
                    let (l, r) = controller.lock().unwrap().func(sample_clock as f32 / sample_rate as f32);
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
    controller: Arc<Mutex<AudioController>>,
}

impl GuiComponent for App {
    fn gui(&mut self, gui: &mut GuiHandler, ctx: &Context) {
        let mut lock = self.controller.lock().unwrap();
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Test");
            ui.checkbox(&mut lock.play, "play");
            ui.add(Slider::new(&mut lock.frequency, 0.0..=1000.0));

            Plot::new("audio_plot")
                .view_aspect(2.0)
                .show(ui, |plot_ui| {
                    // Convert audio samples to plot points
                    let points = (0..1000)
                        .map(|i| lock.func(i as f32 / 1000.0))
                        .enumerate()
                        .map(|(i, sample)| [i as f64, sample.0 as f64])
                        .collect::<Vec<[f64; 2]>>();
                    let plot_points = PlotPoints::new(points);

                    plot_ui.line(Line::new("func", plot_points));
                });
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
        let controller = Arc::new(Mutex::new(AudioController {
            play: true,
            frequency: 440.
        }));
        let player = AudioPlayer::new(controller.clone());
        let app = App {
            player,
            controller
        };

        ComponentRegistry::new()
            .register(Component::Gui(Arc::new(Mutex::new(app))))
    }));
}