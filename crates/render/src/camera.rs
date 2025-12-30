use egui::epaint::Rect;
use egui_wgpu::ScreenDescriptor;
use glam::{Mat4, Vec3};

#[derive(Debug, Clone, Copy)]
pub struct CameraState {
    pub target: [f32; 3],
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
}

pub fn camera_position(camera: CameraState) -> Vec3 {
    let direction = camera_direction(camera);
    let target = Vec3::from(camera.target);
    target + direction * camera.distance.max(0.1)
}

pub fn camera_view_proj(
    camera: CameraState,
    rect: Rect,
    screen_descriptor: &ScreenDescriptor,
) -> Mat4 {
    let viewport_width = (rect.width() * screen_descriptor.pixels_per_point).max(1.0);
    let viewport_height = (rect.height() * screen_descriptor.pixels_per_point).max(1.0);
    let aspect = viewport_width / viewport_height;

    let target = Vec3::from(camera.target);
    let position = camera_position(camera);

    let view = Mat4::look_at_rh(position, target, Vec3::Y);
    let projection = Mat4::perspective_rh(45_f32.to_radians(), aspect, 0.01, 1000.0);
    projection * view
}

fn camera_direction(camera: CameraState) -> Vec3 {
    let pitch = camera.pitch.clamp(-1.54, 1.54);
    let yaw = camera.yaw;

    let cos_pitch = pitch.cos();
    let sin_pitch = pitch.sin();
    let cos_yaw = yaw.cos();
    let sin_yaw = yaw.sin();

    Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw)
}
