use std::borrow::Cow;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use egui::epaint::{PaintCallback, Rect};
use egui_wgpu::wgpu::util::DeviceExt as _;
use egui_wgpu::{Callback, CallbackResources, CallbackTrait};

use crate::camera::{camera_position, camera_view_proj, CameraState};
use crate::mesh_cache::GpuMeshCache;
use crate::scene::RenderScene;
mod mesh;
use mesh::{
    bounds_from_positions, bounds_vertices, build_vertices, cube_mesh, grid_and_axes,
    normals_vertices, LineVertex, Vertex, LINE_ATTRIBUTES, VERTEX_ATTRIBUTES,
};

const DEPTH_FORMAT: egui_wgpu::wgpu::TextureFormat = egui_wgpu::wgpu::TextureFormat::Depth24Plus;

pub struct ViewportRenderer {
    target_format: egui_wgpu::wgpu::TextureFormat,
    stats: Arc<Mutex<ViewportStatsState>>,
    scene: Arc<Mutex<ViewportSceneState>>,
}

#[derive(Debug, Clone, Copy)]
pub enum ViewportShadingMode {
    Lit,
    Normals,
    Depth,
}

#[derive(Debug, Clone, Copy)]
pub struct ViewportDebug {
    pub show_grid: bool,
    pub show_axes: bool,
    pub show_normals: bool,
    pub show_bounds: bool,
    pub normal_length: f32,
    pub shading_mode: ViewportShadingMode,
    pub depth_near: f32,
    pub depth_far: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct ViewportStats {
    pub fps: f32,
    pub frame_time_ms: f32,
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub mesh_count: u32,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_uploads: u64,
}

impl Default for ViewportStats {
    fn default() -> Self {
        Self {
            fps: 0.0,
            frame_time_ms: 0.0,
            vertex_count: 0,
            triangle_count: 0,
            mesh_count: 0,
            cache_hits: 0,
            cache_misses: 0,
            cache_uploads: 0,
        }
    }
}

struct ViewportStatsState {
    last_frame: Option<Instant>,
    stats: ViewportStats,
}

struct ViewportSceneState {
    version: u64,
    scene: Option<RenderScene>,
}

impl ViewportRenderer {
    pub fn new(target_format: egui_wgpu::wgpu::TextureFormat) -> Self {
        Self {
            target_format,
            stats: Arc::new(Mutex::new(ViewportStatsState {
                last_frame: None,
                stats: ViewportStats::default(),
            })),
            scene: Arc::new(Mutex::new(ViewportSceneState {
                version: 0,
                scene: None,
            })),
        }
    }

    pub fn paint_callback(
        &self,
        rect: Rect,
        camera: CameraState,
        debug: ViewportDebug,
    ) -> PaintCallback {
        Callback::new_paint_callback(
            rect,
            ViewportCallback {
                target_format: self.target_format,
                rect,
                camera,
                debug,
                stats: self.stats.clone(),
                scene: self.scene.clone(),
            },
        )
    }

    pub fn stats_snapshot(&self) -> ViewportStats {
        self.stats
            .lock()
            .map(|state| state.stats)
            .unwrap_or_default()
    }

    pub fn set_scene(&self, scene: RenderScene) {
        if let Ok(mut state) = self.scene.lock() {
            state.version = state.version.wrapping_add(1);
            state.scene = Some(scene);
        }
    }

    pub fn clear_scene(&self) {
        if let Ok(mut state) = self.scene.lock() {
            state.version = state.version.wrapping_add(1);
            state.scene = None;
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    light_dir: [f32; 3],
    _pad0: f32,
    camera_pos: [f32; 3],
    _pad1: f32,
    base_color: [f32; 3],
    _pad2: f32,
    debug_params: [f32; 4],
}

struct ViewportCallback {
    target_format: egui_wgpu::wgpu::TextureFormat,
    rect: Rect,
    camera: CameraState,
    debug: ViewportDebug,
    stats: Arc<Mutex<ViewportStatsState>>,
    scene: Arc<Mutex<ViewportSceneState>>,
}

struct PipelineState {
    mesh_pipeline: egui_wgpu::wgpu::RenderPipeline,
    line_pipeline: egui_wgpu::wgpu::RenderPipeline,
    blit_pipeline: egui_wgpu::wgpu::RenderPipeline,
    blit_bind_group: egui_wgpu::wgpu::BindGroup,
    blit_bind_group_layout: egui_wgpu::wgpu::BindGroupLayout,
    blit_sampler: egui_wgpu::wgpu::Sampler,
    offscreen_texture: egui_wgpu::wgpu::Texture,
    offscreen_view: egui_wgpu::wgpu::TextureView,
    depth_texture: egui_wgpu::wgpu::Texture,
    depth_view: egui_wgpu::wgpu::TextureView,
    offscreen_size: [u32; 2],
    uniform_buffer: egui_wgpu::wgpu::Buffer,
    uniform_bind_group: egui_wgpu::wgpu::BindGroup,
    mesh_cache: GpuMeshCache,
    mesh_id: u64,
    mesh_vertices: Vec<Vertex>,
    mesh_bounds: ([f32; 3], [f32; 3]),
    index_count: u32,
    scene_version: u64,
    base_color: [f32; 3],
    grid_buffer: egui_wgpu::wgpu::Buffer,
    grid_count: u32,
    axes_buffer: egui_wgpu::wgpu::Buffer,
    axes_count: u32,
    normals_buffer: egui_wgpu::wgpu::Buffer,
    normals_count: u32,
    normals_length: f32,
    bounds_buffer: egui_wgpu::wgpu::Buffer,
    bounds_count: u32,
}

impl PipelineState {
    fn new(
        device: &egui_wgpu::wgpu::Device,
        target_format: egui_wgpu::wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(egui_wgpu::wgpu::ShaderModuleDescriptor {
            label: Some("grapho_viewport_shader"),
            source: egui_wgpu::wgpu::ShaderSource::Wgsl(Cow::Borrowed(
                r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    light_dir: vec3<f32>,
    _pad0: f32,
    camera_pos: vec3<f32>,
    _pad1: f32,
    base_color: vec3<f32>,
    _pad2: f32,
    debug_params: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) world_pos: vec3<f32>,
    @location(2) color: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.world_pos = input.position;
    out.normal = input.normal;
    out.color = input.color;
    out.position = uniforms.view_proj * vec4<f32>(input.position, 1.0);
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.normal);
    let light_dir = normalize(uniforms.light_dir);
    let view_dir = normalize(uniforms.camera_pos - input.world_pos);
    let half_dir = normalize(light_dir + view_dir);
    let ndotl = max(dot(normal, light_dir), 0.0);
    let spec = pow(max(dot(normal, half_dir), 0.0), 32.0);
    let ambient = 0.15;
    let base = input.color * uniforms.base_color;
    let color = base * (ambient + ndotl) + vec3<f32>(0.9) * spec * 0.2;
    let mode = i32(uniforms.debug_params.x + 0.5);
    if mode == 1 {
        return vec4<f32>(normal * 0.5 + vec3<f32>(0.5), 1.0);
    }
    if mode == 2 {
        let near = uniforms.debug_params.y;
        let far = uniforms.debug_params.z;
        let denom = max(far - near, 0.0001);
        let dist = distance(uniforms.camera_pos, input.world_pos);
        let t = clamp((dist - near) / denom, 0.0, 1.0);
        return vec4<f32>(vec3<f32>(1.0 - t), 1.0);
    }
    return vec4<f32>(color, 1.0);
}

struct LineInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct LineOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_line(input: LineInput) -> LineOutput {
    var out: LineOutput;
    out.position = uniforms.view_proj * vec4<f32>(input.position, 1.0);
    out.color = input.color;
    return out;
}

@fragment
fn fs_line(input: LineOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
"#,
            )),
        });

        let uniform_buffer =
            device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
                label: Some("grapho_viewport_uniforms"),
                contents: bytemuck::bytes_of(&Uniforms {
                    view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
                    light_dir: [0.6, 1.0, 0.2],
                    _pad0: 0.0,
                    camera_pos: [0.0, 0.0, 5.0],
                    _pad1: 0.0,
                    base_color: [0.7, 0.72, 0.75],
                    _pad2: 0.0,
                    debug_params: [0.0, 0.5, 20.0, 0.0],
                }),
                usage: egui_wgpu::wgpu::BufferUsages::UNIFORM
                    | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            });

        let uniform_layout =
            device.create_bind_group_layout(&egui_wgpu::wgpu::BindGroupLayoutDescriptor {
                label: Some("grapho_viewport_uniform_layout"),
                entries: &[egui_wgpu::wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: egui_wgpu::wgpu::ShaderStages::VERTEX
                        | egui_wgpu::wgpu::ShaderStages::FRAGMENT,
                    ty: egui_wgpu::wgpu::BindingType::Buffer {
                        ty: egui_wgpu::wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let uniform_bind_group = device.create_bind_group(&egui_wgpu::wgpu::BindGroupDescriptor {
            label: Some("grapho_viewport_uniform_bind_group"),
            layout: &uniform_layout,
            entries: &[egui_wgpu::wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout =
            device.create_pipeline_layout(&egui_wgpu::wgpu::PipelineLayoutDescriptor {
                label: Some("grapho_viewport_layout"),
                bind_group_layouts: &[&uniform_layout],
                push_constant_ranges: &[],
            });

        let mesh_pipeline =
            device.create_render_pipeline(&egui_wgpu::wgpu::RenderPipelineDescriptor {
                label: Some("grapho_viewport_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: egui_wgpu::wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                    buffers: &[egui_wgpu::wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>()
                            as egui_wgpu::wgpu::BufferAddress,
                        step_mode: egui_wgpu::wgpu::VertexStepMode::Vertex,
                        attributes: &VERTEX_ATTRIBUTES,
                    }],
                },
                fragment: Some(egui_wgpu::wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
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
                depth_stencil: Some(egui_wgpu::wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: egui_wgpu::wgpu::CompareFunction::LessEqual,
                    stencil: egui_wgpu::wgpu::StencilState::default(),
                    bias: egui_wgpu::wgpu::DepthBiasState::default(),
                }),
                multisample: egui_wgpu::wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let line_pipeline =
            device.create_render_pipeline(&egui_wgpu::wgpu::RenderPipelineDescriptor {
                label: Some("grapho_viewport_lines"),
                layout: Some(&pipeline_layout),
                vertex: egui_wgpu::wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_line"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                    buffers: &[egui_wgpu::wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<LineVertex>()
                            as egui_wgpu::wgpu::BufferAddress,
                        step_mode: egui_wgpu::wgpu::VertexStepMode::Vertex,
                        attributes: &LINE_ATTRIBUTES,
                    }],
                },
                fragment: Some(egui_wgpu::wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_line"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(egui_wgpu::wgpu::ColorTargetState {
                        format: target_format,
                        blend: Some(egui_wgpu::wgpu::BlendState::REPLACE),
                        write_mask: egui_wgpu::wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: egui_wgpu::wgpu::PrimitiveState {
                    topology: egui_wgpu::wgpu::PrimitiveTopology::LineList,
                    ..Default::default()
                },
                depth_stencil: Some(egui_wgpu::wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: false,
                    depth_compare: egui_wgpu::wgpu::CompareFunction::LessEqual,
                    stencil: egui_wgpu::wgpu::StencilState::default(),
                    bias: egui_wgpu::wgpu::DepthBiasState::default(),
                }),
                multisample: egui_wgpu::wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let blit_shader = device.create_shader_module(egui_wgpu::wgpu::ShaderModuleDescriptor {
            label: Some("grapho_viewport_blit"),
            source: egui_wgpu::wgpu::ShaderSource::Wgsl(Cow::Borrowed(
                r#"
@group(0) @binding(0)
var blit_tex: texture_2d<f32>;

@group(0) @binding(1)
var blit_sampler: sampler;

struct BlitOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_blit(@builtin(vertex_index) index: u32) -> BlitOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );
    var out: BlitOut;
    out.position = vec4<f32>(positions[index], 0.0, 1.0);
    out.uv = uvs[index];
    return out;
}

@fragment
fn fs_blit(input: BlitOut) -> @location(0) vec4<f32> {
    return textureSample(blit_tex, blit_sampler, input.uv);
}
"#,
            )),
        });

        let blit_bind_group_layout =
            device.create_bind_group_layout(&egui_wgpu::wgpu::BindGroupLayoutDescriptor {
                label: Some("grapho_viewport_blit_layout"),
                entries: &[
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: egui_wgpu::wgpu::ShaderStages::FRAGMENT,
                        ty: egui_wgpu::wgpu::BindingType::Texture {
                            sample_type: egui_wgpu::wgpu::TextureSampleType::Float {
                                filterable: true,
                            },
                            view_dimension: egui_wgpu::wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: egui_wgpu::wgpu::ShaderStages::FRAGMENT,
                        ty: egui_wgpu::wgpu::BindingType::Sampler(
                            egui_wgpu::wgpu::SamplerBindingType::Filtering,
                        ),
                        count: None,
                    },
                ],
            });

        let blit_sampler = device.create_sampler(&egui_wgpu::wgpu::SamplerDescriptor {
            label: Some("grapho_viewport_blit_sampler"),
            mag_filter: egui_wgpu::wgpu::FilterMode::Linear,
            min_filter: egui_wgpu::wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let blit_pipeline_layout =
            device.create_pipeline_layout(&egui_wgpu::wgpu::PipelineLayoutDescriptor {
                label: Some("grapho_viewport_blit_pipeline_layout"),
                bind_group_layouts: &[&blit_bind_group_layout],
                push_constant_ranges: &[],
            });

        let blit_pipeline =
            device.create_render_pipeline(&egui_wgpu::wgpu::RenderPipelineDescriptor {
                label: Some("grapho_viewport_blit_pipeline"),
                layout: Some(&blit_pipeline_layout),
                vertex: egui_wgpu::wgpu::VertexState {
                    module: &blit_shader,
                    entry_point: Some("vs_blit"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                fragment: Some(egui_wgpu::wgpu::FragmentState {
                    module: &blit_shader,
                    entry_point: Some("fs_blit"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(egui_wgpu::wgpu::ColorTargetState {
                        format: target_format,
                        blend: Some(egui_wgpu::wgpu::BlendState::REPLACE),
                        write_mask: egui_wgpu::wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: egui_wgpu::wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: egui_wgpu::wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let (offscreen_texture, offscreen_view, depth_texture, depth_view) =
            create_offscreen_targets(device, target_format, 1, 1);
        let blit_bind_group = device.create_bind_group(&egui_wgpu::wgpu::BindGroupDescriptor {
            label: Some("grapho_viewport_blit_group"),
            layout: &blit_bind_group_layout,
            entries: &[
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 0,
                    resource: egui_wgpu::wgpu::BindingResource::TextureView(&offscreen_view),
                },
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 1,
                    resource: egui_wgpu::wgpu::BindingResource::Sampler(&blit_sampler),
                },
            ],
        });

        let mesh = cube_mesh();
        let mut mesh_cache = GpuMeshCache::new();
        let mesh_id = 1;
        mesh_cache.upload_or_update(
            device,
            mesh_id,
            bytemuck::cast_slice(&mesh.vertices),
            &mesh.indices,
        );
        let index_count = mesh.indices.len() as u32;
        let normals_length = 0.3;
        let normals_vertices = normals_vertices(&mesh.vertices, normals_length);
        let normals_buffer =
            device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
                label: Some("grapho_normals_vertices"),
                contents: bytemuck::cast_slice(&normals_vertices),
                usage: egui_wgpu::wgpu::BufferUsages::VERTEX
                    | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            });
        let bounds_vertices = bounds_vertices(mesh.bounds_min, mesh.bounds_max);
        let bounds_buffer =
            device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
                label: Some("grapho_bounds_vertices"),
                contents: bytemuck::cast_slice(&bounds_vertices),
                usage: egui_wgpu::wgpu::BufferUsages::VERTEX,
            });
        let (grid_vertices, axes_vertices) = grid_and_axes();
        let grid_buffer = device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
            label: Some("grapho_grid_vertices"),
            contents: bytemuck::cast_slice(&grid_vertices),
            usage: egui_wgpu::wgpu::BufferUsages::VERTEX,
        });
        let axes_buffer = device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
            label: Some("grapho_axes_vertices"),
            contents: bytemuck::cast_slice(&axes_vertices),
            usage: egui_wgpu::wgpu::BufferUsages::VERTEX,
        });

        Self {
            mesh_pipeline,
            line_pipeline,
            blit_pipeline,
            blit_bind_group,
            blit_bind_group_layout,
            blit_sampler,
            offscreen_texture,
            offscreen_view,
            depth_texture,
            depth_view,
            offscreen_size: [1, 1],
            uniform_buffer,
            uniform_bind_group,
            mesh_cache,
            mesh_id,
            mesh_vertices: mesh.vertices,
            mesh_bounds: (mesh.bounds_min, mesh.bounds_max),
            index_count,
            scene_version: 0,
            base_color: [0.7, 0.72, 0.75],
            grid_buffer,
            grid_count: grid_vertices.len() as u32,
            axes_buffer,
            axes_count: axes_vertices.len() as u32,
            normals_buffer,
            normals_count: normals_vertices.len() as u32,
            normals_length,
            bounds_buffer,
            bounds_count: bounds_vertices.len() as u32,
        }
    }
}

impl CallbackTrait for ViewportCallback {
    fn prepare(
        &self,
        device: &egui_wgpu::wgpu::Device,
        queue: &egui_wgpu::wgpu::Queue,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut egui_wgpu::wgpu::CommandEncoder,
        callback_resources: &mut CallbackResources,
    ) -> Vec<egui_wgpu::wgpu::CommandBuffer> {
        if callback_resources.get::<PipelineState>().is_none() {
            callback_resources.insert(PipelineState::new(device, self.target_format));
        }

        let view_proj = camera_view_proj(self.camera, self.rect, screen_descriptor);
        let camera_pos = camera_position(self.camera);
        let target = glam::Vec3::from(self.camera.target);
        let forward = (target - camera_pos).normalize_or_zero();
        let mut right = forward.cross(glam::Vec3::Y).normalize_or_zero();
        if right.length_squared() == 0.0 {
            right = glam::Vec3::X;
        }
        let up = right.cross(forward).normalize_or_zero();
        let light_dir = (-forward + right * 0.6 + up * 0.8)
            .normalize_or_zero()
            .to_array();
        let shading_mode = match self.debug.shading_mode {
            ViewportShadingMode::Lit => 0.0,
            ViewportShadingMode::Normals => 1.0,
            ViewportShadingMode::Depth => 2.0,
        };

        if let Some(pipeline) = callback_resources.get_mut::<PipelineState>() {
            let width = (self.rect.width() * screen_descriptor.pixels_per_point)
                .round()
                .max(1.0) as u32;
            let height = (self.rect.height() * screen_descriptor.pixels_per_point)
                .round()
                .max(1.0) as u32;
            ensure_offscreen_targets(device, pipeline, self.target_format, width, height);

            if let Ok(scene_state) = self.scene.lock() {
                match scene_state.scene.clone() {
                    Some(scene) => {
                        if scene_state.version != pipeline.scene_version {
                            apply_scene_to_pipeline(device, pipeline, &scene);
                            pipeline.scene_version = scene_state.version;
                            pipeline.base_color = scene.base_color;
                        }
                    }
                    None => {
                        if scene_state.version != pipeline.scene_version {
                            pipeline.mesh_vertices.clear();
                            pipeline.index_count = 0;
                            pipeline.mesh_bounds = ([0.0; 3], [0.0; 3]);
                            pipeline.base_color = [0.7, 0.72, 0.75];
                            pipeline.scene_version = scene_state.version;
                        }
                    }
                }
            }

            let uniforms = Uniforms {
                view_proj: view_proj.to_cols_array_2d(),
                light_dir,
                _pad0: 0.0,
                camera_pos: camera_pos.to_array(),
                _pad1: 0.0,
                base_color: pipeline.base_color,
                _pad2: 0.0,
                debug_params: [
                    shading_mode,
                    self.debug.depth_near,
                    self.debug.depth_far,
                    0.0,
                ],
            };

            queue.write_buffer(&pipeline.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

            if self.debug.show_normals
                && (self.debug.normal_length - pipeline.normals_length).abs() > 0.0001
            {
                let normals_vertices =
                    normals_vertices(&pipeline.mesh_vertices, self.debug.normal_length);
                queue.write_buffer(
                    &pipeline.normals_buffer,
                    0,
                    bytemuck::cast_slice(&normals_vertices),
                );
                pipeline.normals_length = self.debug.normal_length;
            }

            if let Ok(mut stats_state) = self.stats.lock() {
                let now = Instant::now();
                if let Some(last) = stats_state.last_frame {
                    let dt = (now - last).as_secs_f32();
                    if dt > 0.0 {
                        let fps = 1.0 / dt;
                        let frame_ms = dt * 1000.0;
                        let alpha = 0.1;
                        if stats_state.stats.fps == 0.0 {
                            stats_state.stats.fps = fps;
                            stats_state.stats.frame_time_ms = frame_ms;
                        } else {
                            stats_state.stats.fps += (fps - stats_state.stats.fps) * alpha;
                            stats_state.stats.frame_time_ms +=
                                (frame_ms - stats_state.stats.frame_time_ms) * alpha;
                        }
                    }
                }
                stats_state.last_frame = Some(now);

                let cache_stats = pipeline.mesh_cache.stats_snapshot();
                stats_state.stats.mesh_count = cache_stats.mesh_count;
                stats_state.stats.cache_hits = cache_stats.hits;
                stats_state.stats.cache_misses = cache_stats.misses;
                stats_state.stats.cache_uploads = cache_stats.uploads;
                stats_state.stats.vertex_count = pipeline.mesh_vertices.len() as u32;
                stats_state.stats.triangle_count = pipeline.index_count / 3;
            }

            let mesh = if pipeline.index_count > 0 {
                pipeline.mesh_cache.get(pipeline.mesh_id)
            } else {
                None
            };
            let mut render_pass =
                _egui_encoder.begin_render_pass(&egui_wgpu::wgpu::RenderPassDescriptor {
                    label: Some("grapho_viewport_offscreen"),
                    color_attachments: &[Some(egui_wgpu::wgpu::RenderPassColorAttachment {
                        view: &pipeline.offscreen_view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: egui_wgpu::wgpu::Operations {
                            load: egui_wgpu::wgpu::LoadOp::Clear(egui_wgpu::wgpu::Color {
                                r: 28.0 / 255.0,
                                g: 28.0 / 255.0,
                                b: 28.0 / 255.0,
                                a: 1.0,
                            }),
                            store: egui_wgpu::wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(
                        egui_wgpu::wgpu::RenderPassDepthStencilAttachment {
                            view: &pipeline.depth_view,
                            depth_ops: Some(egui_wgpu::wgpu::Operations {
                                load: egui_wgpu::wgpu::LoadOp::Clear(1.0),
                                store: egui_wgpu::wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        },
                    ),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

            render_pass.set_viewport(0.0, 0.0, width as f32, height as f32, 0.0, 1.0);
            if let Some(mesh) = mesh {
                render_pass.set_pipeline(&pipeline.mesh_pipeline);
                render_pass.set_bind_group(0, &pipeline.uniform_bind_group, &[]);
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass.set_index_buffer(
                    mesh.index_buffer.slice(..),
                    egui_wgpu::wgpu::IndexFormat::Uint32,
                );
                render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            }

            render_pass.set_pipeline(&pipeline.line_pipeline);
            render_pass.set_bind_group(0, &pipeline.uniform_bind_group, &[]);

            if self.debug.show_grid && pipeline.grid_count > 0 {
                render_pass.set_vertex_buffer(0, pipeline.grid_buffer.slice(..));
                render_pass.draw(0..pipeline.grid_count, 0..1);
            }

            if self.debug.show_axes && pipeline.axes_count > 0 {
                render_pass.set_vertex_buffer(0, pipeline.axes_buffer.slice(..));
                render_pass.draw(0..pipeline.axes_count, 0..1);
            }

            if self.debug.show_normals && pipeline.normals_count > 0 {
                render_pass.set_vertex_buffer(0, pipeline.normals_buffer.slice(..));
                render_pass.draw(0..pipeline.normals_count, 0..1);
            }

            if self.debug.show_bounds && pipeline.bounds_count > 0 {
                render_pass.set_vertex_buffer(0, pipeline.bounds_buffer.slice(..));
                render_pass.draw(0..pipeline.bounds_count, 0..1);
            }
        }

        Vec::new()
    }

    fn paint(
        &self,
        info: egui::epaint::PaintCallbackInfo,
        render_pass: &mut egui_wgpu::wgpu::RenderPass<'static>,
        callback_resources: &CallbackResources,
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
        render_pass.set_pipeline(&pipeline.blit_pipeline);
        render_pass.set_bind_group(0, &pipeline.blit_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

fn apply_scene_to_pipeline(
    device: &egui_wgpu::wgpu::Device,
    pipeline: &mut PipelineState,
    scene: &RenderScene,
) {
    let (vertices, indices) = build_vertices(&scene.mesh);
    pipeline.mesh_cache.upload_or_update(
        device,
        pipeline.mesh_id,
        bytemuck::cast_slice(&vertices),
        &indices,
    );

    pipeline.mesh_vertices = vertices;
    pipeline.index_count = indices.len() as u32;
    pipeline.mesh_bounds = bounds_from_positions(&scene.mesh.positions);

    let normals_vertices = normals_vertices(&pipeline.mesh_vertices, pipeline.normals_length);
    pipeline.normals_buffer =
        device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
            label: Some("grapho_normals_vertices"),
            contents: bytemuck::cast_slice(&normals_vertices),
            usage: egui_wgpu::wgpu::BufferUsages::VERTEX | egui_wgpu::wgpu::BufferUsages::COPY_DST,
        });
    pipeline.normals_count = normals_vertices.len() as u32;

    let bounds_vertices = bounds_vertices(pipeline.mesh_bounds.0, pipeline.mesh_bounds.1);
    pipeline.bounds_buffer =
        device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
            label: Some("grapho_bounds_vertices"),
            contents: bytemuck::cast_slice(&bounds_vertices),
            usage: egui_wgpu::wgpu::BufferUsages::VERTEX,
        });
    pipeline.bounds_count = bounds_vertices.len() as u32;
}

fn create_offscreen_targets(
    device: &egui_wgpu::wgpu::Device,
    target_format: egui_wgpu::wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> (
    egui_wgpu::wgpu::Texture,
    egui_wgpu::wgpu::TextureView,
    egui_wgpu::wgpu::Texture,
    egui_wgpu::wgpu::TextureView,
) {
    let size = egui_wgpu::wgpu::Extent3d {
        width: width.max(1),
        height: height.max(1),
        depth_or_array_layers: 1,
    };
    let offscreen_texture = device.create_texture(&egui_wgpu::wgpu::TextureDescriptor {
        label: Some("grapho_viewport_offscreen"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: egui_wgpu::wgpu::TextureDimension::D2,
        format: target_format,
        usage: egui_wgpu::wgpu::TextureUsages::RENDER_ATTACHMENT
            | egui_wgpu::wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let offscreen_view =
        offscreen_texture.create_view(&egui_wgpu::wgpu::TextureViewDescriptor::default());
    let depth_texture = device.create_texture(&egui_wgpu::wgpu::TextureDescriptor {
        label: Some("grapho_viewport_depth"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: egui_wgpu::wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: egui_wgpu::wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth_view = depth_texture.create_view(&egui_wgpu::wgpu::TextureViewDescriptor::default());
    (offscreen_texture, offscreen_view, depth_texture, depth_view)
}

fn ensure_offscreen_targets(
    device: &egui_wgpu::wgpu::Device,
    pipeline: &mut PipelineState,
    target_format: egui_wgpu::wgpu::TextureFormat,
    width: u32,
    height: u32,
) {
    let width = width.max(1);
    let height = height.max(1);
    if pipeline.offscreen_size == [width, height] {
        return;
    }

    let (offscreen_texture, offscreen_view, depth_texture, depth_view) =
        create_offscreen_targets(device, target_format, width, height);
    pipeline.offscreen_texture = offscreen_texture;
    pipeline.offscreen_view = offscreen_view;
    pipeline.depth_texture = depth_texture;
    pipeline.depth_view = depth_view;
    pipeline.offscreen_size = [width, height];
    pipeline.blit_bind_group = device.create_bind_group(&egui_wgpu::wgpu::BindGroupDescriptor {
        label: Some("grapho_viewport_blit_group"),
        layout: &pipeline.blit_bind_group_layout,
        entries: &[
            egui_wgpu::wgpu::BindGroupEntry {
                binding: 0,
                resource: egui_wgpu::wgpu::BindingResource::TextureView(&pipeline.offscreen_view),
            },
            egui_wgpu::wgpu::BindGroupEntry {
                binding: 1,
                resource: egui_wgpu::wgpu::BindingResource::Sampler(&pipeline.blit_sampler),
            },
        ],
    });
}
