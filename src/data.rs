use std::io::BufRead;
use serde::Deserialize;
use serde_json;
use std::sync;
use std::thread;
use std::fmt::Write;
use std::time::Duration;

pub trait AnimSource {
    fn think(&mut self, label: &mut String);
    fn get_gyro(&self) -> ([f32; 3], bool);
    fn get_quat(&self) -> [f32; 4];
}

#[derive(Default, Deserialize)]
pub struct Sample {
    pub dt: f32,
    pub _accel: [f32; 3],
    pub gyro: [f32; 3],
    pub _mag: [f32; 3],
    pub state: [[f32; 7]; 1],
}

pub struct FileData {
    pub samples: Vec<Sample>,
    pub frame: usize,
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
            frame: 0,
        }
    }
}

impl AnimSource for FileData {
    fn think(&mut self, label: &mut String) {
        self.frame = (self.frame + 1) % self.samples.len();

        label.clear();
        write!(label, "{} / {}", self.frame, self.samples.len()).unwrap();
    }

    fn get_gyro(&self) -> ([f32; 3], bool) {
        let s = &self.samples[self.frame];
        let g = s.gyro;
        let dt = s.dt * 0.001;
        ([g[0] * dt, g[1] * dt, g[2] * dt], self.frame == 0)
    }

    fn get_quat(&self) -> [f32; 4] {
        let s = self.samples[self.frame].state[0];
        [ s[0], s[1], s[2], s[3] ]
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
                std::mem::swap(&mut d.sample, &mut buffer);
            }
        }
        eprintln!("{}: disconnected", name);
        {
            let mut d = data.write().unwrap();
            d.open = false;
            d.time = 0.0;
            d.prev = 0.0;
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

    fn get_gyro(&self) -> ([f32; 3], bool) {
        let data = self.data.read().unwrap();
        let g = &data.sample.gyro;
        let dt = data.sample.dt * 0.001;
        ([g[0]*dt, g[1]*dt, g[2]*dt], !data.open)
    }

    fn get_quat(&self) -> [f32; 4] {
        let data = self.data.read().unwrap();
        let s = &data.sample.state[0];
        [s[0], s[1], s[2], s[3]]
    }
}

