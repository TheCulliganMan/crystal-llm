//! Pure camera-rig calculation shared by the Bevy system and tests.

use bevy::prelude::{Quat, Transform, Vec2, Vec3};

use crate::{
    EXPECTED_GRID_SIZE,
    profile::{MAX_PROFILE_HEIGHT, MIN_PROFILE_HEIGHT, SOURCE_TILE_HEIGHT},
};

/// Diorama view measured upward from the horizontal ground plane. Forty-five
/// degrees gives terrain depth and authored vertical faces equal visual
/// weight, instead of the previous more top-down 65-degree presentation.
pub const CAMERA_PITCH_DEGREES: f32 = 45.0;
const CAMERA_FOCAL: f32 = 1.0;
const FAR_DEPTH_MARGIN: f32 = 4096.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoxelCameraPose {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub vertical_fov_radians: f32,
    pub near: f32,
    pub far: f32,
}

impl VoxelCameraPose {
    pub fn transform(self) -> Transform {
        Transform::from_translation(self.eye).looking_at(self.target, self.up)
    }
}

pub fn camera_pose(viewport_size: Vec2) -> VoxelCameraPose {
    let pitch = CAMERA_PITCH_DEGREES.to_radians();
    let tile_height = viewport_size.y / EXPECTED_GRID_SIZE.y as f32;
    let profile_scale = tile_height / SOURCE_TILE_HEIGHT;
    let target_height = (MAX_PROFILE_HEIGHT + MIN_PROFILE_HEIGHT) * 0.5 * profile_scale;
    let target = Vec3::new(0.0, target_height, 0.0);
    let camera_distance = CAMERA_FOCAL * viewport_size.y;
    let eye = target
        + Vec3::new(
            0.0,
            camera_distance * pitch.sin(),
            camera_distance * pitch.cos(),
        );
    VoxelCameraPose {
        eye,
        target,
        up: Vec3::Y,
        // Tie distance and field of view to the source viewport. Straight
        // down, this frames exactly one viewport-height of world pixels;
        // tilting introduces honest perspective instead of orthographically
        // crushing the ground into a short rectangle.
        vertical_fov_radians: 2.0 * (1.0 / (2.0 * CAMERA_FOCAL)).atan(),
        near: (camera_distance * 0.05).max(1.0),
        far: camera_distance * 4.0 + FAR_DEPTH_MARGIN,
    }
}

#[cfg(test)]
fn camera_forward(pose: VoxelCameraPose) -> Vec3 {
    (pose.target - pose.eye).normalize()
}

pub fn card_rotation_toward_camera(pose: VoxelCameraPose) -> Quat {
    let toward_eye = (pose.eye - pose.target).normalize_or_zero();
    let yaw = toward_eye.x.atan2(toward_eye.z);
    let pitch = toward_eye.y.asin();
    // The source sprites are front-on drawings. Rotate the card into the
    // camera plane around its bottom pivot so its pixels remain square on
    // screen instead of being foreshortened by the terrain pitch.
    Quat::from_rotation_y(yaw) * Quat::from_rotation_x(-pitch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_is_tilted_at_the_requested_pitch() {
        let pose = camera_pose(Vec2::new(160.0, 144.0));
        let toward_eye = (pose.eye - pose.target).normalize();
        let actual_pitch = toward_eye.y.asin().to_degrees();

        assert!((actual_pitch - CAMERA_PITCH_DEGREES).abs() < 0.001);
        assert!(pose.eye.y > pose.target.y);
        assert!(pose.eye.z > pose.target.z);
    }

    #[test]
    fn perspective_focal_frames_one_viewport_height_when_top_down() {
        let viewport = Vec2::new(160.0, 144.0);
        let pose = camera_pose(viewport);
        let distance = CAMERA_FOCAL * viewport.y;
        let framed_height = 2.0 * distance * (pose.vertical_fov_radians * 0.5).tan();
        assert!((framed_height - viewport.y).abs() < 1.0e-5);
        assert!(pose.near > 0.0);
        assert!(pose.far > distance);
    }

    #[test]
    fn runtime_projection_depth_range_contains_authored_world() {
        let pose = camera_pose(Vec2::new(640.0, 576.0));
        assert!(pose.near < (pose.eye - pose.target).length());
        assert!(pose.far > (pose.eye - pose.target).length() + 640.0);
    }

    #[test]
    fn transform_faces_the_camera_target() {
        let pose = camera_pose(Vec2::new(160.0, 144.0));
        let transform = pose.transform();
        assert!(transform.forward().dot(camera_forward(pose)) > 0.999);
    }

    #[test]
    fn actor_card_faces_the_camera_without_foreshortening() {
        let pose = camera_pose(Vec2::new(640.0, 576.0));
        let rotation = card_rotation_toward_camera(pose);
        let toward_eye = (pose.eye - pose.target).normalize();
        let camera_up = pose.transform().up().as_vec3();

        assert!(rotation.mul_vec3(Vec3::Z).dot(toward_eye) > 0.999);
        assert!(rotation.mul_vec3(Vec3::Y).dot(camera_up) > 0.999);
    }
}
