use std::borrow::Cow;

use egui_wgpu::wgpu::util::DeviceExt as _;

use crate::mesh_cache::GpuMeshCache;
use crate::scene::RenderScene;

use super::mesh::{
    bounds_from_positions, bounds_vertices, build_vertices, cube_mesh, grid_and_axes,
    normals_vertices, LineVertex, Vertex, LINE_ATTRIBUTES, VERTEX_ATTRIBUTES,
};

pub(super) const DEPTH_FORMAT: egui_wgpu::wgpu::TextureFormat =
    egui_wgpu::wgpu::TextureFormat::Depth24Plus;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Uniforms {
    pub(super) view_proj: [[f32; 4]; 4],
    pub(super) light_dir: [f32; 3],
    pub(super) _pad0: f32,
    pub(super) camera_pos: [f32; 3],
    pub(super) _pad1: f32,
    pub(super) base_color: [f32; 3],
    pub(super) _pad2: f32,
    pub(super) debug_params: [f32; 4],
}

pub(super) struct PipelineState {
    pub(super) mesh_pipeline: egui_wgpu::wgpu::RenderPipeline,
    pub(super) line_pipeline: egui_wgpu::wgpu::RenderPipeline,
    pub(super) blit_pipeline: egui_wgpu::wgpu::RenderPipeline,
    pub(super) blit_bind_group: egui_wgpu::wgpu::BindGroup,
    pub(super) blit_bind_group_layout: egui_wgpu::wgpu::BindGroupLayout,
    pub(super) blit_sampler: egui_wgpu::wgpu::Sampler,
    pub(super) offscreen_texture: egui_wgpu::wgpu::Texture,
    pub(super) offscreen_view: egui_wgpu::wgpu::TextureView,
    pub(super) depth_texture: egui_wgpu::wgpu::Texture,
    pub(super) depth_view: egui_wgpu::wgpu::TextureView,
    pub(super) offscreen_size: [u32; 2],
    pub(super) uniform_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) uniform_bind_group: egui_wgpu::wgpu::BindGroup,
    pub(super) mesh_cache: GpuMeshCache,
    pub(super) mesh_id: u64,
    pub(super) mesh_vertices: Vec<Vertex>,
    pub(super) mesh_bounds: ([f32; 3], [f32; 3]),
    pub(super) index_count: u32,
    pub(super) scene_version: u64,
    pub(super) base_color: [f32; 3],
    pub(super) grid_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) grid_count: u32,
    pub(super) axes_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) axes_count: u32,
    pub(super) normals_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) normals_count: u32,
    pub(super) normals_length: f32,
    pub(super) bounds_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) bounds_count: u32,
}

impl PipelineState {
    pub(super) fn new(
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

pub(super) fn apply_scene_to_pipeline(
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
            usage: egui_wgpu::wgpu::BufferUsages::VERTEX
                | egui_wgpu::wgpu::BufferUsages::COPY_DST,
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

pub(super) fn ensure_offscreen_targets(
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
