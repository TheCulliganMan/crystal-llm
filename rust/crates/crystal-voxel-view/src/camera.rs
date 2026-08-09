//! Pure camera-rig calculation shared by the Bevy system and tests.

use bevy::prelude::{Quat, Transform, Vec2, Vec3};

use crate::{
    EXPECTED_GRID_SIZE,
    profile::{MAX_PROFILE_HEIGHT, MIN_PROFILE_HEIGHT, SOURCE_TILE_HEIGHT},
};

pub const CAMERA_PITCH_DEGREES: f32 = 65.0;
const CAMERA_DISTANCE: f32 = 512.0;
const PROJECTION_MARGIN: f32 = 1.02;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoxelCameraPose {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub projection_size: Vec2,
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
    let eye = target
        + Vec3::new(
            0.0,
            CAMERA_DISTANCE * pitch.sin(),
            CAMERA_DISTANCE * pitch.cos(),
        );
    VoxelCameraPose {
        eye,
        target,
        up: Vec3::Y,
        // Fixed projection and render target must have the same aspect ratio;
        // a mismatched computed height would anisotropically stretch pixels.
        projection_size: viewport_size * PROJECTION_MARGIN,
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
    fn orthographic_extent_preserves_the_render_target_aspect() {
        let viewport = Vec2::new(160.0, 144.0);
        let projection = camera_pose(viewport).projection_size;
        assert_eq!(projection, viewport * PROJECTION_MARGIN);
        assert!((projection.x / projection.y - viewport.x / viewport.y).abs() < 1.0e-6);
    }

    #[test]
    fn runtime_projection_contains_profile_height_and_depth_corners() {
        let viewport = Vec2::new(640.0, 576.0);
        let pose = camera_pose(viewport);
        let pitch = CAMERA_PITCH_DEGREES.to_radians();
        let scale = viewport.y / EXPECTED_GRID_SIZE.y as f32 / SOURCE_TILE_HEIGHT;
        let half_projection = pose.projection_size.y * 0.5;

        for height in [MIN_PROFILE_HEIGHT * scale, MAX_PROFILE_HEIGHT * scale] {
            for depth in [-viewport.y * 0.5, viewport.y * 0.5] {
                let screen_y = (height - pose.target.y) * pitch.cos() - depth * pitch.sin();
                assert!(screen_y.abs() <= half_projection);
            }
        }
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
