use cgmath::{Matrix3, Matrix4};
use wasm_bindgen::JsValue;
use web_sys::{ImageBitmap, WebGlBuffer, WebGlRenderingContext as GL, WebGlTexture};

// pub(crate) fn get_context() -> Result<GL, JsValue> {
//     let document = document();
//     let canvas = document.get_element_by_id("canvas").unwrap();
//     let canvas: web_sys::HtmlCanvasElement =
//         canvas.dyn_into::<web_sys::HtmlCanvasElement>().unwrap();

//     Ok(canvas
//         .get_context("webgl")?
//         .ok_or_else(|| js_str!("no context"))?
//         .dyn_into::<GL>()?)
// }

/// Initialize a texture and load an image from ImageBitmap.
/// It is synchronous, meaning the image has to be loaded already.
pub(crate) fn load_texture(gl: &GL, bitmap: ImageBitmap) -> Result<WebGlTexture, JsValue> {
    fn is_power_of_2(value: u32) -> bool {
        (value & (value - 1)) == 0
    }
    let texture = gl.create_texture().unwrap();
    gl.bind_texture(GL::TEXTURE_2D, Some(&texture));

    let level = 0;
    let internal_format = GL::RGBA as i32;
    let src_format = GL::RGBA;
    let src_type = GL::UNSIGNED_BYTE;
    gl.tex_image_2d_with_u32_and_u32_and_image_bitmap(
        GL::TEXTURE_2D,
        level,
        internal_format,
        src_format,
        src_type,
        &bitmap,
    )?;
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::NEAREST as i32);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, GL::LINEAR as i32);

    // https://www.khronos.org/webgl/wiki/WebGL_and_OpenGL_Differences#Non-Power_of_Two_Texture_Support
    if is_power_of_2(bitmap.width()) && is_power_of_2(bitmap.height()) {
        // Yes, it's a power of 2. Generate mips.
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::REPEAT as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::REPEAT as i32);
        gl.generate_mipmap(GL::TEXTURE_2D);
    } else {
        // No, it's not a power of 2. Turn off mips and set
        // wrapping to clamp to edge
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, GL::NEAREST as i32);
    }

    Ok(texture)
}

pub fn create_texture(gl: &GL, size: usize, format: u32) -> Result<WebGlTexture, JsValue> {
    let texture = gl.create_texture().unwrap();
    gl.bind_texture(GL::TEXTURE_2D, Some(&texture));

    let pixel_size = match format {
        GL::LUMINANCE => 1,
        GL::RGB => 3,
        _ => todo!(),
    };
    let buf = vec![0u8; size * size * pixel_size];

    let level = 0;
    let internal_format = format as i32;
    let src_format = format;
    let src_type = GL::UNSIGNED_BYTE;
    gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        GL::TEXTURE_2D,
        level,
        internal_format,
        size as i32,
        size as i32,
        0,
        src_format,
        src_type,
        Some(&buf),
    )?;
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::NEAREST as i32);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, GL::LINEAR as i32);

    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::REPEAT as i32);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::REPEAT as i32);

    Ok(texture)
}

pub fn vertex_buffer_data(context: &GL, vertices: &[f32]) {
    // Note that `Float32Array::view` is somewhat dangerous (hence the
    // `unsafe`!). This is creating a raw view into our module's
    // `WebAssembly.Memory` buffer, but if we allocate more pages for ourself
    // (aka do a memory allocation in Rust) it'll cause the buffer to change,
    // causing the `Float32Array` to be invalid.
    //
    // As a result, after `Float32Array::view` we have to be very careful not to
    // do any memory allocations before it's dropped.
    unsafe {
        let vert_array = js_sys::Float32Array::view(vertices);

        context.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &vert_array, GL::STATIC_DRAW);
    };
}

pub fn _vertex_buffer_sub_data(context: &GL, vertices: &[f32]) {
    // Note that `Float32Array::view` is somewhat dangerous (hence the
    // `unsafe`!). This is creating a raw view into our module's
    // `WebAssembly.Memory` buffer, but if we allocate more pages for ourself
    // (aka do a memory allocation in Rust) it'll cause the buffer to change,
    // causing the `Float32Array` to be invalid.
    //
    // As a result, after `Float32Array::view` we have to be very careful not to
    // do any memory allocations before it's dropped.
    unsafe {
        let vert_array = js_sys::Float32Array::view(vertices);

        context.buffer_sub_data_with_i32_and_array_buffer_view(GL::ARRAY_BUFFER, 0, &vert_array);
    };
}

pub fn enable_buffer(gl: &GL, buffer: &WebGlBuffer, elements: i32, vertex_position: u32) {
    gl.bind_buffer(GL::ARRAY_BUFFER, Some(buffer));
    gl.vertex_attrib_pointer_with_i32(vertex_position, elements, GL::FLOAT, false, 0, 0);
    gl.enable_vertex_attrib_array(vertex_position);
}

/// An extension trait that returns flatten slice of a matrix from `cgmath`.
pub(crate) trait Flatten {
    /// Returns flattened slice `&[f32]` into the internal buffer of `&self`.
    /// Convenient for passing data to WebGL API.
    ///
    /// We could do the same with `<Matrix3<f32> as AsRef<[f32; 9]>>::as_ref(matrix)`,
    /// but it's repetitive and hard to read.
    /// Also, we can use method chaining that sometimes reduces indentation.
    fn flatten(&self) -> &[f32];
}

impl Flatten for Matrix3<f32> {
    fn flatten(&self) -> &[f32] {
        <Matrix3<f32> as AsRef<[f32; 9]>>::as_ref(self)
    }
}

impl Flatten for Matrix4<f32> {
    fn flatten(&self) -> &[f32] {
        <Matrix4<f32> as AsRef<[f32; 16]>>::as_ref(self)
    }
}
