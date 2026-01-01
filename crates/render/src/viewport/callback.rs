use std::sync::{Arc, Mutex};

#[cfg(target_arch = "wasm32")]
use web_time::Instant;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use egui::epaint::Rect;
use egui_wgpu::{CallbackResources, CallbackTrait};

use crate::camera::{camera_position, camera_view_proj, CameraState};
use super::mesh::normals_vertices;
use super::pipeline::{
    apply_scene_to_pipeline, ensure_offscreen_targets, PipelineState, Uniforms,
};
use super::{ViewportDebug, ViewportSceneState, ViewportShadingMode, ViewportStatsState};

pub(super) struct ViewportCallback {
    pub(super) target_format: egui_wgpu::wgpu::TextureFormat,
    pub(super) rect: Rect,
    pub(super) camera: CameraState,
    pub(super) debug: ViewportDebug,
    pub(super) stats: Arc<Mutex<ViewportStatsState>>,
    pub(super) scene: Arc<Mutex<ViewportSceneState>>,
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
