pub mod app;
mod plot_renderer;

use std::collections::{HashMap};
use std::fs;
use std::path::PathBuf;
use std::time::{Instant};
use bytemuck::{cast_slice, Pod, Zeroable};
use cen::app::app::{AppComponent, AppConfig};
use cen::app::Cen;
use cen::app::engine::CenContext;
use cen::app::gui::{GuiComponent, GuiContext};
use cen::ash::vk;
use cen::ash::vk::{BufferUsageFlags, DescriptorSetLayoutBinding, DescriptorType, DeviceSize, PushConstantRange, ShaderStageFlags, WriteDescriptorSet};
use cen::egui;
use cen::egui::{Context};
use cen::gpu_allocator::MemoryLocation;
use cen::graphics::pipeline_store::{PipelineKey};
use cen::graphics::renderer::RenderComponent;
use cen::vulkan::{Buffer, ComputePipelineConfig, DescriptorSetLayout, Device, SlangModule};
use cen::winit::event::{ElementState, KeyEvent, WindowEvent};
use cen::winit::keyboard::{Key, ModifiersState, NamedKey};
use egui::containers::menu;
use cpal::{Stream};
use cpal::traits::StreamTrait;
use egui_code_editor::{CodeEditor, ColorTheme, Completer, Syntax};
use ringbuf::consumer::Consumer;
use ringbuf::producer::Producer;
use ringbuf::traits::Split;
use crate::app::cpal_wrapper::StreamFactory;
use crate::app::knob::Knob;
use crate::app::syntax::{glsl_syntax, slang_syntax};
use crate::plot_renderer::PlotRenderer;

const SAMPLES_PER_SECOND: usize = 44800;
const BUFFER_DURATION: f32 = 1f32;
const BUFFER_SAMPLES: usize = (SAMPLES_PER_SECOND as f32 * BUFFER_DURATION) as usize;
const AUDIO_BUFFER_SIZE: usize = 1024 * 4;

struct AudioControls {
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

        let (producer, mut consumer) = ringbuf::HeapRb::<f32>::new(AUDIO_BUFFER_SIZE).split();

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
    controls: AudioControls,
    pipeline: Option<PipelineKey>,
    buffer: Buffer,
    code: String,
    shader_errors: Option<String>,
    compile: bool,
    file_path: Option<PathBuf>,
    play: bool,
    repeat_play: bool,
    played_offset: Option<usize>,
    start_time: Option<Instant>,
    plot: PlotRenderer,
    modifiers: ModifiersState,
    last_compile: Option<Instant>,
    syntax: Syntax,
    completer: Completer
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct PushConstants {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    time: f32,
    samples_per_second: u32,
    total_samples: u32,
    frequency: f32,
    volume: f32,
}

impl AppComponent for App {

    fn new(ctx: &mut CenContext) -> Self {

        let controls = AudioControls {
            volume: 1.0,
            a: 0.02,
            b: 0.5,
            c: 1.0,
            d: 1.0,
            frequency: 0.15,
        };

        let player = AudioPlayer::new();

        let mut shader_errors = None;

        let default_file = "audio/sine.slang";
        let code = fs::read_to_string(default_file).expect("Failed to read audio file");

        let descriptor_set_layout = Self::descriptor_set_layout(&ctx.gfx.device);
        let pipeline = ctx.create_pipeline(Self::pipeline_config(descriptor_set_layout, "shaders/audio.slang", code.clone()))
            .map_err(|e| shader_errors = Some(e.to_string()))
            .ok();

        let buffer = Buffer::new(
            &ctx.gfx.device,
            &mut ctx.gfx.allocator,
            MemoryLocation::GpuToCpu,
            size_of::<f32>() as DeviceSize * 2 * BUFFER_SAMPLES as u64,
            BufferUsageFlags::STORAGE_BUFFER
        );

        let syntax = if default_file.ends_with("slang") {
            slang_syntax()
        } else {
            glsl_syntax()
        };
        let completer = Completer::new_with_syntax(&syntax).with_user_words();

        Self {
            player,
            controls,
            pipeline,
            buffer: buffer.clone(),
            code,
            play: false,
            repeat_play: false,
            modifiers: Default::default(),
            played_offset: None,
            start_time: None,
            shader_errors,
            compile: true,
            file_path: Some(default_file.into()),
            plot: PlotRenderer::new(ctx, buffer),
            last_compile: None,
            syntax,
            completer
        }
    }

    fn window_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput {
                event:
                KeyEvent {
                    logical_key: key,
                    state: ElementState::Pressed,
                    ..
                },
                ..
            } => match key.as_ref() {
                Key::Named(NamedKey::Enter) if self.modifiers.control_key() => {
                    self.compile = true;
                },
                _ => {}
            },
            _ => {}
        }
    }
}

impl App {

    fn descriptor_set_layout(device: &Device) -> DescriptorSetLayout {
        DescriptorSetLayout::new_push_descriptor(
            device,
            &[
                DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(ShaderStageFlags::COMPUTE)
            ]
        )
    }

    fn pipeline_config(descriptor_set_layout: DescriptorSetLayout, file_name: &str, code: String) -> ComputePipelineConfig {

        let shader_source = if file_name.ends_with("glsl") {
            PathBuf::from("shaders/audio.comp")
        } else {
            PathBuf::from("shaders/audio.slang")
        };

        let mut macros = HashMap::new();
        macros.insert("audio_function".to_string(), code.clone());
        ComputePipelineConfig {
            shader_source,
            descriptor_set_layouts: vec![ descriptor_set_layout ],
            push_constant_ranges: vec![
                PushConstantRange::default()
                    .stage_flags(ShaderStageFlags::COMPUTE)
                    .offset(0)
                    .size(size_of::<PushConstants>() as u32)
            ],
            macros,
            slang_modules: vec![SlangModule {
                name: "user_audio".to_string(),
                source: code
            }],
            ..Default::default()
        }
    }

    fn load_file(&mut self, path: PathBuf) {
        self.syntax = if path.to_str().unwrap().ends_with("slang") {
            slang_syntax()
        } else {
            glsl_syntax()
        };
        self.completer = Completer::new_with_syntax(&self.syntax).with_user_words();

        self.file_path = Some(path.clone());
        self.code = fs::read_to_string(path.clone()).expect("Failed to read audio file");
        self.compile = true;
    }

    fn update_shader(&mut self, ctx: &mut CenContext) {
        let descriptor_set_layout = Self::descriptor_set_layout(&ctx.gfx.device);
        let pipeline_config = Self::pipeline_config(descriptor_set_layout, self.file_path.as_ref().unwrap().to_str().unwrap(), self.code.clone());

        let pipeline = if let Some(key) = self.pipeline {
            // TODO: Make pipeline keys refcounted
            ctx.pipelines.pipeline_store.write(key, pipeline_config)
        } else {
            ctx.pipelines.create_pipeline(pipeline_config)
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

        self.last_compile = Some(Instant::now())
    }
}

impl RenderComponent for App {
    fn render(&mut self, ctx: &mut CenContext<'_>) {

        if self.compile {
            self.compile = false;
            self.update_shader(ctx);
        }

        // If we're already playing
        if self.repeat_play && self.played_offset.is_none() {
            self.play = true;
        }

        if let Some(start_time) = &self.start_time {
            if Instant::now().duration_since(*start_time).as_millis() as f32 / 1000f32 >= BUFFER_DURATION {
                self.played_offset = None;
                self.start_time = None;

                if self.repeat_play {
                    self.play = true;
                }
            }
        }

        if self.play {
            self.played_offset = Some(0);
            self.start_time = Some(Instant::now());
            self.play = false;
        }

        if let Some(played_offset) = &mut self.played_offset {
            // The played offset is x2 as it contains both channels
            if *played_offset < BUFFER_SAMPLES * 2 {
                let binding = self.buffer.mapped().unwrap();
                let gpu_data: &[f32] = cast_slice(binding.as_slice());
                // for i in 0..400 {
                //     info!("{}", gpu_data[i * 2]);
                // }
                // info!("break");
                let remaining = &gpu_data[*played_offset..];
                let count = self.player.producer.push_slice(remaining);
                *played_offset += count;
            }
        }

        if self.pipeline.is_none() {
            return;
        }

        // Calculate audio
        let pipeline = ctx.pipelines.pipeline_store.get(self.pipeline.unwrap()).unwrap();
        ctx.command_buffer.bind_pipeline(pipeline);

        let push_constants = PushConstants {
            time: 0.0,
            samples_per_second: SAMPLES_PER_SECOND as u32,
            total_samples: BUFFER_SAMPLES as u32,
            frequency: self.controls.frequency,
            volume: self.controls.volume,
            a: self.controls.a,
            b: self.controls.b,
            c: self.controls.c,
            d: self.controls.d,
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
    fn gui(&mut self, gui_handler: &mut GuiContext, ctx: &Context) {
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

        let time_offset = if let Some(time) = self.start_time {
            Instant::now().duration_since(time).as_millis() as usize * (SAMPLES_PER_SECOND / 1000)
        } else {
            0
        };

        egui::SidePanel::left("scene_tree")
            .resizable(true)
            .default_width(520.0)
            .min_width(80.0)
            .show(ctx, |ui| {
                self.plot.ui(gui_handler, ui, time_offset);

                ui.horizontal(|ui| {
                    if ui.button("play").clicked() {
                        self.play = true;
                    }
                    ui.checkbox(&mut self.repeat_play, "Repeat");

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(t) = self.last_compile {
                            let elapsed = t.elapsed().as_secs_f32();
                            const FADE_TIME: f32 = 0.5;
                            if elapsed < FADE_TIME {
                                let default = ui.visuals().widgets.inactive.weak_bg_fill;
                                let orange = egui::Color32::from_rgb(241, 121, 25);
                                let t = (FADE_TIME - elapsed) / FADE_TIME;

                                let lerp = |a: u8, b: u8| -> u8 { (a as f32 + (b as f32 - a as f32) * t) as u8 };
                                let color = egui::Color32::from_rgb(
                                    lerp(default.r(), orange.r()),
                                    lerp(default.g(), orange.g()),
                                    lerp(default.b(), orange.b()),
                                );
                                ui.visuals_mut().widgets.inactive.weak_bg_fill = color;
                                ui.visuals_mut().widgets.hovered.weak_bg_fill = color;
                            }
                        }

                        if ui.button("compile").clicked() {
                            self.compile = true;
                        }
                    });
                });

                ui.horizontal(|ui| {
                    let knob_size = (20.0 + 2.0) * 2.0; // (radius + padding) * 2
                    let n = 6.0f32;
                    let total = knob_size * n + ui.spacing().item_spacing.x * (n - 1.0);
                    let offset = (ui.available_width() - total) / 2.0;
                    if offset > 0.0 { ui.add_space(offset); }
                    ui.add(Knob::new(&mut self.controls.volume, 0f32..=1f32).text("Volume"));
                    ui.add(Knob::new(&mut self.controls.frequency, 0f32..=1f32).text("Freq"));
                    ui.add(Knob::new(&mut self.controls.a, 0f32..=1f32).text("opt[0]"));
                    ui.add(Knob::new(&mut self.controls.b, 0f32..=1f32).text("opt[1]"));
                    ui.add(Knob::new(&mut self.controls.c, 0f32..=1f32).text("opt[2]"));
                    ui.add(Knob::new(&mut self.controls.d, 0f32..=1f32).text("opt[3]"));
                });

            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let error_height = 200.0;
            let available = ui.available_height();

            egui::ScrollArea::vertical()
                .max_height(available - error_height)
                .show(ui, |ui| {
                    CodeEditor::default()
                        .id_source("code editor")
                        .with_rows(12)
                        .with_fontsize(14.0)
                        .with_theme(ColorTheme::GITHUB_DARK)
                        .with_syntax(self.syntax.clone())
                        .with_numlines(true)
                        .show_with_completer(ui, &mut self.code, &mut self.completer);
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
    Cen::<App>::run(
        AppConfig::default()
            .width(1200)
            .height(800)
            .vsync(false)
            .fullscreen(false)
            .resizable(true)
            .log_fps(true)
    )
}
