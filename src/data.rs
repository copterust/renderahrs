use std::io::BufRead;
use std::sync;
use std::thread;
use std::fmt::Write;
use std::time::Duration;

use bevy::math::Quat;
use serde::Deserialize;
use serde_json;

use crate::intg;

pub trait AnimSource {
    fn think(&mut self, label: &mut String);
    fn get_gyro(&self) -> Quat;
    fn get_quat(&self) -> Quat;
    fn get_arrows(&self) -> [[f32; 3]; 2];
}

#[derive(Default, Deserialize)]
pub struct Sample {
    pub dt: f32,
    pub accel: [f32; 3],
    pub gyro: [f32; 3],
    pub mag: [f32; 3],
    pub state: [[f32; 7]; 1],
}

#[derive(Default)]
pub struct FileData {
    pub samples: Vec<Sample>,
    pub frame: usize,
    pub intg: intg::Gyro,
}

impl FileData {
    pub fn load(name: &str) -> FileData {
        let f = std::io::BufReader::new(std::fs::File::open(name).unwrap());
        let mut v = Vec::new();
        for l in f.lines() {
            let l = l.unwrap();
            let s: Sample = match serde_json::from_str(&l) {
                Ok(s) => s,
                Err(_) => continue,
            };
            v.push(s);
        }

        FileData {
            samples: v,
            ..FileData::default()
        }
    }
}

impl AnimSource for FileData {
    fn think(&mut self, label: &mut String) {
        if self.frame == 0 {
            self.intg.reset();
        }
        let s = &self.samples[self.frame];
        self.intg.add_sample(s.dt, s.gyro);

        self.frame = (self.frame + 1) % self.samples.len();

        label.clear();
        write!(label, "{} / {}", self.frame, self.samples.len()).unwrap();
    }

    fn get_gyro(&self) -> Quat {
        self.intg.t.rotation
    }

    fn get_quat(&self) -> Quat {
        let s = self.samples[self.frame].state[0];
        Quat::from_xyzw(s[1], s[2], s[3], s[0])
    }

    fn get_arrows(&self) -> [[f32; 3]; 2] {
        let s = &self.samples[self.frame];
        [s.accel, s.mag]
    }
}

#[derive(Default)]
pub struct Stream {
    data: sync::Arc<sync::RwLock<StreamData>>,
}

#[derive(Default)]
pub struct StreamData {
    time: f32,
    prev: f32,
    open: bool,
    intg: intg::Gyro,
    sample: Box<Sample>,
}

impl Stream {
    pub fn start(name: &str) -> Stream {
        let stream = Stream::default();
        let data = stream.data.clone();
        let name = name.to_string();
        thread::spawn(move || read_forever(&name, data));
        stream
    }
}

fn read_forever(name: &str, data: sync::Arc<sync::RwLock<StreamData>>) {
    let mut buffer = Box::new(Sample::default());

    loop {
        let f = loop {
            match serialport::new(name, 460800).timeout(Duration::new(60, 0)).open() {
                Ok(f) => break f,
                Err(e) => {
                    thread::sleep_ms(100);
                }
            }
        };
        let f = std::io::BufReader::new(f);
        eprintln!("{}: connected", name);
        for l in f.lines() {
            let l = match l {
                Ok(l) => l,
                Err(e) => {
                    use std::io::ErrorKind::*;
                    match e.kind() {
                        BrokenPipe | TimedOut | UnexpectedEof => break,
                        _ => continue,
                    }
                },
            };
            *buffer = match serde_json::from_str(&l) {
                Ok(s) => s,
                Err(_) => continue,
            };

            {
                let mut d = data.write().unwrap();
                d.time += buffer.dt;
                d.open = true;
                d.intg.add_sample(buffer.dt, buffer.gyro);
                std::mem::swap(&mut d.sample, &mut buffer);
            }
        }
        eprintln!("{}: disconnected", name);
        {
            let mut d = data.write().unwrap();
            d.open = false;
            d.time = 0.0;
            d.prev = 0.0;
            d.intg.reset();
        }
    }
}

impl AnimSource for Stream {
    fn think(&mut self, label: &mut String) {
        let (open, time, prev) = {
            let mut d = self.data.write().unwrap();
            let t = d.time;
            let p = d.prev;
            d.prev = d.time;
            (d.open, t, p)
        };

        label.clear();
        if open {
            write!(label, "Live {:.2} ({}ms)", time * 0.001, time - prev).unwrap();
        } else {
            write!(label, "Offline").unwrap();
        }
    }

    fn get_gyro(&self) -> Quat {
        let data = self.data.read().unwrap();
        data.intg.t.rotation
    }

    fn get_quat(&self) -> Quat {
        let data = self.data.read().unwrap();
        let s = &data.sample.state[0];
        Quat::from_xyzw(s[1], s[2], s[3], s[0])
    }

    fn get_arrows(&self) -> [[f32; 3]; 2] {
        let s = &self.data.read().unwrap().sample;
        [s.accel, s.mag]
    }
}
