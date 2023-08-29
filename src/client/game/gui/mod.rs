pub mod crosshair;
pub mod inventory;

use self::{
    crosshair::{Crosshair, CrosshairConfig},
    inventory::{Inventory, InventoryConfig},
};
use crate::client::{
    event_loop::{Event, EventHandler},
    renderer::{
        effect::{Blit, Effect, PostProcessor},
        Renderer,
    },
};
use nalgebra::{vector, Matrix4, Vector3};
use serde::Deserialize;

pub struct Gui {
    blit: Blit,
    crosshair: Crosshair,
    pub inventory: Inventory,
}

impl Gui {
    pub fn new(
        renderer: &Renderer,
        input_bind_group_layout: &wgpu::BindGroupLayout,
        textures_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        Self {
            blit: Blit::new(renderer, input_bind_group_layout, PostProcessor::FORMAT),
            crosshair: Crosshair::new(renderer, input_bind_group_layout),
            inventory: Inventory::new(renderer, textures_bind_group_layout),
        }
    }

    pub fn draw(
        &self,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        depth_view: &wgpu::TextureView,
        input_bind_group: &wgpu::BindGroup,
        textures_bind_group: &wgpu::BindGroup,
    ) {
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            self.blit.draw(&mut render_pass, input_bind_group);
            self.crosshair.draw(&mut render_pass, input_bind_group);
        }
        {
            self.inventory.draw(
                &mut encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: true,
                        }),
                        stencil_ops: None,
                    }),
                }),
                textures_bind_group,
            );
        }
    }

    fn element_size(Renderer { config, .. }: &Renderer, factor: f32) -> f32 {
        (config.height as f32 * 0.0325).max(13.5) * factor
    }

    fn element_scaling(size: f32) -> Vector3<f32> {
        vector![size, size, 1.0]
    }

    fn viewport(Renderer { config, .. }: &Renderer) -> Matrix4<f32> {
        Matrix4::new_translation(&vector![-1.0, -1.0, 0.0]).prepend_nonuniform_scaling(&vector![
            2.0 / config.width as f32,
            2.0 / config.height as f32,
            1.0
        ])
    }
}

impl EventHandler for Gui {
    type Context<'a> = &'a Renderer;

    fn handle(&mut self, event: &Event, renderer: Self::Context<'_>) {
        self.crosshair.handle(event, renderer);
        self.inventory.handle(event, renderer);
    }
}

#[derive(Deserialize)]
pub struct GuiConfig {
    crosshair: CrosshairConfig,
    inventory: InventoryConfig,
}
