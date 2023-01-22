use crate::client::event_loop::{Event, EventHandler};
use bitflags::bitflags;
use nalgebra::{matrix, vector, Matrix4, Point3, Vector3};
use std::{
    f32::consts::{FRAC_PI_2, TAU},
    time::Duration,
};
use winit::event::{
    DeviceEvent, ElementState, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent,
};

pub struct Camera {
    forward: Vector3<f32>,
    right: Vector3<f32>,
    up: Vector3<f32>,
    origin: Point3<f32>,
    world_up: Vector3<f32>,
    yaw: f32,
    pitch: f32,
}

impl Camera {
    pub fn new(origin: Point3<f32>, dir: Vector3<f32>, up: Vector3<f32>) -> Self {
        let world_up = up.normalize();
        let forward = dir.normalize();
        let right = world_up.cross(&forward).normalize();
        let up = forward.cross(&right);
        let yaw = forward.z.atan2(forward.x);
        let pitch = forward.y.asin();
        Self {
            forward,
            right,
            up,
            origin,
            world_up,
            yaw,
            pitch,
        }
    }

    pub fn forward(&self) -> Vector3<f32> {
        self.forward
    }

    pub fn right(&self) -> Vector3<f32> {
        self.right
    }

    pub fn up(&self) -> Vector3<f32> {
        self.up
    }

    pub fn origin(&self) -> Point3<f32> {
        self.origin
    }

    pub fn mat(&self) -> Matrix4<f32> {
        matrix![
            self.right.x,   self.right.y,   self.right.z,   -self.origin.coords.dot(&self.right);
            self.up.x,      self.up.y,      self.up.z,      -self.origin.coords.dot(&self.up);
            self.forward.x, self.forward.y, self.forward.z, -self.origin.coords.dot(&self.forward);
            0.0,            0.0,            0.0,            1.0;
        ]
    }
}

pub struct CameraController {
    dx: f32,
    dy: f32,
    relevant_keys: Keys,
    key_history: Keys,
    relevant_buttons: MouseButtons,
    button_history: MouseButtons,
    speed: f32,
    sensitivity: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            dx: 0.0,
            dy: 0.0,
            relevant_keys: Keys::empty(),
            key_history: Keys::empty(),
            relevant_buttons: MouseButtons::empty(),
            button_history: MouseButtons::empty(),
            speed,
            sensitivity,
        }
    }

    pub fn apply_updates(&mut self, camera: &mut Camera, dt: Duration) -> Changes {
        let mut changes = Changes::empty();

        if self.dx != 0.0 || self.dy != 0.0 {
            self.apply_rotation(camera);
            self.dx = 0.0;
            self.dy = 0.0;
            changes.insert(Changes::ROTATED);
        }

        if !self.relevant_keys.is_empty() {
            self.apply_movement(camera, dt);
            changes.insert(Changes::MOVED);
        }

        if !self.relevant_buttons.is_empty() {
            if self.relevant_buttons.contains(MouseButtons::LEFT) {
                changes.insert(Changes::BLOCK_DESTROYED);
            } else if self.relevant_buttons.contains(MouseButtons::RIGHT) {
                changes.insert(Changes::BLOCK_PLACED);
            }
        }

        changes
    }

    fn apply_rotation(&mut self, camera: &mut Camera) {
        const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;
        const ROTATION_SCALE: f32 = 1.0 / 60.0;

        let dr = self.sensitivity * ROTATION_SCALE;
        camera.yaw = (camera.yaw - self.dx * dr) % TAU;
        camera.pitch = (camera.pitch - self.dy * dr).clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);

        let (sin_yaw, cos_yaw) = camera.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = camera.pitch.sin_cos();

        camera.forward = vector![cos_yaw * cos_pitch, sin_pitch, sin_yaw * cos_pitch];
        camera.right = camera.world_up.cross(&camera.forward).normalize();
        camera.up = camera.forward.cross(&camera.right);
    }

    fn apply_movement(&mut self, camera: &mut Camera, dt: Duration) {
        let dp = self.speed * dt.as_secs_f32();
        let right = camera.right;
        let up = camera.world_up;
        let forward = right.cross(&up);

        let mut dir = Vector3::zeros();

        if self.relevant_keys.contains(Keys::W) {
            dir += forward;
        } else if self.relevant_keys.contains(Keys::S) {
            dir -= forward;
        }

        if self.relevant_keys.contains(Keys::A) {
            dir -= right;
        } else if self.relevant_keys.contains(Keys::D) {
            dir += right;
        }

        if self.relevant_keys.contains(Keys::LSHIFT) {
            dir -= up;
        } else if self.relevant_keys.contains(Keys::SPACE) {
            dir += up;
        }

        camera.origin += dir.normalize() * dp;
    }
}

impl EventHandler for CameraController {
    type Context<'a> = ();

    fn handle(&mut self, event: &Event, _: Self::Context<'_>) {
        match event {
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta: (dx, dy) },
                ..
            } => {
                self.dx += *dx as f32;
                self.dy += *dy as f32;
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(keycode),
                            state,
                            ..
                        },
                    ..
                } => {
                    let (key, opp) = match keycode {
                        VirtualKeyCode::W => (Keys::W, Keys::S),
                        VirtualKeyCode::A => (Keys::A, Keys::D),
                        VirtualKeyCode::S => (Keys::S, Keys::W),
                        VirtualKeyCode::D => (Keys::D, Keys::A),
                        VirtualKeyCode::LShift => (Keys::LSHIFT, Keys::SPACE),
                        VirtualKeyCode::Space => (Keys::SPACE, Keys::LSHIFT),
                        _ => return,
                    };
                    match state {
                        ElementState::Pressed => {
                            self.relevant_keys.insert(key);
                            self.relevant_keys.remove(opp);
                            self.key_history.insert(key);
                        }
                        ElementState::Released => {
                            self.relevant_keys.remove(key);
                            if self.key_history.contains(opp) {
                                self.relevant_keys.insert(opp);
                            }
                            self.key_history.remove(key);
                        }
                    }
                }
                WindowEvent::MouseInput { button, state, .. } => {
                    let (button, opp) = match button {
                        MouseButton::Left => (MouseButtons::LEFT, MouseButtons::RIGHT),
                        MouseButton::Right => (MouseButtons::RIGHT, MouseButtons::LEFT),
                        _ => return,
                    };
                    match state {
                        ElementState::Pressed => {
                            self.relevant_buttons.insert(button);
                            self.relevant_buttons.remove(opp);
                            self.button_history.insert(button);
                        }
                        ElementState::Released => {
                            self.relevant_buttons.remove(button);
                            if self.button_history.contains(opp) {
                                self.relevant_buttons.insert(opp);
                            }
                            self.button_history.remove(button);
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}

bitflags! {
    pub struct Changes: u8 {
        const ROTATED = 1 << 0;
        const MOVED = 1 << 1;
        const BLOCK_DESTROYED = 1 << 3;
        const BLOCK_PLACED = 1 << 2;
    }

    struct Keys: u8 {
        const W = 1 << 0;
        const A = 1 << 1;
        const S = 1 << 2;
        const D = 1 << 3;
        const LSHIFT = 1 << 4;
        const SPACE = 1 << 5;
    }

    struct MouseButtons: u8 {
        const LEFT = 1 << 0;
        const RIGHT = 1 << 1;
    }
}
