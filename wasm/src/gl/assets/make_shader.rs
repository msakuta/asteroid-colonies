//! A collection of shader programs
use wasm_bindgen::JsValue;
use web_sys::{WebGlRenderingContext as GL, WebGlShader, WebGlUniformLocation};

use crate::{console_log, gl::shader_bundle::GetLocations};

use super::{super::shader_bundle::ShaderBundle, compile_shader, link_program};

pub(super) fn make_flat_shader(gl: &GL) -> Result<ShaderBundle, JsValue> {
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

pub(super) fn make_textured_shader(
    gl: &GL,
) -> Result<(ShaderBundle, WebGlShader, WebGlShader), JsValue> {
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

pub(crate) struct BackgroundLocations {
    pub texture_ores: Option<WebGlUniformLocation>,
    pub draw_ore_overlay: Option<WebGlUniformLocation>,
}

impl GetLocations for BackgroundLocations {
    fn get_locations(gl: &GL, program: &web_sys::WebGlProgram) -> Self {
        let get_uniform = |location: &str| {
            let op: Option<WebGlUniformLocation> = gl.get_uniform_location(&program, location);
            if op.is_none() {
                console_log!("Warning: location {} undefined", location);
            } else {
                console_log!("location {} defined", location);
            }
            op
        };
        Self {
            texture_ores: get_uniform("textureOres"),
            draw_ore_overlay: get_uniform("drawOreOverlay"),
        }
    }
}

pub(crate) type BgShaderBundle = ShaderBundle<BackgroundLocations>;

pub(super) fn make_background_shader(
    gl: &GL,
    vert_shader: &WebGlShader,
) -> Result<BgShaderBundle, JsValue> {
    let frag_shader = compile_shader(
        &gl,
        GL::FRAGMENT_SHADER,
        r#"
        precision mediump float;

        varying vec2 texCoords;

        uniform sampler2D texture;
        uniform sampler2D texture2;
        uniform sampler2D textureOres;
        uniform float alpha;
        uniform float widthScale;
        uniform float heightScale;
        uniform bool drawOreOverlay;
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

            if (drawOreOverlay) {
                gl_FragColor = texColor * texture2D( textureOres, vec2(xi, yi) );
            }
            else {
                gl_FragColor = texColor;
            }

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

pub(super) fn make_vertex_textured_shader(gl: &GL) -> Result<ShaderBundle, String> {
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

pub(super) fn make_instancing_shader(gl: &GL) -> Result<ShaderBundle, String> {
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

pub(super) fn _make_sprite_shader(gl: &GL) -> Result<ShaderBundle, String> {
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
