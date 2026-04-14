pub mod app;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use cen::graphics::renderer::RenderContext;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use bytemuck::{cast_slice, Pod, Zeroable};
use cen::app::component::{Component, ComponentRegistry};
use cen::app::engine::InitContext;
use cen::app::gui::{GuiComponent, GuiHandler};
use cen::ash::vk;
use cen::ash::vk::{BufferUsageFlags, DescriptorSetLayoutBinding, DescriptorType, DeviceSize, PushConstantRange, ShaderStageFlags, WriteDescriptorSet};
use cen::egui;
use cen::egui::{Context, Slider};
use cen::gpu_allocator::MemoryLocation;
use cen::graphics::pipeline_store::{PipelineConfig, PipelineKey};
use cen::graphics::renderer::RenderComponent;
use cen::vulkan::{Buffer, DescriptorSetLayout};
use egui_plot::{Line, Plot, PlotPoints};
use egui::containers::menu;
use cpal::{Stream};
use cpal::traits::StreamTrait;
use egui_code_editor::{CodeEditor, ColorTheme, Completer, Syntax};
use crate::app::cpal_wrapper::StreamFactory;

// Pretty low amount as I didn't optimize
const BUFFER_SAMPLES: usize = 128 * 128;

struct AudioPacket {
    data: [f32; BUFFER_SAMPLES]
}

struct AudioController {
    engine_start_time: SystemTime,
    play_start_time: SystemTime,
    frequency: f32,
    volume: f32,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    audio: Vec<AudioPacket>,
}

impl AudioController {
    fn get_sample(&self, i: u64) -> f32 {
        let packet = i as usize / BUFFER_SAMPLES;
        if packet >= self.audio.len() {
            return 0.;
            // panic!("Not enough packets! packet: {}, available: {}", packet, self.audio.len());
        }

        let index = i as usize % BUFFER_SAMPLES;
        self.audio[packet].data[index]
    }

    // The audio of the entire engine
    fn sample(&self, t: f32) -> (f32, f32) {
        let offset = self.play_start_time.duration_since(self.engine_start_time).unwrap_or(Duration::new(0, 0)).as_secs_f32();
        let i = ((t - offset) * BUFFER_SAMPLES as f32) as u64;
        let audio = self.get_sample(i);
        (audio * self.volume, audio * self.volume)
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
    pipeline: Option<PipelineKey>,
    buffer: Buffer,
    frame_index: u32,
    code: String,
    shader_errors: Option<String>,
    compile: bool,
    file_path: Option<PathBuf>
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct PushConstants {
    time: f32,
    samples: u32,
    frequency: f32,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
}

impl App {

    fn load_file(&mut self, path: PathBuf) {
        self.file_path = Some(path.clone());
        self.code = fs::read_to_string(path).expect("Failed to read audio file");
        self.compile = true;
    }

    fn update_shader(&mut self, ctx: &mut RenderContext) {
        let descriptor_set_layout = DescriptorSetLayout::new_push_descriptor(
            ctx.device,
            &[
                DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(ShaderStageFlags::COMPUTE)
            ]
        );
        let mut macros = HashMap::new();
        macros.insert("audio_function".to_string(), self.code.clone());
        let pipeline_config = PipelineConfig {
            shader_path: PathBuf::from("shaders/audio.comp"),
            descriptor_set_layouts: vec![ descriptor_set_layout ],
            push_constant_ranges: vec![
                PushConstantRange::default()
                    .stage_flags(ShaderStageFlags::COMPUTE)
                    .offset(0)
                    .size(size_of::<PushConstants>() as u32)
            ],
            macros,
        };

        let pipeline = if let Some(pipeline) = self.pipeline {
            ctx.pipeline_store.update(pipeline, pipeline_config)
        } else {
            ctx.pipeline_store.insert(pipeline_config)
        };
        match pipeline {
            Ok(key) => {
                self.pipeline = Some( key );
                self.shader_errors = None;
            }
            Err(e) => {
                self.shader_errors = Some( e.to_string() )
            }
        }
    }

    fn new(ctx: &mut InitContext) -> Self {
        let default_file = "audio/kick.glsl";
        let code = fs::read_to_string(default_file).expect("Failed to read audio file");

        let controller = Arc::new(Mutex::new(AudioController {
            volume: 1.0,
            a: 1.0,
            b: 0.0,
            c: 1.0,
            d: 1.0,
            engine_start_time: SystemTime::now(),
            play_start_time: SystemTime::now(),
            frequency: 440.,
            audio: vec![],
        }));
        let player = AudioPlayer::new(controller.clone());

        let descriptor_set_layout = DescriptorSetLayout::new_push_descriptor(
            ctx.device,
            &[
                DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(ShaderStageFlags::COMPUTE)
            ]
        );
        let mut macros = HashMap::new();
        macros.insert("audio_function".to_string(), code.clone());
        let pipeline_config = PipelineConfig {
            shader_path: PathBuf::from("shaders/audio.comp"),
            descriptor_set_layouts: vec![ descriptor_set_layout ],
            push_constant_ranges: vec![
                PushConstantRange::default()
                    .stage_flags(ShaderStageFlags::COMPUTE)
                    .offset(0)
                    .size(size_of::<PushConstants>() as u32)
            ],
            macros,
        };

        let mut shader_errors = None;
        let pipeline = match ctx.pipeline_store.insert(pipeline_config) {
            Ok(p) => { Some(p) }
            Err(e) => {
                shader_errors = Some( e.to_string() );
                None
            }
        };

        let buffer = Buffer::new(
            ctx.device,
            ctx.allocator,
            MemoryLocation::CpuToGpu,
            size_of::<f32>() as DeviceSize * BUFFER_SAMPLES as u64,
            BufferUsageFlags::STORAGE_BUFFER
        );

        Self {
            player,
            controller,
            pipeline,
            buffer,
            frame_index: 0,
            code,
            shader_errors,
            compile: false,
            file_path: Some(default_file.into())
        }
    }
}

impl RenderComponent for App {
    fn render(&mut self, ctx: &mut RenderContext<'_>) {

        if self.compile {
            self.compile = false;
            self.update_shader(ctx);
        }

        let mut lock = self.controller.lock().unwrap();

        if self.frame_index >= 1 {
            let binding = self.buffer.mapped().unwrap();
            let gpu_data: &[f32] = cast_slice(binding.as_slice());
            let packet = AudioPacket {
                data: gpu_data.try_into().unwrap()
            };
            lock.audio.clear();
            lock.audio.push(packet);
        }

        if self.pipeline.is_none() {
            return;
        }

        let pipeline = ctx.pipeline_store.get(self.pipeline.unwrap()).unwrap();
        ctx.command_buffer.bind_pipeline(&pipeline);

        let push_constants = PushConstants {
            time: 0.0,
            samples: BUFFER_SAMPLES as u32,
            frequency: lock.frequency,
            a: lock.a,
            b: lock.b,
            c: lock.c,
            d: lock.d,
        };
        ctx.command_buffer.push_constants(
            &pipeline,
            ShaderStageFlags::COMPUTE,
            0,
            &bytemuck::cast_slice(std::slice::from_ref(&push_constants))
        );

        let bindings = [vk::DescriptorBufferInfo::default()
            .buffer(*self.buffer.handle())
            .offset(0)
            .range(self.buffer.size())
        ];

        let write_descriptor_set = WriteDescriptorSet::default()
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&bindings);

        ctx.command_buffer.push_descriptor_set(
            &pipeline,
            0,
            &[write_descriptor_set]
        );

        ctx.command_buffer.dispatch(BUFFER_SAMPLES as u32 / 128, 1, 1);

        self.frame_index += 1;
    }
}

impl GuiComponent for App {
    fn gui(&mut self, _: &mut GuiHandler, ctx: &Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            menu::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_directory(std::env::current_dir().unwrap())
                            .pick_file() {
                            self.load_file(path);
                        }
                    }
                    if ui.button("Save").clicked() {
                        if let Some(path) = &self.file_path {
                            fs::write(path, &self.code).unwrap();
                        }
                    }
                });
                let label: String = if let Some(path) = &self.file_path {
                    path.to_str().unwrap().to_string()
                } else { "Example".to_string() };
                ui.label(label);
            });
        });

        let mut lock = self.controller.lock().unwrap();
        egui::SidePanel::left("scene_tree")
            .resizable(true)
            .default_width(520.0)
            .min_width(80.0)
            .show(ctx, |ui| {
                if let Some(audio) = lock.audio.first() {
                    Plot::new("audio_plot")
                        .view_aspect(2.0)
                        .show(ui, |plot_ui| {
                            let total_samples = BUFFER_SAMPLES;
                            // Convert audio samples to plot points
                            let points = (0..total_samples)
                                .map(|i| audio.data[i])
                                .enumerate()
                                .map(|(i, sample)| [i as f64, sample as f64])
                                .collect::<Vec<[f64; 2]>>();
                            let plot_points = PlotPoints::new(points);

                            plot_ui.line(Line::new("audio", plot_points));
                        });
                }

                ui.horizontal(|ui| {
                    if ui.button("play").clicked() {
                        lock.play_start_time = SystemTime::now() + Duration::new(0, 5000000);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("compile").clicked() {
                            self.compile = true;
                        }
                    });
                });

                ui.style_mut().spacing.slider_width = 190.;
                ui.add(Slider::new(&mut lock.volume, 0.0..=1.0).text("Volume"));
                ui.add(Slider::new(&mut lock.frequency, 0.0..=1000.0).text("Frequency"));
                ui.add(Slider::new(&mut lock.a, 0.0..=2.0).text("a"));
                ui.add(Slider::new(&mut lock.b, -1.0..=1.0).text("b"));
                ui.add(Slider::new(&mut lock.c, 0.0..=2.0).text("c"));
                ui.add(Slider::new(&mut lock.d, 0.0..=2.0).text("d"));

            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let error_height = 200.0;
            let available = ui.available_height();

            egui::ScrollArea::vertical()
                .max_height(available - error_height)
                .show(ui, |ui| {
                    let syntax = Syntax::shell();
                    let mut completer = Completer::new_with_syntax(&syntax).with_user_words();
                    CodeEditor::default()
                        .id_source("code editor")
                        .with_rows(12)
                        .with_fontsize(14.0)
                        .with_theme(ColorTheme::GRUVBOX)
                        .with_syntax(syntax.clone())
                        .with_numlines(true)
                        .show_with_completer(ui, &mut self.code, &mut completer);
                });

            ui.add(
                egui::TextEdit::multiline(&mut self.shader_errors.clone().unwrap_or("".to_string()))
                    .font(egui::TextStyle::Monospace)
                    .code_editor()
                    .desired_rows(10)
                    .lock_focus(true)
                    .desired_width(f32::INFINITY)
            );
        });

        // The gui isn't the correct call for this, but there's no other place right now
        self.player.play();
    }
}

fn main() {
    let cen_conf = cen::app::app::AppConfig::default()
        .width(1200)
        .height(800)
        .vsync(true)
        .fullscreen(false)
        .resizable(true)
        .log_fps(true);

    cen::app::Cen::run(cen_conf, Box::new(move |ctx| {
        let app = Arc::new(Mutex::new(App::new(ctx)));
        ComponentRegistry::new()
            .register(Component::Gui(app.clone()))
            .register(Component::Render(app))
    }));
}