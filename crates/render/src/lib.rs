use egui::epaint::{PaintCallback, Rect};
use egui_wgpu::{Callback, CallbackResources, CallbackTrait};

pub struct ViewportRenderer {
    target_format: egui_wgpu::wgpu::TextureFormat,
}

impl ViewportRenderer {
    pub fn new(target_format: egui_wgpu::wgpu::TextureFormat) -> Self {
        Self { target_format }
    }

    pub fn paint_callback(&self, rect: Rect) -> PaintCallback {
        Callback::new_paint_callback(
            rect,
            ViewportCallback {
                target_format: self.target_format,
            },
        )
    }
}

struct ViewportCallback {
    target_format: egui_wgpu::wgpu::TextureFormat,
}

struct PipelineState {
    pipeline: egui_wgpu::wgpu::RenderPipeline,
}

impl PipelineState {
    fn new(
        device: &egui_wgpu::wgpu::Device,
        target_format: egui_wgpu::wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(egui_wgpu::wgpu::ShaderModuleDescriptor {
            label: Some("grapho_viewport_shader"),
            source: egui_wgpu::wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                r#"
@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var pos = vec2<f32>(-1.0, -1.0);
    if (idx == 1u) {
        pos = vec2<f32>(3.0, -1.0);
    }
    if (idx == 2u) {
        pos = vec2<f32>(-1.0, 3.0);
    }
    return vec4<f32>(pos, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.08, 0.10, 0.12, 1.0);
}
"#,
            )),
        });

        let pipeline_layout =
            device.create_pipeline_layout(&egui_wgpu::wgpu::PipelineLayoutDescriptor {
                label: Some("grapho_viewport_layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let pipeline = device.create_render_pipeline(&egui_wgpu::wgpu::RenderPipelineDescriptor {
            label: Some("grapho_viewport_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: egui_wgpu::wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(egui_wgpu::wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(egui_wgpu::wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(egui_wgpu::wgpu::BlendState::REPLACE),
                    write_mask: egui_wgpu::wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: egui_wgpu::wgpu::PrimitiveState {
                topology: egui_wgpu::wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: egui_wgpu::wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self { pipeline }
    }
}

impl CallbackTrait for ViewportCallback {
    fn prepare(
        &self,
        device: &egui_wgpu::wgpu::Device,
        _queue: &egui_wgpu::wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut egui_wgpu::wgpu::CommandEncoder,
        callback_resources: &mut CallbackResources,
    ) -> Vec<egui_wgpu::wgpu::CommandBuffer> {
        if callback_resources.get::<PipelineState>().is_none() {
            callback_resources.insert(PipelineState::new(device, self.target_format));
        }
        Vec::new()
    }

    fn paint<'a>(
        &'a self,
        info: egui::epaint::PaintCallbackInfo,
        render_pass: &mut egui_wgpu::wgpu::RenderPass<'a>,
        callback_resources: &'a CallbackResources,
    ) {
        let viewport = info.viewport_in_pixels();
        if viewport.width_px <= 0 || viewport.height_px <= 0 {
            return;
        }

        let clip = info.clip_rect_in_pixels();
        if clip.width_px <= 0 || clip.height_px <= 0 {
            return;
        }

        let Some(pipeline) = callback_resources.get::<PipelineState>() else {
            return;
        };

        render_pass.set_viewport(
            viewport.left_px as f32,
            viewport.top_px as f32,
            viewport.width_px as f32,
            viewport.height_px as f32,
            0.0,
            1.0,
        );
        render_pass.set_scissor_rect(
            clip.left_px.max(0) as u32,
            clip.top_px.max(0) as u32,
            clip.width_px.max(0) as u32,
            clip.height_px.max(0) as u32,
        );
        render_pass.set_pipeline(&pipeline.pipeline);
        render_pass.draw(0..3, 0..1);
    }
}
