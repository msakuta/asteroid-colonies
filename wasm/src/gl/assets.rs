use crate::console_log;

use super::{
    shader_bundle::ShaderBundle,
    utils::{load_texture, vertex_buffer_data},
};
use asteroid_colonies_logic::building::BuildingType;
use slice_of_array::prelude::SliceFlatExt;
use wasm_bindgen::{prelude::*, JsCast, JsValue};
use web_sys::{
    ImageBitmap, WebGlBuffer, WebGlProgram, WebGlRenderingContext as GL, WebGlShader, WebGlTexture,
};

pub(crate) const MAX_SPRITES: usize = 512;
pub(crate) const SPRITE_COMPONENTS: usize = 4;
pub(crate) const WIRE_SEGMENTS: i32 = 10;
pub(crate) const HARVESTING_SEGMENTS: usize = 20;
pub(crate) const HARVESTING_THICKNESS: f32 = 0.1;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = ANGLEInstancedArrays)]
    pub(crate) type AngleInstancedArrays;

    #[wasm_bindgen(method, getter, js_name = VERTEX_ATTRIB_ARRAY_DIVISOR_ANGLE)]
    pub(crate) fn vertex_attrib_array_divisor_angle(this: &AngleInstancedArrays) -> i32;

    #[wasm_bindgen(method, catch, js_name = drawArraysInstancedANGLE)]
    pub(crate) fn draw_arrays_instanced_angle(
        this: &AngleInstancedArrays,
        mode: u32,
        first: i32,
        count: i32,
        primcount: i32,
    ) -> Result<(), JsValue>;

    // TODO offset should be i64
    #[wasm_bindgen(method, catch, js_name = drawElementsInstancedANGLE)]
    pub(crate) fn draw_elements_instanced_angle(
        this: &AngleInstancedArrays,
        mode: u32,
        count: i32,
        type_: u32,
        offset: i32,
        primcount: i32,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(method, js_name = vertexAttribDivisorANGLE)]
    pub(crate) fn vertex_attrib_divisor_angle(
        this: &AngleInstancedArrays,
        index: u32,
        divisor: u32,
    );
}

pub(crate) struct Assets {
    pub instanced_arrays_ext: Option<AngleInstancedArrays>,

    pub tex_back: WebGlTexture,
    pub tex_cursor: WebGlTexture,
    pub tex_move_cursor: WebGlTexture,
    pub tex_crew: WebGlTexture,
    pub tex_power_grid: WebGlTexture,
    pub tex_conveyor: WebGlTexture,
    pub tex_atomic_battery: WebGlTexture,
    pub tex_battery: WebGlTexture,
    pub tex_storage: WebGlTexture,
    pub tex_medium_storage: WebGlTexture,
    pub tex_crew_cabin: WebGlTexture,
    pub tex_excavator: WebGlTexture,
    pub tex_assembler: WebGlTexture,
    pub tex_furnace: WebGlTexture,
    pub tex_raw_ore: WebGlTexture,
    pub tex_iron_ingot: WebGlTexture,
    pub tex_copper_ingot: WebGlTexture,
    pub tex_lithium_ingot: WebGlTexture,
    pub tex_cilicate: WebGlTexture,
    pub tex_gear: WebGlTexture,
    pub tex_wire: WebGlTexture,
    pub tex_circuit: WebGlTexture,
    pub tex_battery_item: WebGlTexture,
    pub tex_conveyor_item: WebGlTexture,
    pub tex_assembler_component: WebGlTexture,
    pub tex_construction: WebGlTexture,
    pub tex_deconstruction: WebGlTexture,
    pub tex_cleanup: WebGlTexture,

    pub flat_shader: Option<ShaderBundle>,
    pub textured_shader: Option<ShaderBundle>,
    pub textured_instancing_shader: Option<ShaderBundle>,
    pub textured_alpha_shader: Option<ShaderBundle>,

    pub screen_buffer: Option<WebGlBuffer>,
    pub rect_buffer: Option<WebGlBuffer>,
    pub cursor_buffer: Option<WebGlBuffer>,
    pub wire_buffer: Option<WebGlBuffer>,
    pub harvesting_buffer: Option<WebGlBuffer>,

    pub sprites_buffer: Option<WebGlBuffer>,
}

impl Assets {
    pub fn new(context: &GL, image_assets: js_sys::Array) -> Result<Self, JsValue> {
        let load_texture_local = |path| -> Result<WebGlTexture, JsValue> {
            if let Some(value) = image_assets.iter().find(|value| {
                let array = js_sys::Array::from(value);
                array.iter().next() == Some(JsValue::from_str(path))
            }) {
                let array = js_sys::Array::from(&value).to_vec();
                let ret = load_texture(
                    &context,
                    array
                        .get(3)
                        .cloned()
                        .ok_or_else(|| {
                            JsValue::from_str(&format!(
                                "Couldn't convert value to ImageBitmap: {:?}",
                                path
                            ))
                        })?
                        .dyn_into::<ImageBitmap>()?,
                );
                console_log!("Loaded {}", path);
                ret
            } else {
                Err(JsValue::from_str("Couldn't find texture"))
            }
        };

        Ok(Assets {
            instanced_arrays_ext: None,
            tex_back: load_texture_local("bg32")?,
            tex_cursor: load_texture_local("cursor")?,
            tex_move_cursor: load_texture_local("move_cursor")?,
            tex_crew: load_texture_local("crew")?,
            tex_power_grid: load_texture_local("power_grid")?,
            tex_conveyor: load_texture_local("conveyor")?,
            tex_atomic_battery: load_texture_local("atomic_battery")?,
            tex_battery: load_texture_local("battery")?,
            tex_storage: load_texture_local("storage")?,
            tex_medium_storage: load_texture_local("medium_storage")?,
            tex_crew_cabin: load_texture_local("crew_cabin")?,
            tex_excavator: load_texture_local("excavator")?,
            tex_assembler: load_texture_local("assembler")?,
            tex_furnace: load_texture_local("furnace")?,
            tex_raw_ore: load_texture_local("raw_ore")?,
            tex_iron_ingot: load_texture_local("iron_ingot")?,
            tex_copper_ingot: load_texture_local("copper_ingot")?,
            tex_lithium_ingot: load_texture_local("lithium_ingot")?,
            tex_cilicate: load_texture_local("cilicate")?,
            tex_gear: load_texture_local("gear")?,
            tex_wire: load_texture_local("wire")?,
            tex_circuit: load_texture_local("circuit")?,
            tex_battery_item: load_texture_local("battery_item")?,
            tex_conveyor_item: load_texture_local("conveyor_item")?,
            tex_assembler_component: load_texture_local("assembler_component")?,
            tex_construction: load_texture_local("construction")?,
            tex_deconstruction: load_texture_local("deconstruction")?,
            tex_cleanup: load_texture_local("cleanup")?,

            flat_shader: None,
            textured_shader: None,
            textured_instancing_shader: None,
            textured_alpha_shader: None,
            screen_buffer: None,
            rect_buffer: None,
            cursor_buffer: None,
            wire_buffer: None,
            harvesting_buffer: None,
            sprites_buffer: None,
        })
    }

    pub fn building_to_tex(&self, ty: BuildingType) -> Option<&WebGlTexture> {
        Some(match ty {
            BuildingType::Power => &self.tex_atomic_battery,
            BuildingType::Battery => &self.tex_battery,
            BuildingType::Excavator => &self.tex_excavator,
            BuildingType::Storage => &self.tex_storage,
            BuildingType::MediumStorage => &self.tex_medium_storage,
            BuildingType::CrewCabin => &self.tex_crew_cabin,
            BuildingType::Assembler => &self.tex_assembler,
            BuildingType::Furnace => &self.tex_furnace,
            _ => panic!("Uncovered building type!"),
        })
    }

    pub(crate) fn prepare(&mut self, gl: &GL) -> Result<(), JsValue> {
        self.instanced_arrays_ext = gl
            .get_extension("ANGLE_instanced_arrays")
            .unwrap_or(None)
            .map(|v| v.unchecked_into::<AngleInstancedArrays>());
        console_log!(
            "WebGL Instanced arrays is {}",
            if self.instanced_arrays_ext.is_some() {
                "available"
            } else {
                "not available"
            }
        );

        // let vert_shader = compile_shader(
        //     &context,
        //     GL::VERTEX_SHADER,
        //     r#"
        //     attribute vec2 vertexData;
        //     uniform mat4 transform;
        //     uniform mat3 texTransform;
        //     varying vec2 texCoords;
        //     void main() {
        //         gl_Position = transform * vec4(vertexData.xy, 0.01, 1.0);

        //         texCoords = (texTransform * vec3((vertexData.xy - 1.) * 0.5, 1.)).xy;
        //     }
        // "#,
        // )?;
        // let frag_shader = compile_shader(
        //     &context,
        //     GL::FRAGMENT_SHADER,
        //     r#"
        //     precision mediump float;

        //     varying vec2 texCoords;

        //     uniform sampler2D texture;
        //     uniform float alpha;

        //     void main() {
        //         // vec4 texColor = texture2D( texture, vec2(texCoords.x, texCoords.y) );
        //         // gl_FragColor = vec4(texColor.rgb, texColor.a * alpha);
        //         gl_FragColor = vec4(1, 1, 1, 1);
        //     }
        // "#,
        // )?;
        // let program = link_program(&context, &vert_shader, &frag_shader)?;
        // context.use_program(Some(&program));

        // let shader = ShaderBundle::new(&context, program);

        // context.active_texture(GL::TEXTURE0);

        // context.uniform1i(shader.texture_loc.as_ref(), 0);
        // context.uniform1f(shader.alpha_loc.as_ref(), 1.);

        let vert_shader = compile_shader(
            &gl,
            GL::VERTEX_SHADER,
            r#"
            attribute vec2 vertexData;
            uniform mat4 transform;
            void main() {
                gl_Position = transform * vec4(vertexData.xy, 0., 1.0);
            }
        "#,
        )?;
        let frag_shader = compile_shader(
            &gl,
            GL::FRAGMENT_SHADER,
            r#"
            precision mediump float;
            uniform vec4 color;

            void main() {
                gl_FragColor = color;
            }
        "#,
        )?;
        let program = link_program(&gl, &vert_shader, &frag_shader)?;
        self.flat_shader = Some(ShaderBundle::new(&gl, program.clone()));
        gl.use_program(Some(&program));
        gl.uniform4f(
            self.flat_shader.as_ref().unwrap().color_loc.as_ref(),
            0.5,
            0.,
            0.,
            1.,
        );
        console_log!(
            "program: {}",
            program == self.flat_shader.as_ref().unwrap().program
        );

        gl.enable(GL::BLEND);
        gl.blend_equation(GL::FUNC_ADD);
        gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);

        // self.assets.sprite_shader = Some(shader);

        let vert_shader = compile_shader(
            &gl,
            GL::VERTEX_SHADER,
            r#"
            attribute vec2 vertexData;
            uniform mat4 transform;
            uniform mat3 texTransform;
            varying vec2 texCoords;
            void main() {
                gl_Position = transform * vec4(vertexData.xy, 0., 1.0);

                texCoords = (texTransform * vec3(vertexData.xy, 1.)).xy;
            }
        "#,
        )?;
        let frag_shader = compile_shader(
            &gl,
            GL::FRAGMENT_SHADER,
            r#"
            precision mediump float;

            varying vec2 texCoords;

            uniform sampler2D texture;
            uniform float alpha;

            void main() {
                vec4 texColor = texture2D( texture, texCoords.xy );
                gl_FragColor = vec4(texColor.rgb, texColor.a * alpha);
                if(gl_FragColor.a < 0.01)
                    discard;
            }
        "#,
        )?;
        let program = link_program(&gl, &vert_shader, &frag_shader)?;
        gl.use_program(Some(&program));
        console_log!("ShaderBundle textured_shader:");
        self.textured_shader = Some(ShaderBundle::new(&gl, program));

        gl.uniform1f(
            self.textured_shader
                .as_ref()
                .and_then(|s| s.alpha_loc.as_ref()),
            1.,
        );

        gl.active_texture(GL::TEXTURE0);
        gl.uniform1i(
            self.textured_shader
                .as_ref()
                .and_then(|s| s.texture_loc.as_ref()),
            0,
        );

        let vert_shader_instancing = compile_shader(
            &gl,
            GL::VERTEX_SHADER,
            r#"
            attribute vec2 vertexData;
            attribute vec4 position;
            // attribute float alpha;
            uniform mat4 transform;
            uniform mat3 texTransform;
            varying vec2 texCoords;
            // varying float alphaVar;

            void main() {
                mat4 centerize = mat4(
                    4, 0, 0, 0,
                    0, -4, 0, 0,
                    0, 0, 4, 0,
                    -1, 1, -1, 1);
                gl_Position = /*centerize **/ (transform * (vec4(vertexData.xy, 0.0, 1.0) + vec4(position.xy, 0.0, 1.0)));
                texCoords = (texTransform * vec3(
                    vertexData.xy + vec2(position.z, position.w), 1.)).xy;
                // alphaVar = alpha;
            }
        "#,
        )?;
        let frag_shader_instancing = compile_shader(
            &gl,
            GL::FRAGMENT_SHADER,
            r#"
            precision mediump float;

            varying vec2 texCoords;
            // varying float alphaVar;

            uniform sampler2D texture;

            void main() {
                vec4 texColor = texture2D( texture, vec2(texCoords.x, texCoords.y) );
                gl_FragColor = texColor;
                if(gl_FragColor.a < 0.5)
                    discard;
            }
        "#,
        )?;
        let program = link_program(&gl, &vert_shader_instancing, &frag_shader_instancing)?;
        let shader = ShaderBundle::new(&gl, program);
        self.textured_instancing_shader = Some(shader);

        let vert_shader = compile_shader(
            &gl,
            GL::VERTEX_SHADER,
            r#"
            attribute vec2 vertexData;
            uniform mat4 transform;
            uniform mat3 texTransform;
            varying vec2 texCoords;
            void main() {
                gl_Position = transform * vec4(vertexData.xy, 0., 1.0);

                texCoords = (texTransform * vec3(vertexData.xy, 1.)).xy;
            }
        "#,
        )?;
        let frag_shader = compile_shader(
            &gl,
            GL::FRAGMENT_SHADER,
            r#"
            precision mediump float;

            varying vec2 texCoords;

            uniform float alpha;
            uniform sampler2D texture;

            void main() {
                vec4 texColor = texture2D( texture, texCoords.xy );
                float prodAlpha = texColor.a * alpha;
                gl_FragColor = vec4(texColor.rgb, prodAlpha);
            }
        "#,
        )?;
        let program = link_program(&gl, &vert_shader, &frag_shader)?;
        gl.use_program(Some(&program));
        self.textured_alpha_shader = Some(ShaderBundle::new(&gl, program));

        gl.active_texture(GL::TEXTURE0);
        gl.uniform1i(
            self.textured_alpha_shader
                .as_ref()
                .and_then(|s| s.texture_loc.as_ref()),
            0,
        );

        self.rect_buffer = Some(gl.create_buffer().ok_or("failed to create buffer")?);
        gl.bind_buffer(GL::ARRAY_BUFFER, self.rect_buffer.as_ref());
        let rect_vertices: [f32; 8] = [1., 1., -1., 1., -1., -1., 1., -1.];
        vertex_buffer_data(&gl, &rect_vertices);

        self.screen_buffer = Some(gl.create_buffer().ok_or("failed to create buffer")?);
        gl.bind_buffer(GL::ARRAY_BUFFER, self.screen_buffer.as_ref());
        let rect_vertices: [f32; 8] = [1., 1., 0., 1., 0., 0., 1., 0.];
        vertex_buffer_data(&gl, &rect_vertices);

        self.sprites_buffer = Some(gl.create_buffer().ok_or("failed to create buffer")?);
        gl.bind_buffer(GL::ARRAY_BUFFER, self.sprites_buffer.as_ref());
        gl.buffer_data_with_i32(
            GL::ARRAY_BUFFER,
            (MAX_SPRITES * SPRITE_COMPONENTS * std::mem::size_of::<f32>()) as i32,
            GL::DYNAMIC_DRAW,
        );

        self.cursor_buffer = Some(gl.create_buffer().ok_or("failed to create buffer")?);
        gl.bind_buffer(GL::ARRAY_BUFFER, self.cursor_buffer.as_ref());
        const CURSOR_LINE_WIDTH: f32 = 0.05;
        let rect_vertices = [
            [1. + CURSOR_LINE_WIDTH, 1. + CURSOR_LINE_WIDTH],
            [1. - CURSOR_LINE_WIDTH, 1. - CURSOR_LINE_WIDTH],
            [-(1. + CURSOR_LINE_WIDTH), 1. + CURSOR_LINE_WIDTH],
            [-(1. - CURSOR_LINE_WIDTH), 1. - CURSOR_LINE_WIDTH],
            [-(1. + CURSOR_LINE_WIDTH), -(1. + CURSOR_LINE_WIDTH)],
            [-(1. - CURSOR_LINE_WIDTH), -(1. - CURSOR_LINE_WIDTH)],
            [1. + CURSOR_LINE_WIDTH, -(1. + CURSOR_LINE_WIDTH)],
            [1. - CURSOR_LINE_WIDTH, -(1. - CURSOR_LINE_WIDTH)],
            [1. + CURSOR_LINE_WIDTH, 1. + CURSOR_LINE_WIDTH],
            [1. - CURSOR_LINE_WIDTH, 1. - CURSOR_LINE_WIDTH],
        ];
        vertex_buffer_data(&gl, rect_vertices.flat());

        self.wire_buffer = Some(gl.create_buffer().ok_or("failed to create buffer")?);
        gl.bind_buffer(GL::ARRAY_BUFFER, self.wire_buffer.as_ref());
        gl.buffer_data_with_i32(GL::ARRAY_BUFFER, (WIRE_SEGMENTS + 1) * 4, GL::DYNAMIC_DRAW);

        self.harvesting_buffer = Some(gl.create_buffer().ok_or("failed to create buffer")?);
        gl.bind_buffer(GL::ARRAY_BUFFER, self.harvesting_buffer.as_ref());
        gl.buffer_data_with_i32(
            GL::ARRAY_BUFFER,
            ((HARVESTING_SEGMENTS + 1) * std::mem::size_of::<f32>() * 4) as i32,
            GL::DYNAMIC_DRAW,
        );

        gl.clear_color(0.0, 0.0, 0.5, 1.);

        Ok(())
    }
}

pub fn compile_shader(context: &GL, shader_type: u32, source: &str) -> Result<WebGlShader, String> {
    let shader = context
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    context.shader_source(&shader, source);
    context.compile_shader(&shader);

    if context
        .get_shader_parameter(&shader, GL::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(context
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| String::from("Unknown error creating shader")))
    }
}

pub fn link_program(
    context: &GL,
    vert_shader: &WebGlShader,
    frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let program = context
        .create_program()
        .ok_or_else(|| String::from("Unable to create shader object"))?;

    context.attach_shader(&program, vert_shader);
    context.attach_shader(&program, frag_shader);
    context.link_program(&program);

    if context
        .get_program_parameter(&program, GL::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(context
            .get_program_info_log(&program)
            .unwrap_or_else(|| String::from("Unknown error creating program object")))
    }
}
