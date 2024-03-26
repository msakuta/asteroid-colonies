use std::cell::Cell;

use crate::console_log;

use super::{
    shader_bundle::ShaderBundle,
    utils::{create_texture, load_texture, vertex_buffer_data},
};
use asteroid_colonies_logic::{building::BuildingType, ItemType};

use wasm_bindgen::{prelude::*, JsCast, JsValue};
use web_sys::{
    ImageBitmap, WebGlBuffer, WebGlProgram, WebGlRenderingContext as GL, WebGlShader, WebGlTexture,
};

pub(crate) const MAX_SPRITES: usize = 512;
pub(crate) const SPRITE_COMPONENTS: usize = 4;

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
    /// Instanced array extension, may use later
    pub _instanced_arrays_ext: Option<AngleInstancedArrays>,

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
    pub tex_excavate: WebGlTexture,
    pub tex_path: WebGlTexture,

    /// A special texture that holds tile types in the map.
    /// The shader will use it as a lookup table to quickly render
    /// many tiles with mixed types.
    pub tex_bg_sampler: WebGlTexture,
    /// A buffer to hold the type indices of the tiles.
    /// Stored in CPU memory to check if there is any change.
    /// If there was, the buffer data is transferred to the GPU texture memory.
    pub bg_sampler_buf: Cell<Vec<u8>>,

    pub flat_shader: ShaderBundle,
    pub textured_shader: ShaderBundle,
    pub multi_textured_shader: ShaderBundle,
    pub vertex_textured_shader: ShaderBundle,
    /// Textured instancing shader, may use later
    pub _textured_instancing_shader: Option<ShaderBundle>,

    pub screen_buffer: WebGlBuffer,
    // pub rect_buffer: WebGlBuffer,
    pub path_buffer: WebGlBuffer,

    pub _sprites_buffer: WebGlBuffer,
}

impl Assets {
    pub fn new(gl: &GL, image_assets: js_sys::Array) -> Result<Self, JsValue> {
        let load_texture_local = |path| -> Result<WebGlTexture, JsValue> {
            if let Some(value) = image_assets.iter().find(|value| {
                let array = js_sys::Array::from(value);
                array.iter().next() == Some(JsValue::from_str(path))
            }) {
                let array = js_sys::Array::from(&value).to_vec();
                let ret = load_texture(
                    &gl,
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

        let instanced_arrays_ext = gl
            .get_extension("ANGLE_instanced_arrays")
            .unwrap_or(None)
            .map(|v| v.unchecked_into::<AngleInstancedArrays>());

        console_log!(
            "WebGL Instanced arrays is {}",
            if instanced_arrays_ext.is_some() {
                "available"
            } else {
                "not available"
            }
        );

        let (textured_shader, vert_shader, _frag_shader) = make_textured_shader(gl)?;

        let screen_buffer = gl.create_buffer().ok_or("failed to create buffer")?;
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&screen_buffer));
        let rect_vertices: [f32; 8] = [1., 1., 0., 1., 0., 0., 1., 0.];
        vertex_buffer_data(&gl, &rect_vertices);

        // let rect_buffer = gl.create_buffer().ok_or("failed to create buffer")?;
        // gl.bind_buffer(GL::ARRAY_BUFFER, Some(&rect_buffer));
        // let rect_vertices: [f32; 8] = [1., 1., -1., 1., -1., -1., 1., -1.];
        // vertex_buffer_data(&gl, &rect_vertices);

        let path_buffer = gl.create_buffer().ok_or("failed to create buffer")?;
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&path_buffer));

        let sprites_buffer = gl.create_buffer().ok_or("failed to create buffer")?;
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&sprites_buffer));
        gl.buffer_data_with_i32(
            GL::ARRAY_BUFFER,
            (MAX_SPRITES * SPRITE_COMPONENTS * std::mem::size_of::<f32>()) as i32,
            GL::DYNAMIC_DRAW,
        );

        gl.enable(GL::BLEND);
        gl.blend_equation(GL::FUNC_ADD);
        gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);

        gl.clear_color(0.0, 0.0, 0.5, 1.);

        Ok(Assets {
            _instanced_arrays_ext: instanced_arrays_ext,
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
            tex_excavate: load_texture_local("excavate")?,
            tex_path: load_texture_local("path")?,

            tex_bg_sampler: create_texture(gl, 128)?,
            bg_sampler_buf: Cell::new(vec![]),

            flat_shader: make_flat_shader(gl)?,
            textured_shader,
            multi_textured_shader: make_multitex_shader(gl, &vert_shader)?,
            vertex_textured_shader: make_vertex_textured_shader(gl)?,
            _textured_instancing_shader: make_instancing_shader(gl).ok(),
            screen_buffer,
            // rect_buffer,
            path_buffer,
            _sprites_buffer: sprites_buffer,
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

    pub fn item_to_tex(&self, item: ItemType) -> &WebGlTexture {
        match item {
            ItemType::RawOre => &self.tex_raw_ore,
            ItemType::IronIngot => &self.tex_iron_ingot,
            ItemType::CopperIngot => &self.tex_copper_ingot,
            ItemType::LithiumIngot => &self.tex_lithium_ingot,
            ItemType::Cilicate => &self.tex_cilicate,
            ItemType::Gear => &self.tex_gear,
            ItemType::Wire => &self.tex_wire,
            ItemType::Circuit => &self.tex_circuit,
            ItemType::Battery => &self.tex_battery_item,
            ItemType::PowerGridComponent => &self.tex_power_grid,
            ItemType::ConveyorComponent => &self.tex_conveyor_item,
            ItemType::AssemblerComponent => &self.tex_assembler_component,
        }
    }
}

fn make_flat_shader(gl: &GL) -> Result<ShaderBundle, JsValue> {
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
    Ok(ShaderBundle::new(&gl, program))
}

fn make_textured_shader(gl: &GL) -> Result<(ShaderBundle, WebGlShader, WebGlShader), JsValue> {
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
    let shader = ShaderBundle::new(&gl, program);

    gl.uniform1f(shader.alpha_loc.as_ref(), 1.);

    gl.active_texture(GL::TEXTURE0);
    gl.uniform1i(shader.texture_loc.as_ref(), 0);

    Ok((shader, vert_shader, frag_shader))
}

fn make_multitex_shader(gl: &GL, vert_shader: &WebGlShader) -> Result<ShaderBundle, JsValue> {
    let frag_shader = compile_shader(
        &gl,
        GL::FRAGMENT_SHADER,
        r#"
        precision mediump float;

        varying vec2 texCoords;

        uniform sampler2D texture;
        uniform sampler2D texture2;
        uniform float alpha;
        uniform float widthScale;
        uniform float heightScale;
        const float sampleSize = 128.;
        // Margin is a way to work around border artifacts between tiles
        const float margin = 1. / 32.;
        const float marginDiscard = 30. / 32.;

        void main() {
            float x = texCoords.x;
            float xi = floor(x * sampleSize) / sampleSize;
            float xf = (x - xi) * sampleSize;
            float y = texCoords.y;
            float yi = floor(y * sampleSize) / sampleSize;
            float yf = (y - yi) * sampleSize;
            if(xi < 0. || 1. < xi || yi < 0. || 1. < yi){
                gl_FragColor = vec4(0., 0., 0., 1.);
                return;
            }
            vec4 first = texture2D( texture2, vec2(xi, yi) );
            vec4 texColor = texture2D( texture, vec2(
                (xf + margin) * marginDiscard * widthScale,
                ((yf + margin) * marginDiscard + first[0] * 2.) * heightScale) );
            gl_FragColor = texColor;
            // gl_FragColor = vec4(texColor.rgb, texColor.a * alpha);
            if(gl_FragColor.a < 0.01)
                discard;
        }
    "#,
    )?;
    let program = link_program(&gl, vert_shader, &frag_shader)?;
    gl.use_program(Some(&program));
    console_log!("ShaderBundle multi_textured_shader:");
    let shader = ShaderBundle::new(&gl, program);

    gl.uniform1f(shader.width_scale_loc.as_ref(), 1. / 4.);

    gl.uniform1f(shader.height_scale_loc.as_ref(), 1. / 8.);

    Ok(shader)
}

fn make_vertex_textured_shader(gl: &GL) -> Result<ShaderBundle, String> {
    let vert_shader = compile_shader(
        &gl,
        GL::VERTEX_SHADER,
        r#"
        attribute vec2 vertexData;
        attribute vec2 texCoord;
        uniform mat4 transform;
        uniform mat3 texTransform;
        varying vec2 texCoords;
        void main() {
            gl_Position = transform * vec4(vertexData.xy, 0., 1.0);

            texCoords = (texTransform * vec3(texCoord.xy, 1.)).xy;
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
        uniform vec4 color;

        void main() {
            vec4 texColor = texture2D( texture, texCoords.xy );
            gl_FragColor = color * vec4(texColor.rgb, texColor.a);
            if(gl_FragColor.a < 0.01)
                discard;
        }
    "#,
    )?;
    let program = link_program(&gl, &vert_shader, &frag_shader)?;
    gl.use_program(Some(&program));
    console_log!("ShaderBundle multi_textured_shader:");
    let shader = ShaderBundle::new(&gl, program);

    gl.uniform1f(shader.width_scale_loc.as_ref(), 1. / 4.);

    gl.uniform1f(shader.height_scale_loc.as_ref(), 1. / 8.);

    Ok(shader)
}

fn make_instancing_shader(gl: &GL) -> Result<ShaderBundle, String> {
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
    Ok(ShaderBundle::new(&gl, program))
}

fn _make_sprite_shader(gl: &GL) -> Result<ShaderBundle, String> {
    let vert_shader = compile_shader(
        &gl,
        GL::VERTEX_SHADER,
        r#"
            attribute vec2 vertexData;
            uniform mat4 transform;
            uniform mat3 texTransform;
            varying vec2 texCoords;
            void main() {
                gl_Position = transform * vec4(vertexData.xy, 0.01, 1.0);

                texCoords = (texTransform * vec3((vertexData.xy - 1.) * 0.5, 1.)).xy;
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
                // vec4 texColor = texture2D( texture, vec2(texCoords.x, texCoords.y) );
                // gl_FragColor = vec4(texColor.rgb, texColor.a * alpha);
                gl_FragColor = vec4(1, 1, 1, 1);
            }
        "#,
    )?;
    let program = link_program(&gl, &vert_shader, &frag_shader)?;
    gl.use_program(Some(&program));

    let shader = ShaderBundle::new(&gl, program);

    gl.active_texture(GL::TEXTURE0);

    gl.uniform1i(shader.texture_loc.as_ref(), 0);
    gl.uniform1f(shader.alpha_loc.as_ref(), 1.);
    Ok(shader)
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
