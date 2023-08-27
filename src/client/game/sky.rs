use super::player::Player;
use crate::{
    client::{
        event_loop::{Event, EventHandler},
        renderer::{
            effect::PostProcessor, program::Program, texture::image::ImageTextureArray,
            uniform::Uniform, Renderer,
        },
    },
    server::{
        game::clock::{Stage, Time},
        ServerEvent,
    },
    shared::{color::Rgb, utils},
};
use bytemuck::{Pod, Zeroable};
use nalgebra::{vector, Matrix4, Point3, Vector3};
use std::{f32::consts::PI, mem};

pub struct Sky {
    objects: Objects,
    uniform: Uniform<SkyUniformData>,
    sun_intensity: Option<f32>,
    time: Result<Time, Time>,
}

impl Sky {
    pub fn new(renderer: &Renderer, player_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        Self {
            objects: Objects::new(renderer, player_bind_group_layout),
            uniform: Uniform::uninit_mut(renderer, wgpu::ShaderStages::VERTEX),
            sun_intensity: None,
            time: Err(Default::default()),
        }
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        self.uniform.bind_group_layout()
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        self.uniform.bind_group()
    }

    pub fn draw(
        &self,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        player_bind_group: &wgpu::BindGroup,
    ) {
        let time = self.time.unwrap_or_else(|_| unreachable!());
        self.objects.draw(
            &mut encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(Default::default()),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            }),
            player_bind_group,
            time.sun_dir(),
            self.sun_intensity.unwrap_or_else(|| unreachable!()),
            time.moon_dir(),
            time.is_am(),
        );
    }
}

impl EventHandler for Sky {
    type Context<'a> = &'a Renderer;

    fn handle(&mut self, event: &Event, renderer: Self::Context<'_>) {
        match event {
            Event::UserEvent(ServerEvent::TimeUpdated(time)) => {
                self.time = Err(*time);
            }
            Event::MainEventsCleared => {
                if let Err(time) = self.time {
                    let data = SkyUniformData::new(time.stage());
                    self.uniform.set(renderer, &data);
                    self.sun_intensity = Some(data.sun_intensity);
                    self.time = Ok(time);
                }
            }
            _ => {}
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod)]
struct SkyUniformData {
    light_intensity: Rgb<f32>,
    sun_intensity: f32,
}

impl SkyUniformData {
    const DAY_INTENSITY: Rgb<f32> = Rgb::splat(1.0);
    const NIGHT_INTENSITY: Rgb<f32> = Rgb::new(0.15, 0.15, 0.3);
    const SUN_INTENSITY: f32 = 15.0;

    fn new(stage: Stage) -> Self {
        match stage {
            Stage::Dawn { progress } => Self {
                light_intensity: utils::lerp(Self::NIGHT_INTENSITY, Self::DAY_INTENSITY, progress),
                sun_intensity: utils::lerp(0.0, Self::SUN_INTENSITY, progress),
            },
            Stage::Day => Self {
                light_intensity: Self::DAY_INTENSITY,
                sun_intensity: Self::SUN_INTENSITY,
            },
            Stage::Dusk { progress } => Self {
                light_intensity: utils::lerp(Self::DAY_INTENSITY, Self::NIGHT_INTENSITY, progress),
                sun_intensity: utils::lerp(Self::SUN_INTENSITY, 0.0, progress),
            },
            Stage::Night => Self {
                light_intensity: Self::NIGHT_INTENSITY,
                sun_intensity: 0.0,
            },
        }
    }
}

struct Objects {
    textures: ImageTextureArray,
    program: Program,
}

impl Objects {
    fn new(renderer: &Renderer, player_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        let textures = ImageTextureArray::new(
            renderer,
            [
                "assets/textures/sky/sun.png",
                "assets/textures/sky/moon.png",
            ],
            true,
            true,
            1,
        );
        let program = Program::new(
            renderer,
            wgpu::include_wgsl!("../../../assets/shaders/object.wgsl"),
            &[],
            &[player_bind_group_layout, textures.bind_group_layout()],
            &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::VERTEX_FRAGMENT,
                range: 0..mem::size_of::<ObjectsPushConstants>() as u32,
            }],
            PostProcessor::FORMAT,
            None,
            None,
            None,
        );
        Self { textures, program }
    }

    #[rustfmt::skip]
    fn draw<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        player_bind_group: &'a wgpu::BindGroup,
        sun_dir: Vector3<f32>,
        sun_intensity: f32,
        moon_dir: Vector3<f32>,
        is_am: bool,
    ) {
        self.program.bind(render_pass, [player_bind_group, self.textures.bind_group()]);
        render_pass.set_push_constants(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            0,
            bytemuck::cast_slice(&[ObjectsPushConstants::sun(sun_dir, sun_intensity, is_am)]),
        );
        render_pass.draw(0..6, 0..1);
        render_pass.set_push_constants(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            0,
            bytemuck::cast_slice(&[ObjectsPushConstants::moon(moon_dir, is_am)]),
        );
        render_pass.draw(0..6, 0..1);
    }
}

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod)]
struct ObjectsPushConstants {
    transform: Matrix4<f32>,
    tex_idx: u32,
    brightness: f32,
}

impl ObjectsPushConstants {
    const SIZE: f32 = 0.125;

    fn sun(dir: Vector3<f32>, intensity: f32, is_am: bool) -> Self {
        Self::new(dir, Self::SIZE, 0, intensity.max(1.0), is_am)
    }

    fn moon(dir: Vector3<f32>, is_am: bool) -> Self {
        Self::new(dir, Self::SIZE, 1, 1.0, is_am)
    }

    fn new(dir: Vector3<f32>, size: f32, tex_idx: u32, brightness: f32, is_am: bool) -> Self {
        Self {
            transform: Matrix4::face_towards(&dir.into(), &Point3::origin(), &Player::WORLD_UP)
                * Matrix4::new_nonuniform_scaling(&vector![size, size, 1.0])
                * if is_am {
                    Matrix4::new_rotation(Vector3::y() * PI)
                        * Matrix4::new_rotation(Vector3::x() * PI)
                } else {
                    Matrix4::identity()
                },
            tex_idx,
            brightness,
        }
    }
}
