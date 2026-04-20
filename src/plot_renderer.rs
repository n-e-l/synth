use std::collections::HashMap;
use std::path::PathBuf;
use bytemuck::{Pod, Zeroable};
use cen::app::engine::InitContext;
use cen::app::gui::GuiHandler;
use cen::app::Texture;
use cen::ash::vk;
use cen::ash::vk::{AccessFlags, AttachmentLoadOp, AttachmentStoreOp, BufferUsageFlags, ClearColorValue, ClearValue, DescriptorSetLayoutBinding, DescriptorType, DeviceSize, Extent2D, Extent3D, Filter, Format, ImageLayout, ImageUsageFlags, Offset2D, PipelineStageFlags, PushConstantRange, Rect2D, RenderingAttachmentInfo, ResolveModeFlags, SampleCountFlags, ShaderStageFlags, Viewport, WriteDescriptorSet};
use cen::egui;
use cen::egui::{Color32, Pos2, Rect, Sense, Stroke, Ui};
use cen::gpu_allocator::MemoryLocation;
use cen::graphics::pipeline_store::{PipelineKey};
use cen::graphics::renderer::{RenderComponent, RenderContext};
use cen::vulkan::{Allocator, Buffer, ComputePipelineConfig, DescriptorSetLayout, Device, GraphicsPipelineConfig, Image, ImageConfig, ImageTrait};
use crate::{BUFFER_DURATION, BUFFER_SAMPLES, SAMPLES_PER_SECOND};

pub struct PlotRenderer {
    zoom: f32,
    sample_offset: f32,
    ms_image: Image,
    image: Image,
    audio_buffer: Buffer,
    minmax_buffer: Buffer,
    minmax_pipeline: PipelineKey,
    texture: Option<Texture>,
    graph_pipeline: PipelineKey,
    background_pipeline: PipelineKey,
    current_sample: usize
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct PushConstants {
    samples_per_seconds: u32,
    total_samples: u32,
    pixels_x: u32,
    pixels_y: u32,
    zoom: f32,
    offset: u32,
    current_sample: usize
}

impl PlotRenderer {
    pub fn new(ctx: &mut InitContext, audio_buffer: Buffer) -> Self {

        // Image
        let width = 100;
        let height = 100;
        let image = Image::new(
            ctx.device,
            ctx.allocator,
            ImageConfig {
                extent: Extent3D {
                    width,
                    height,
                    depth: 1
                },
                samples: SampleCountFlags::TYPE_1,
                filter: Filter::LINEAR,
                image_usage_flags: ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::COLOR_ATTACHMENT | ImageUsageFlags::SAMPLED,
                ..Default::default()
            }
        );

        let ms_image = Image::new(
            ctx.device,
            ctx.allocator,
            ImageConfig {
                extent: Extent3D {
                    width,
                    height,
                    depth: 1
                },
                samples: SampleCountFlags::TYPE_4,
                filter: Filter::LINEAR,
                image_usage_flags: ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::COLOR_ATTACHMENT | ImageUsageFlags::SAMPLED,
                ..Default::default()
            }
        );

        // Buffer
        let minmax_buffer = Buffer::new(
            ctx.device,
            ctx.allocator,
            MemoryLocation::GpuOnly,
            (width as usize * size_of::<f32>() * 4) as DeviceSize,
            BufferUsageFlags::STORAGE_BUFFER | BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::TRANSFER_SRC
        );

        // Pipelines
        let minmax_pipeline = match ctx.pipeline_store.insert(
            ComputePipelineConfig {
                shader_source: PathBuf::from("shaders/minmax_audio_pixels.comp"),
                descriptor_set_layouts: vec![
                    DescriptorSetLayout::new_push_descriptor(
                        ctx.device,
                        &[
                            DescriptorSetLayoutBinding::default()
                                .binding(0)
                                .descriptor_type(DescriptorType::STORAGE_BUFFER)
                                .descriptor_count(1)
                                .stage_flags(ShaderStageFlags::COMPUTE),
                            DescriptorSetLayoutBinding::default()
                                .binding(1)
                                .descriptor_type(DescriptorType::STORAGE_BUFFER)
                                .descriptor_count(1)
                                .stage_flags(ShaderStageFlags::COMPUTE),
                        ]
                    )
                ],
                push_constant_ranges: vec![
                    PushConstantRange::default()
                        .stage_flags(ShaderStageFlags::COMPUTE)
                        .offset(0)
                        .size(size_of::<PushConstants>() as u32)
                ],
                macros: HashMap::new()
            }
        ) {
            Ok(p) => { p }
            Err(e) => {
                panic!( "{}", e )
            }
        };

        let graph_pipeline = match ctx.pipeline_store.insert(
            GraphicsPipelineConfig {
                vertex_shader_source: PathBuf::from("shaders/audio_plot.vert"),
                fragment_shader_source: PathBuf::from("shaders/audio_plot.frag"),
                color_formats: vec![Format::R8G8B8A8_UNORM],
                depth_format: None,
                sample_count: SampleCountFlags::TYPE_4,
                descriptor_set_layouts: vec![
                    DescriptorSetLayout::new_push_descriptor(
                        ctx.device,
                        &[
                            DescriptorSetLayoutBinding::default()
                                .binding(0)
                                .descriptor_type(DescriptorType::STORAGE_BUFFER)
                                .descriptor_count(1)
                                .stage_flags(ShaderStageFlags::VERTEX),
                        ]
                    )
                ],
                push_constant_ranges: vec![
                    PushConstantRange::default()
                        .stage_flags(ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT)
                        .offset(0)
                        .size(size_of::<PushConstants>() as u32)
                ],
                macros: HashMap::new()
            }
        ) {
            Ok(p) => { p }
            Err(e) => {
                panic!( "{}", e )
            }
        };

        let background_pipeline = match ctx.pipeline_store.insert(
            GraphicsPipelineConfig {
                vertex_shader_source: PathBuf::from("shaders/fullscreen.vert"),
                fragment_shader_source: PathBuf::from("shaders/audio_background.frag"),
                color_formats: vec![Format::R8G8B8A8_UNORM],
                depth_format: None,
                sample_count: SampleCountFlags::TYPE_4,
                descriptor_set_layouts: vec![],
                push_constant_ranges: vec![
                    PushConstantRange::default()
                        .stage_flags(ShaderStageFlags::FRAGMENT)
                        .offset(0)
                        .size(size_of::<PushConstants>() as u32)
                ],
                macros: HashMap::new()
            }
        ) {
            Ok(p) => { p }
            Err(e) => {
                panic!( "{}", e )
            }
        };

        Self {
            audio_buffer,
            image,
            ms_image,
            minmax_buffer,
            minmax_pipeline,
            graph_pipeline,
            background_pipeline,
            zoom: 1.1,
            texture: None,
            sample_offset: SAMPLES_PER_SECOND as f32 / 2f32,
            current_sample: 0
        }
    }

    fn resize_gpu_handles(&mut self, device: &Device, allocator: &mut Allocator, width: u32, height: u32) {
        self.image = Image::new(
            device,
            allocator,
            ImageConfig {
                extent: Extent3D {
                    width,
                    height,
                    depth: 1
                },
                image_usage_flags: ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::COLOR_ATTACHMENT | ImageUsageFlags::SAMPLED,
                samples: SampleCountFlags::TYPE_1,
                filter: Filter::LINEAR,
                ..Default::default()
            }
        );

        self.ms_image = Image::new(
            device,
            allocator,
            ImageConfig {
                extent: Extent3D {
                    width,
                    height,
                    depth: 1
                },
                image_usage_flags: ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::COLOR_ATTACHMENT | ImageUsageFlags::SAMPLED,
                samples: SampleCountFlags::TYPE_4,
                filter: Filter::LINEAR,
                ..Default::default()
            }
        );

        self.minmax_buffer = Buffer::new(
            device,
            allocator,
            MemoryLocation::GpuOnly,
            (width as usize * size_of::<f32>() * 4) as DeviceSize, // Two floats (min, max) per pixel
            BufferUsageFlags::STORAGE_BUFFER | BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::TRANSFER_SRC
        );
    }
}

impl RenderComponent for PlotRenderer {
    fn render(&mut self, ctx: &mut RenderContext) {

        // Compute per-pixel min-max values
        let minmax_pipeline = ctx.pipeline_store.get(self.minmax_pipeline).unwrap();
        ctx.command_buffer.bind_pipeline(minmax_pipeline);

        let push_constants = PushConstants {
            samples_per_seconds: SAMPLES_PER_SECOND as u32,
            total_samples: BUFFER_SAMPLES as u32,
            pixels_x: self.image.width(),
            pixels_y: self.image.height(),
            zoom: self.zoom,
            offset: self.sample_offset as u32,
            current_sample: self.current_sample
        };
        ctx.command_buffer.push_constants(
            minmax_pipeline,
            ShaderStageFlags::COMPUTE,
            0,
            &bytemuck::cast_slice(std::slice::from_ref(&push_constants))
        );

        // Manual track as push_descriptor doesn't have support yet for tracking
        ctx.command_buffer.track(&self.audio_buffer);
        ctx.command_buffer.track(&self.minmax_buffer);
        ctx.command_buffer.push_descriptor_set(
            minmax_pipeline,
            0,
            &[
                WriteDescriptorSet::default()
                    .dst_binding(0)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&[self.audio_buffer.binding()]),
                WriteDescriptorSet::default()
                    .dst_binding(1)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&[self.minmax_buffer.binding()])
                ]
        );

        ctx.command_buffer.dispatch( (self.image.width() + 63) / 64, 1, 1);

        // Draw the plot
        ctx.command_buffer.image_barrier(
            &self.image,
            ImageLayout::UNDEFINED,
            ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            PipelineStageFlags::TOP_OF_PIPE,
            PipelineStageFlags::FRAGMENT_SHADER,
            AccessFlags::NONE,
            AccessFlags::SHADER_WRITE
        );
        ctx.command_buffer.image_barrier(
            &self.ms_image,
            ImageLayout::UNDEFINED,
            ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            PipelineStageFlags::TOP_OF_PIPE,
            PipelineStageFlags::FRAGMENT_SHADER,
            AccessFlags::NONE,
            AccessFlags::SHADER_WRITE
        );

        ctx.command_buffer.set_viewport(Viewport{ x: 0f32, y: 0f32, width: self.image.width() as f32, height: self.image.height() as f32, min_depth: 0f32, max_depth: 0f32});
        ctx.command_buffer.set_scissor(Rect2D { offset: Offset2D::default(), extent: Extent2D { width: self.image.width(), height: self.image.height() }});

        let color_attachments = vec![
            RenderingAttachmentInfo::default()
                .image_layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .load_op(AttachmentLoadOp::CLEAR)
                .store_op(AttachmentStoreOp::STORE)
                .clear_value(ClearValue { color: ClearColorValue { float32: [0f32, 0f32, 0f32, 1f32] } })
                .image_view(self.ms_image.image_view())
                .resolve_image_layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .resolve_image_view(self.image.image_view())
                .resolve_mode(ResolveModeFlags::AVERAGE)
        ];
        let rendering_info = vk::RenderingInfoKHR::default()
            .render_area(Rect2D { offset: Offset2D { x: 0, y: 0 }, extent: Extent2D { width: self.image.width(), height: self.image.height() } })
            .layer_count(1)
            .view_mask(0)
            .color_attachments(&color_attachments);
        ctx.command_buffer.begin_rendering(&rendering_info);
        {
            // Background
            let background_pipeline = ctx.pipeline_store.get(self.background_pipeline).unwrap();
            ctx.command_buffer.bind_pipeline(background_pipeline);

            let push_constants = PushConstants {
                samples_per_seconds: SAMPLES_PER_SECOND as u32,
                total_samples: BUFFER_SAMPLES as u32,
                pixels_x: self.image.width(),
                pixels_y: self.image.height(),
                zoom: self.zoom,
                offset: self.sample_offset as u32,
                current_sample: self.current_sample
            };
            ctx.command_buffer.push_constants(
                background_pipeline,
                ShaderStageFlags::FRAGMENT,
                0,
                &bytemuck::cast_slice(std::slice::from_ref(&push_constants))
            );
            ctx.command_buffer.draw(6, 1, 0,  0);

            // Graph
            let graph_pipeline = ctx.pipeline_store.get(self.graph_pipeline).unwrap();
            ctx.command_buffer.bind_pipeline(graph_pipeline);

            ctx.command_buffer.push_constants(
                graph_pipeline,
                ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT,
                0,
                &bytemuck::cast_slice(std::slice::from_ref(&push_constants))
            );

            ctx.command_buffer.push_descriptor_set(
                graph_pipeline,
                0,
                &[
                    WriteDescriptorSet::default()
                        .dst_binding(0)
                        .dst_array_element(0)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .buffer_info(&[self.minmax_buffer.binding()]),
                ]
            );

            ctx.command_buffer.draw(6, self.image.width(), 0,  0);
        }
        ctx.command_buffer.end_rendering();

        ctx.command_buffer.image_barrier(
            &self.image,
            ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            PipelineStageFlags::FRAGMENT_SHADER,
            PipelineStageFlags::BOTTOM_OF_PIPE,
            AccessFlags::SHADER_WRITE,
            AccessFlags::NONE
        );
    }
}

impl PlotRenderer {
    pub fn ui(&mut self, gui: &mut GuiHandler, ui: &mut Ui, current_sample: usize) {

        self.current_sample = current_sample;

        if self.texture.is_none() {
            self.texture = Some(gui.create_texture(&self.image));
        }

        let top_margin = 10.0;
        let label_width = 30.0;  // left axis
        let label_height = 20.0; // bottom timescale

        let mut total_rect = ui.available_rect_before_wrap();
        total_rect.set_height(f32::min( total_rect.width() / 2., total_rect.height() ));

        // Plot width
        let waveform_rect = Rect::from_min_max(
            Pos2::new(total_rect.min.x + label_width, total_rect.min.y + top_margin),
            Pos2::new(total_rect.max.x, total_rect.max.y - label_height),
        );

        let left_rect = Rect::from_min_max(
            Pos2::new(total_rect.min.x, total_rect.min.y + top_margin),
            Pos2::new(total_rect.min.x + label_width, total_rect.max.y - label_height),
        );

        let bottom_rect = Rect::from_min_max(
            Pos2::new(total_rect.min.x + label_width, total_rect.max.y - label_height),
            total_rect.max,
        );

        let (response, painter) = ui.allocate_painter(
            total_rect.size(),
            Sense::click_and_drag(),
        );

        // Recreate images if needed
        let scale = ui.ctx().pixels_per_point();
        let pixel_width  = (waveform_rect.width() * scale) as u32;
        let pixel_height  = (waveform_rect.height() * scale) as u32;
        if self.image.width() != pixel_width || self.image.height() != pixel_height {
            self.resize_gpu_handles(gui.device, gui.allocator, pixel_width, pixel_height);
            self.texture = Some(gui.create_texture(&self.image));
        }

        // Left labels
        for i in 0..=4 {
            let frac = i as f32 / 4.0;
            let amp = 1.0 - frac * 2.0; // 1.0 to -1.0
            let y = left_rect.min.y + frac * left_rect.height();
            painter.text(
                Pos2::new(left_rect.max.x - 4.0, y),
                egui::Align2::RIGHT_CENTER,
                format!("{:.1}", amp),
                egui::FontId::monospace(10.0),
                Color32::from_white_alpha(180),
            );
        }

        painter.image(
            gui.get_texture_id(self.texture.as_ref().unwrap()),
            waveform_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), // full UV
            Color32::WHITE,
        );

        // bottom timescale
        let view_start = (self.sample_offset as f32 - SAMPLES_PER_SECOND as f32 * self.zoom * 0.5) / SAMPLES_PER_SECOND as f32;
        let view_end   = (self.sample_offset as f32 + SAMPLES_PER_SECOND as f32 * self.zoom * 0.5) / SAMPLES_PER_SECOND as f32;
        let view_range = view_end - view_start;

        let rough_step = view_range / 8.0;
        let magnitude = rough_step.log10().floor();
        let pow = 10f32.powf(magnitude);
        let normalized = rough_step / pow;
        let step = if normalized < 2.0 { 2.0 } else if normalized < 5.0 { 5.0 } else { 10.0 } * pow;

        let first_tick = (view_start / step).ceil() * step;
        let mut t = first_tick;
        while t <= view_end {
            let frac = (t - view_start) / view_range;
            let x = bottom_rect.min.x + frac * bottom_rect.width();

            painter.line_segment(
                [Pos2::new(x, bottom_rect.min.y), Pos2::new(x, bottom_rect.min.y + 3.0)],
                Stroke::new(1.0, Color32::from_white_alpha(60)),
            );
            painter.text(
                Pos2::new(x, bottom_rect.min.y + 4.0),
                egui::Align2::CENTER_TOP,
                format!("{:.2}s", t),
                egui::FontId::monospace(10.0),
                Color32::from_white_alpha(180),
            );

            t += step;
        }

        // Interactions
        if let Some(_) = response.hover_pos() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.zoom *= 1.0 + scroll * -0.001;
            }
        }

        if response.dragged_by(egui::PointerButton::Primary) {
            let delta = response.drag_delta() * ui.pixels_per_point();
            let samples_per_pixel = SAMPLES_PER_SECOND as f32 / self.image.width() as f32 * self.zoom;
            self.sample_offset -= delta.x * samples_per_pixel;
        }

        // Scroll bar
        let bar_rect = Rect::from_min_max(
            Pos2::new(waveform_rect.min.x, waveform_rect.max.y - 6.0),
            waveform_rect.max,
        );

        let thumb_frac = self.sample_offset / BUFFER_SAMPLES as f32;
        let thumb_width = (bar_rect.width() * self.zoom / BUFFER_DURATION ).clamp(8.0, bar_rect.width());
        let thumb_x = bar_rect.min.x + thumb_frac * (bar_rect.width() - thumb_width);
        let thumb_rect = Rect::from_min_max(
            Pos2::new(thumb_x, bar_rect.min.y),
            Pos2::new(thumb_x + thumb_width, bar_rect.max.y),
        );

        if thumb_width < bar_rect.width() {

            // Draggable
            let thumb_response = ui.interact(thumb_rect, ui.id().with("scrollbar_thumb"), Sense::drag());
            if thumb_response.dragged() {
                let delta = thumb_response.drag_delta().x;
                let scroll_range = bar_rect.width() - thumb_width;
                self.sample_offset = (self.sample_offset + delta / scroll_range * BUFFER_SAMPLES as f32)
                    .clamp(0.0, BUFFER_SAMPLES as f32);
            }

            // Render
            let thumb_color = if thumb_response.dragged() {
                Color32::from_white_alpha(200)
            } else if thumb_response.hovered() {
                Color32::from_white_alpha(150)
            } else {
                Color32::from_white_alpha(100)
            };
            painter.rect_filled(thumb_rect, 3.0, thumb_color);
        }

    }
}
