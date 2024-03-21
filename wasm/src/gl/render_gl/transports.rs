use super::{super::utils::Flatten, lerp, RenderContext};
use crate::AsteroidColonies;

use ::asteroid_colonies_logic::{ItemType, Transport, TILE_SIZE};
use cgmath::{Matrix3, Matrix4, SquareMatrix, Vector3};

use web_sys::WebGlRenderingContext as GL;

impl AsteroidColonies {
    pub(super) fn render_gl_transports(&self, gl: &GL, ctx: &RenderContext) {
        let RenderContext {
            frac_frame,
            assets,
            shader,
            to_screen,
            offset,
            scale,
            ..
        } = ctx;

        gl.uniform1f(shader.alpha_loc.as_ref(), 1.0);
        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::identity().flatten(),
        );

        let render_transport = |t: &Transport| {
            let Some(&pos) = t.path.last() else {
                return;
            };
            let tex = match t.item {
                ItemType::RawOre => &assets.tex_raw_ore,
                ItemType::IronIngot => &assets.tex_iron_ingot,
                ItemType::CopperIngot => &assets.tex_copper_ingot,
                ItemType::LithiumIngot => &assets.tex_lithium_ingot,
                ItemType::Cilicate => &assets.tex_cilicate,
                ItemType::Gear => &assets.tex_gear,
                ItemType::Wire => &assets.tex_wire,
                ItemType::Circuit => &assets.tex_circuit,
                ItemType::Battery => &assets.tex_battery_item,
                ItemType::PowerGridComponent => &assets.tex_power_grid,
                ItemType::ConveyorComponent => &assets.tex_conveyor_item,
                ItemType::AssemblerComponent => &assets.tex_assembler_component,
            };
            gl.bind_texture(GL::TEXTURE_2D, Some(tex));
            let [x, y] = if 2 <= t.path.len() {
                lerp(pos, t.path[t.path.len() - 2], *frac_frame)
            } else {
                [pos[0] as f64, pos[1] as f64]
            };
            let x = (x + offset[0] as f64 / TILE_SIZE) as f32;
            let y = (y + offset[1] as f64 / TILE_SIZE) as f32;
            let transform = to_screen
                * scale
                * Matrix4::from_translation(Vector3::new(x, y, 0.))
                * Matrix4::from_translation(Vector3::new(0.5, 0.5, 0.))
                * Matrix4::from_scale(0.5)
                * Matrix4::from_translation(Vector3::new(-0.5, -0.5, 0.));
            gl.uniform_matrix4fv_with_f32_array(
                shader.transform_loc.as_ref(),
                false,
                transform.flatten(),
            );
            gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
        };

        for t in self.game.iter_transport() {
            // context.set_stroke_style(&JsValue::from("#ffff00"));
            // context.set_line_width(2.);
            // context.begin_path();
            // for node in &t.path {
            //     context.line_to(
            //         (node[0] as f64 + 0.5) * TILE_SIZE + offset[0],
            //         (node[1] as f64 + 0.5) * TILE_SIZE + offset[1],
            //     );
            // }
            // context.stroke();
            render_transport(&t);
        }
    }
}
