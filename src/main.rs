pub mod app;
mod plot_renderer;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use cen::graphics::renderer::RenderContext;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use bytemuck::{cast_slice, Pod, Zeroable};
use cen::app::component::{Component, ComponentRegistry};
use cen::app::engine::InitContext;
use cen::app::gui::{GuiComponent, GuiHandler};
use cen::ash::vk;
use cen::ash::vk::{BufferUsageFlags, DescriptorSetLayoutBinding, DescriptorType, DeviceSize, PushConstantRange, ShaderStageFlags, WriteDescriptorSet};
use cen::egui;
use cen::egui::{Context, Slider};
use cen::gpu_allocator::MemoryLocation;
use cen::graphics::pipeline_store::{ComputePipelineConfig, PipelineKey};
use cen::graphics::renderer::RenderComponent;
use cen::vulkan::{Buffer, DescriptorSetLayout};
use egui_plot::{Line, Plot, PlotPoints};
use egui::containers::menu;
use cpal::{Stream};
use cpal::traits::StreamTrait;
use egui_code_editor::{CodeEditor, ColorTheme, Completer, Syntax};
use minmaxlttb::{LttbBuilder, LttbMethod, Point};
use ringbuf::consumer::Consumer;
use ringbuf::producer::Producer;
use ringbuf::traits::Split;
use crate::app::cpal_wrapper::StreamFactory;
use crate::plot_renderer::PlotRenderer;

const SAMPLES_PER_SECOND: usize = 44800;
const BUFFER_SAMPLES: usize = 44800;

struct AudioController {
    frequency: f32,
    volume: f32,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
}

struct AudioPlayer {
    _stream: Stream,
    producer: ringbuf::HeapProd<f32>
}

impl AudioPlayer {
    fn new() -> Self {
        let sf = StreamFactory::default_factory().unwrap();

        let (producer, mut consumer) = ringbuf::HeapRb::<f32>::new(SAMPLES_PER_SECOND * 4).split();

        let routin = Box::new(move |len: usize| -> Vec<f32> {
            let mut out = vec![0.0; len];
            consumer.pop_slice(&mut out); // returns silence if starved
            out
        });

        let stream = sf.create_stream(routin).unwrap();
        StreamTrait::play(&stream).unwrap();

        Self {
            _stream: stream,
            producer
        }
    }

}

struct App
{
    player: AudioPlayer,
    controller: AudioController,
    pipeline: Option<PipelineKey>,
    buffer: Buffer,
    code: String,
    shader_errors: Option<String>,
    compile: bool,
    file_path: Option<PathBuf>,
    play: bool,
    repeat_play: bool,
    last_play: Instant,
    plot: PlotRenderer
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct PushConstants {
    time: f32,
    samples_per_second: u32,
    total_samples: u32,
    frequency: f32,
    volume: f32,
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
        let pipeline_config = ComputePipelineConfig {
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
            ctx.pipeline_store.write(pipeline, pipeline_config)
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

        let controller = AudioController {
            volume: 1.0,
            a: 1.0,
            b: 0.0,
            c: 1.0,
            d: 1.0,
            frequency: 440.,
        };
        let player = AudioPlayer::new();

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
        let pipeline_config = ComputePipelineConfig {
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
            MemoryLocation::GpuToCpu,
            size_of::<f32>() as DeviceSize * BUFFER_SAMPLES as u64,
            BufferUsageFlags::STORAGE_BUFFER
        );

        Self {
            player,
            controller,
            pipeline,
            buffer: buffer.clone(),
            code,
            play: false,
            repeat_play: false,
            last_play: Instant::now(),
            shader_errors,
            compile: false,
            file_path: Some(default_file.into()),
            plot: PlotRenderer::new(ctx, buffer)
        }
    }
}

impl RenderComponent for App {
    fn render(&mut self, ctx: &mut RenderContext<'_>) {

        if self.compile {
            self.compile = false;
            self.update_shader(ctx);
        }

        let binding = self.buffer.mapped().unwrap();
        let gpu_data: &[f32] = cast_slice(binding.as_slice());

        if self.repeat_play {
            self.play = false;
            if Instant::now().duration_since(self.last_play).as_millis() > 1000 {
                self.player.producer.push_slice(gpu_data);
                self.last_play = Instant::now();
            }
        }

        if self.play {
            let count = self.player.producer.push_slice(gpu_data);
            println!("count {}", count);
            self.play = false;
        }

        if self.pipeline.is_none() {
            return;
        }

        // Calculate audio
        let pipeline = ctx.pipeline_store.get(self.pipeline.unwrap()).unwrap();
        ctx.command_buffer.bind_pipeline(pipeline);

        let push_constants = PushConstants {
            time: 0.0,
            samples_per_second: SAMPLES_PER_SECOND as u32,
            total_samples: BUFFER_SAMPLES as u32,
            frequency: self.controller.frequency,
            volume: self.controller.volume,
            a: self.controller.a,
            b: self.controller.b,
            c: self.controller.c,
            d: self.controller.d,
        };
        ctx.command_buffer.push_constants(
            pipeline,
            ShaderStageFlags::COMPUTE,
            0,
            &bytemuck::cast_slice(std::slice::from_ref(&push_constants))
        );

        ctx.command_buffer.push_descriptor_set(
            pipeline,
            0,
            &[
                WriteDescriptorSet::default()
                    .dst_binding(0)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&[self.buffer.binding()])
            ]
        );

        ctx.command_buffer.dispatch(BUFFER_SAMPLES as u32 / 128, 1, 1);

        // Calculate plot
        self.plot.render(ctx);
    }
}

impl GuiComponent for App {
    fn gui(&mut self, gui_handler: &mut GuiHandler, ctx: &Context) {
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

        egui::SidePanel::left("scene_tree")
            .resizable(true)
            .default_width(520.0)
            .min_width(80.0)
            .show(ctx, |ui| {
                self.plot.ui(gui_handler, ui);

                ui.horizontal(|ui| {
                    if ui.button("play").clicked() {
                        self.play = true;
                    }
                    ui.checkbox(&mut self.repeat_play, "Repeat");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("compile").clicked() {
                            self.compile = true;
                        }
                    });
                });

                ui.style_mut().spacing.slider_width = 190.;
                ui.add(Slider::new(&mut self.controller.volume, 0.0..=1.0).text("Volume"));
                ui.add(Slider::new(&mut self.controller.frequency, 0.0..=1000.0).text("Frequency"));
                ui.add(Slider::new(&mut self.controller.a, 0.0..=2.0).text("option[0]"));
                ui.add(Slider::new(&mut self.controller.b, -1.0..=1.0).text("option[1]"));
                ui.add(Slider::new(&mut self.controller.c, 0.0..=2.0).text("option[2]"));
                ui.add(Slider::new(&mut self.controller.d, 0.0..=2.0).text("option[3]"));

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
    }
}

fn main() {
    let cen_conf = cen::app::app::AppConfig::default()
        .width(1200)
        .height(800)
        .vsync(false)
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
