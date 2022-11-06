use bevy::prelude::*;

#[derive(Default)]
pub struct Gyro {
    pub t: Transform,
}

impl Gyro {
    pub fn add_sample(&mut self, dt_ms: f32, w: [f32; 3]) {
        // mpu9520 X right, Y fwd, Z down, left handed
        // bevy    X right, Y up, Z back, right handed
        let dt = dt_ms * 0.001;

        self.t.rotate_local_axis(Vec3::X, -w[0] * dt);
        self.t.rotate_local_axis(Vec3::Y, w[2] * dt);
        self.t.rotate_local_axis(Vec3::Z, w[1] * dt);
    }

    pub fn reset(&mut self) {
        self.t = Transform::default();
    }
}
