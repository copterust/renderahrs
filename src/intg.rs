use bevy::prelude::*;

#[derive(Default)]
pub struct Gyro {
    pub q: Quat,
}

impl Gyro {
    pub fn add_sample(&mut self, dt_ms: f32, w: [f32; 3]) {
        // mpu9520 X right, Y fwd, Z down, left handed
        // bevy    X right, Y up, Z back, right handed
        let dt = dt_ms * 0.001;
        let lx = Quat::from_axis_angle(self.q * Vec3::new(1., 0., 0.), w[0] * dt);
        let ly = Quat::from_axis_angle(self.q * Vec3::new(0., 1., 0.), w[2] * dt);
        let lz = Quat::from_axis_angle(self.q * Vec3::new(0., 0., 1.), -w[1] * dt);
        self.q *= lx * ly * lz;
    }

    pub fn reset(&mut self) {
        self.q = Quat::default();
    }
}
