use web_sys::{WebGlProgram, WebGlRenderingContext as GL, WebGlUniformLocation};

use crate::console_log;

pub(crate) struct ShaderBundle<L = ()> {
    pub program: WebGlProgram,
    pub vertex_position: u32,
    #[allow(dead_code)] // May use later
    pub tex_coord_position: u32,
    pub texture_loc: Option<WebGlUniformLocation>,
    pub texture2_loc: Option<WebGlUniformLocation>,
    pub transform_loc: Option<WebGlUniformLocation>,
    pub tex_transform_loc: Option<WebGlUniformLocation>,
    pub width_scale_loc: Option<WebGlUniformLocation>,
    pub height_scale_loc: Option<WebGlUniformLocation>,
    #[allow(dead_code)] // May use later
    pub attrib_position_loc: i32,
    #[allow(dead_code)] // May use later
    pub alpha_loc: Option<WebGlUniformLocation>,
    pub color_loc: Option<WebGlUniformLocation>,
    /// Extra locations. It could be a `HashMap<String, GLuint>`, but it would be more
    /// efficient to statically know the list of locations.
    pub locations: L,
}

impl<L: GetLocations> ShaderBundle<L> {
    pub fn new(gl: &GL, program: WebGlProgram) -> Self {
        let get_uniform = |location: &str| {
            let op: Option<WebGlUniformLocation> = gl.get_uniform_location(&program, location);
            if op.is_none() {
                console_log!("Warning: location {} undefined", location);
            } else {
                console_log!("location {} defined", location);
            }
            op
        };
        let vertex_position = gl.get_attrib_location(&program, "vertexData") as u32;
        let tex_coord_position = gl.get_attrib_location(&program, "texCoord") as u32;
        console_log!("vertex_position: {}", vertex_position);
        console_log!("tex_coord_position: {}", tex_coord_position);
        let attrib_position_loc = gl.get_attrib_location(&program, "position");
        Self {
            vertex_position,
            tex_coord_position,
            texture_loc: get_uniform("texture"),
            texture2_loc: get_uniform("texture2"),
            transform_loc: get_uniform("transform"),
            tex_transform_loc: get_uniform("texTransform"),
            alpha_loc: get_uniform("alpha"),
            color_loc: get_uniform("color"),
            width_scale_loc: get_uniform("widthScale"),
            height_scale_loc: get_uniform("heightScale"),
            attrib_position_loc,
            locations: L::get_locations(gl, &program),
            // Program has to be later than others
            program,
        }
    }
}

/// A trait representing a set of shader locations, which knows how to retrieve them
/// from a compiled shader program.
pub trait GetLocations {
    fn get_locations(gl: &GL, program: &WebGlProgram) -> Self;
}

impl GetLocations for () {
    fn get_locations(_: &GL, _: &WebGlProgram) -> Self {
        ()
    }
}
