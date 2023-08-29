use crate::{
    client::{
        event_loop::{Event, EventHandler},
        renderer::{
            effect::PostProcessor, program::Program, texture::image::ImageTextureArray, Renderer,
        },
        CLIENT_CONFIG,
    },
    server::{game::clock::Time, ServerEvent},
};
use bytemuck::{Pod, Zeroable};
use nalgebra::{vector, Matrix4, Point3, Vector3};
use serde::Deserialize;
use std::mem;

pub struct ObjectArray {
    textures: ImageTextureArray,
    program: Program,
    sun_pc: ObjectPushConstants,
    moon_pc: ObjectPushConstants,
}

impl ObjectArray {
    pub fn new(
        renderer: &Renderer,
        player_bind_group_layout: &wgpu::BindGroupLayout,
        sky_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
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
            wgpu::include_wgsl!("../../../../assets/shaders/object.wgsl"),
            &[],
            &[
                player_bind_group_layout,
                sky_bind_group_layout,
                textures.bind_group_layout(),
            ],
            &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::VERTEX_FRAGMENT,
                range: 0..mem::size_of::<ObjectPushConstants>() as u32,
            }],
            PostProcessor::FORMAT,
            None,
            None,
            None,
        );
        Self {
            textures,
            program,
            sun_pc: ObjectPushConstants::new_sun(Time::default()),
            moon_pc: ObjectPushConstants::new_moon(Time::default()),
        }
    }

    #[rustfmt::skip]
    pub fn draw<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        player_bind_group: &'a wgpu::BindGroup,
        sky_bind_group: &'a wgpu::BindGroup,
    ) {
        self.program.bind(
            render_pass,
            [
                player_bind_group,
                sky_bind_group,
                self.textures.bind_group(),
            ],
        );
        render_pass.set_push_constants(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            0,
            bytemuck::cast_slice(&[self.sun_pc]),
        );
        render_pass.draw(0..6, 0..1);
        render_pass.set_push_constants(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            0,
            bytemuck::cast_slice(&[self.moon_pc]),
        );
        render_pass.draw(0..6, 0..1);
    }
}

impl EventHandler for ObjectArray {
    type Context<'a> = ();

    fn handle(&mut self, event: &Event, _: Self::Context<'_>) {
        if let Event::UserEvent(ServerEvent::TimeUpdated(time)) = event {
            self.sun_pc = ObjectPushConstants::new_sun(*time);
            self.moon_pc = ObjectPushConstants::new_moon(*time);
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod)]
struct ObjectPushConstants {
    m: Matrix4<f32>,
    tex_idx: u32,
}

impl ObjectPushConstants {
    fn new_sun(time: Time) -> Self {
        Self::new(time.rotation() * Vector3::x(), 0, time.is_am())
    }

    fn new_moon(time: Time) -> Self {
        Self::new(time.rotation() * -Vector3::x(), 1, time.is_am())
    }

    fn new(dir: Vector3<f32>, tex_idx: u32, is_am: bool) -> Self {
        let size = CLIENT_CONFIG.sky.object.size;
        Self {
            m: Matrix4::face_towards(&dir.into(), &Point3::origin(), &Self::up(is_am))
                .prepend_nonuniform_scaling(&vector![size, size, 1.0]),
            tex_idx,
        }
    }

    fn up(is_am: bool) -> Vector3<f32> {
        if is_am {
            -Vector3::y()
        } else {
            Vector3::y()
        }
    }
}

#[derive(Deserialize)]
pub struct ObjectConfig {
    size: f32,
}
