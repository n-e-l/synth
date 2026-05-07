use std::collections::HashMap;
use std::path::PathBuf;
use bytemuck::{Pod, Zeroable};
use cen::app::engine::CenContext;
use cen::app::gui::GuiContext;
use cen::app::{ImageFlags, ImageResource};
use cen::ash::vk;
use cen::ash::vk::{AccessFlags, AttachmentLoadOp, AttachmentStoreOp, BufferUsageFlags, ClearColorValue, ClearValue, DependencyFlags, DescriptorSetLayoutBinding, DescriptorType, DeviceSize, Extent2D, Extent3D, Filter, Format, ImageLayout, ImageUsageFlags, Offset2D, PipelineStageFlags, PushConstantRange, Rect2D, RenderingAttachmentInfo, ResolveModeFlags, SampleCountFlags, ShaderStageFlags, Viewport, WriteDescriptorSet};
use cen::egui;
use cen::egui::{Color32, Pos2, Rect, Sense, Stroke, Ui};
use cen::gpu_allocator::MemoryLocation;
use cen::graphics::{GraphicsContext, ImageContext};
use cen::graphics::pipeline_store::{PipelineKey};
use cen::graphics::renderer::{RenderComponent};
use cen::vulkan::{Buffer, ComputePipelineConfig, DescriptorSetLayout, GraphicsPipelineConfig, ImageConfig, ImageTrait};
use crate::{BUFFER_DURATION, BUFFER_SAMPLES, SAMPLES_PER_SECOND};

pub struct PlotRenderer {
    zoom: f32,
    sample_offset: f32,
    minmax_pipeline: PipelineKey,
    graph_pipeline: PipelineKey,
    background_pipeline: PipelineKey,
    current_sample: usize,
    audio_buffer: Buffer,
    gpu_handles: GpuHandles
}

struct GpuHandles {
    image: ImageResource,
    ms_image: ImageResource,
    minmax_buffer: Buffer,
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
    current_sample: u32,
    channel: u32,
}

impl PlotRenderer {
    pub fn new(ctx: &mut CenContext, audio_buffer: Buffer) -> Self {

        // Image
        let width = 100;
        let height = 100;

        // Pipelines
        let minmax_pipeline = ctx.create_pipeline(
            ComputePipelineConfig {
                shader_source: PathBuf::from("shaders/minmax_audio_pixels.comp"),
                descriptor_set_layouts: vec![
                    DescriptorSetLayout::new_push_descriptor(
                        &ctx.gfx.device,
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
                ..Default::default()
            }
        ).unwrap_or_else(|e| panic!("{}", e));

        let graph_pipeline = ctx.create_pipeline(
            GraphicsPipelineConfig {
                vertex_shader_source: PathBuf::from("shaders/audio_plot.vert"),
                fragment_shader_source: PathBuf::from("shaders/audio_plot.frag"),
                color_formats: vec![Format::R8G8B8A8_UNORM],
                depth_format: None,
                sample_count: SampleCountFlags::TYPE_4,
                descriptor_set_layouts: vec![
                    DescriptorSetLayout::new_push_descriptor(
                        &ctx.gfx.device,
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
        ).unwrap_or_else(|e| panic!("{}", e));

        let background_pipeline = ctx.create_pipeline(
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
        ).unwrap_or_else(|e| panic!("{}", e));

        Self {
            gpu_handles: Self::create_gpu_handles(&mut ctx.gfx, &mut ctx.images, width, height),
            audio_buffer,
            minmax_pipeline,
            graph_pipeline,
            background_pipeline,
            zoom: 1.1,
            sample_offset: SAMPLES_PER_SECOND as f32 / 2f32,
            current_sample: 0
        }
    }

    fn create_gpu_handles(gfx: &mut GraphicsContext, image_context: &mut ImageContext, width: u32, height: u32) -> GpuHandles {
        let image = image_context.create_image(
                gfx,
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
            },
            ImageFlags::empty()
        );

        let ms_image = image_context.create_image(
            gfx,
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
            },
            ImageFlags::empty()
        );

        let minmax_buffer = Buffer::new(
            &gfx.device,
            &mut gfx.allocator,
            MemoryLocation::GpuOnly,
            (width as usize * size_of::<f32>() * 8) as DeviceSize, // Two floats (min, max) per pixel
            BufferUsageFlags::STORAGE_BUFFER | BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::TRANSFER_SRC
        );

        GpuHandles {
            image,
            ms_image,
            minmax_buffer
        }
    }
}

impl RenderComponent for PlotRenderer {
    fn render(&mut self, ctx: &mut CenContext) {

        // Compute per-pixel min-max values
        let minmax_pipeline = ctx.pipelines.get(self.minmax_pipeline).unwrap();
        ctx.command_buffer.bind_pipeline(minmax_pipeline);

        let image = ctx.images.get(&self.gpu_handles.image);
        let ms_image = ctx.images.get(&self.gpu_handles.ms_image);

        let push_constants = PushConstants {
            samples_per_seconds: SAMPLES_PER_SECOND as u32,
            total_samples: BUFFER_SAMPLES as u32,
            pixels_x: image.width(),
            pixels_y: image.height(),
            zoom: self.zoom,
            offset: self.sample_offset as u32,
            current_sample: self.current_sample as u32,
            channel: 0,
        };
        ctx.command_buffer.push_constants(
            minmax_pipeline,
            ShaderStageFlags::COMPUTE,
            0,
            &bytemuck::cast_slice(std::slice::from_ref(&push_constants))
        );

        // Manual track as push_descriptor doesn't have support yet for tracking
        ctx.command_buffer.track(&self.audio_buffer);
        ctx.command_buffer.track(&self.gpu_handles.minmax_buffer);

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
                    .buffer_info(&[self.gpu_handles.minmax_buffer.binding()])
                ]
        );

        ctx.command_buffer.dispatch( (image.width() + 63) / 64, 1, 1);

        // Transition minmax
        ctx.command_buffer.buffer_barrier(
            PipelineStageFlags::COMPUTE_SHADER,
            PipelineStageFlags::FRAGMENT_SHADER,
            AccessFlags::SHADER_WRITE,
            AccessFlags::SHADER_READ,
            DependencyFlags::empty(),
            self.gpu_handles.minmax_buffer.size(),
            0,
            &self.gpu_handles.minmax_buffer
        );

        // Draw the plot
        ctx.command_buffer.image_barrier(
            image,
            ImageLayout::UNDEFINED,
            ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            PipelineStageFlags::TOP_OF_PIPE,
            PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            AccessFlags::NONE,
            AccessFlags::COLOR_ATTACHMENT_WRITE
        );
        ctx.command_buffer.image_barrier(
            ms_image,
            ImageLayout::UNDEFINED,
            ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            PipelineStageFlags::TOP_OF_PIPE,
            PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            AccessFlags::NONE,
            AccessFlags::COLOR_ATTACHMENT_WRITE
        );

        ctx.command_buffer.set_viewport(Viewport{ x: 0f32, y: 0f32, width: image.width() as f32, height: image.height() as f32, min_depth: 0f32, max_depth: 0f32});
        ctx.command_buffer.set_scissor(Rect2D { offset: Offset2D::default(), extent: Extent2D { width: image.width(), height: image.height() }});

        let color_attachments = vec![
            RenderingAttachmentInfo::default()
                .image_layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .load_op(AttachmentLoadOp::CLEAR)
                .store_op(AttachmentStoreOp::STORE)
                .clear_value(ClearValue { color: ClearColorValue { float32: [0f32, 0f32, 0f32, 1f32] } })
                .image_view(ms_image.image_view())
                .resolve_image_layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .resolve_image_view(image.image_view())
                .resolve_mode(ResolveModeFlags::AVERAGE)
        ];
        let rendering_info = vk::RenderingInfoKHR::default()
            .render_area(Rect2D { offset: Offset2D { x: 0, y: 0 }, extent: Extent2D { width: image.width(), height: image.height() } })
            .layer_count(1)
            .view_mask(0)
            .color_attachments(&color_attachments);
        ctx.command_buffer.begin_rendering(&rendering_info);
        {
            // Background
            let background_pipeline = ctx.pipelines.get(self.background_pipeline).unwrap();
            ctx.command_buffer.bind_pipeline(background_pipeline);

            let mut push_constants = PushConstants {
                samples_per_seconds: SAMPLES_PER_SECOND as u32,
                total_samples: BUFFER_SAMPLES as u32,
                pixels_x: image.width(),
                pixels_y: image.height(),
                zoom: self.zoom,
                offset: self.sample_offset as u32,
                current_sample: self.current_sample as u32,
                channel: 0,
            };
            ctx.command_buffer.push_constants(
                background_pipeline,
                ShaderStageFlags::FRAGMENT,
                0,
                &bytemuck::cast_slice(std::slice::from_ref(&push_constants))
            );
            ctx.command_buffer.draw(6, 1, 0,  0);

            // Graph - channel 0
            let graph_pipeline = ctx.pipelines.get(self.graph_pipeline).unwrap();
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
                        .buffer_info(&[self.gpu_handles.minmax_buffer.binding()]),
                ]
            );

            // -1 to stop wrapping bug
            ctx.command_buffer.draw(6, image.width() - 1, 0,  0);

            // Graph - channel 1
            push_constants.channel = 1;
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
                        .buffer_info(&[self.gpu_handles.minmax_buffer.binding()]),
                ]
            );

            // -1 to stop wrapping bug
            ctx.command_buffer.draw(6, image.width() - 1, 0,  0);
        }
        ctx.command_buffer.end_rendering();

        ctx.command_buffer.image_barrier(
            image,
            ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            PipelineStageFlags::BOTTOM_OF_PIPE,
            AccessFlags::COLOR_ATTACHMENT_WRITE,
            AccessFlags::NONE
        );
    }
}

impl PlotRenderer {
    pub fn ui(&mut self, gui: &mut GuiContext, ui: &mut Ui, current_sample: usize) {

        self.current_sample = current_sample;

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

        let current_extent = gui.images.get(&self.gpu_handles.image).extent();
        if current_extent.width != pixel_width || current_extent.height != pixel_height {
            self.gpu_handles = Self::create_gpu_handles(&mut gui.gfx, &mut gui.images, pixel_width, pixel_height);
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
            gui.get_texture(&mut self.gpu_handles.image),
            waveform_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), // full UV
            Color32::WHITE,
        );

        // bottom timescale
        let view_start = (self.sample_offset - SAMPLES_PER_SECOND as f32 * self.zoom * 0.5) / SAMPLES_PER_SECOND as f32;
        let view_end   = (self.sample_offset + SAMPLES_PER_SECOND as f32 * self.zoom * 0.5) / SAMPLES_PER_SECOND as f32;
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
            let samples_per_pixel = SAMPLES_PER_SECOND as f32 / pixel_width as f32 * self.zoom;
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
