pub mod app;

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use cen::app::component::{Component, ComponentRegistry};
use cen::app::gui::{GuiComponent, GuiHandler};
use cen::egui;
use cen::egui::{Context, Slider};
use egui_plot::{Line, Plot, PlotPoints};
use cpal::{Stream};
use cpal::traits::StreamTrait;
use crate::app::cpal_wrapper::StreamFactory;

struct AudioController {
    engine_start_time: SystemTime,
    play_start_time: SystemTime,
    frequency: f32,
}

impl AudioController {
    fn func(&self, mut t: f32) -> (f32, f32) {
        if t > 1.0 {
            t = 0.0;
        }

        let tau = 2.0 * std::f32::consts::PI;
        let n = f32::sin(tau * self.frequency * t);
        let m = n*f32::powf(1.0-t,3.0);
        let a = (f32::sin(t*tau)/2.0-0.5)*m;
        let b = (f32::sin(t*tau + tau*0.5)/2.0-0.5)*m;

        (a, b)
    }

    // The audio of the entire engine
    fn sample(&self, t: f32) -> (f32, f32) {
        let offset = self.play_start_time.duration_since(self.engine_start_time).unwrap_or(Duration::new(0, 0)).as_secs_f32();
        self.func(t - offset)
    }
}

struct AudioPlayer {
    stream: Stream,
}

impl AudioPlayer {
    fn new(controller: Arc<Mutex<AudioController>>) -> Self {
        let sf = StreamFactory::default_factory().unwrap();

        let sample_rate = sf.config().sample_rate.0;
        let mut sample_index: u64 = 0;
        let routin = Box::new(move |len: usize| -> Vec<f32> {

            let lock = controller.lock().unwrap();
            let data: Vec<_> = (0..len / 2) // len is apparently left *and* right
                .flat_map(|_| {
                    sample_index += 1;
                    let (l, r) = lock.sample((sample_index as f64 / sample_rate as f64) as f32);
                    vec![l, r]
                })
                .collect();

            data
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
    fn gui(&mut self, _: &mut GuiHandler, ctx: &Context) {
        let mut lock = self.controller.lock().unwrap();
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("play").clicked() {
                lock.play_start_time = SystemTime::now() + Duration::new(0, 5000000);
            }
            ui.add(Slider::new(&mut lock.frequency, 0.0..=1000.0));

            Plot::new("audio_plot")
                .view_aspect(2.0)
                .show(ui, |plot_ui| {
                    let samples_per_second = 3000;
                    let duration = 1.0;
                    let total_samples = (duration * samples_per_second as f32) as i32;
                    // Convert audio samples to plot points
                    let points = (0..total_samples)
                        .map(|i| lock.func(duration * i as f32 / total_samples as f32))
                        .enumerate()
                        .map(|(i, sample)| [i as f64 / samples_per_second as f64, sample.0 as f64])
                        .collect::<Vec<[f64; 2]>>();
                    let plot_points = PlotPoints::new(points);

                    plot_ui.line(Line::new("audio", plot_points));
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

    cen::app::Cen::run(cen_conf, Box::new(move |_| {
        let controller = Arc::new(Mutex::new(AudioController {
            engine_start_time: SystemTime::now(),
            play_start_time: SystemTime::now(),
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