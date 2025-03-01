use std::fs::File;

use super::State;

use std::io::Read;

use anyhow::{anyhow, Context, Error, Result};
use eframe::glow::NativeShader;
use egui::PaintCallbackInfo;

pub struct FractalGl {
    program: eframe::glow::Program,
    vertex_array: eframe::glow::VertexArray,
}

impl FractalGl {
    pub fn new(gl: &eframe::glow::Context) -> Result<Self> {
        use eframe::glow::HasContext as _;
        unsafe {
            let program = gl
                .create_program()
                .map_err(|e| anyhow!("Cannot create program: {}", e))?;

            let mut vertex_shader_source = String::new();
            File::open("assets/vertex.shader")
                .and_then(|mut x| x.read_to_string(&mut vertex_shader_source))
                .context("Cannot read the Vertex Shaders")?;

            let mut fragment_shader_source = String::new();
            File::open("assets/fragment.shader")
                .and_then(|mut x| x.read_to_string(&mut fragment_shader_source))
                .context("Cannot read the Fragment Shaders")?;

            let shader_sources = [
                (glow::VERTEX_SHADER, vertex_shader_source),
                (glow::FRAGMENT_SHADER, fragment_shader_source),
            ];

            let shaders: Vec<_> = shader_sources
                .iter()
                .map(|(shader_type, shader_source)| {
                    let shader = gl
                        .create_shader(*shader_type)
                        .map_err(|e| anyhow!("Cannot create shader: {}", e) as Error)?;
                    gl.shader_source(shader, &format!("{}\n{}", "#version 330", shader_source));
                    gl.compile_shader(shader);
                    assert!(
                        gl.get_shader_compile_status(shader),
                        "Failed to compile Shader {shader_type} - {}:\n{}",
                        gl.get_shader_info_log(shader),
                        shader_source
                    );
                    gl.attach_shader(program, shader);
                    Ok(shader)
                })
                .collect::<Result<Vec<NativeShader>>>()?;

            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!("{}", gl.get_program_info_log(program));
            }

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            let vertex_array = gl
                .create_vertex_array()
                .map_err(|e| anyhow!("Cannot create vertex array: {}", e))?;

            Ok(Self {
                program,
                vertex_array,
            })
        }
    }

    pub fn destroy(&self, gl: &eframe::glow::Context) {
        use eframe::glow::HasContext as _;
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vertex_array);
        }
    }

    pub fn paint(&self, gl: &eframe::glow::Context, state: State, paint_info: PaintCallbackInfo) {
        use eframe::glow::HasContext as _;
        unsafe {
            gl.use_program(Some(self.program));

            let mappings = [
                ("u_fractalZoom", state.zoom),
                ("u_brightness", state.brightness),
                ("u_gamma", state.gamma),
                ("u_contrast", state.contrast),
                ("u_r", state.r),
                ("u_g", state.g),
                ("u_b", state.b),
            ];

            for (label, value) in mappings.iter() {
                gl.uniform_1_f32(
                    gl.get_uniform_location(self.program, label).as_ref(),
                    *value,
                );
            }

            gl.uniform_1_i32(
                gl.get_uniform_location(self.program, "u_highQuality")
                    .as_ref(),
                if state.high_quality { 1 } else { 0 },
            );

            gl.uniform_1_i32(
                gl.get_uniform_location(self.program, "u_fractal_type")
                    .as_ref(),
                state.fractal_type as i32,
            );

            // Not happy about needing to call this method here and pass around the paint_info,
            // but ViewportInPixels (type of vieport) isn't publicly available so I couldn't find
            // a way to pass it as argument, and creating a whole new type was a bit overkill.
            let viewport = paint_info.viewport_in_pixels();

            let u_fractal_position = gl.get_uniform_location(self.program, "u_fractalPosition");
            gl.uniform_2_f32(
                u_fractal_position.as_ref(),
                // The viewport is
                state.center_position.x
                    + viewport.left_px as f32 / state.zoom // shift to skip the edge
                    + 0.5 * viewport.width_px as f32 / state.zoom, // shift to put the center_position in the middle
                state.center_position.y
                    + viewport.top_px as f32 / state.zoom
                    + 0.5 * viewport.height_px as f32 / state.zoom,
            );

            let c_julia = gl.get_uniform_location(self.program, "u_cJulia");
            gl.uniform_2_f32(c_julia.as_ref(), state.c_julia.x, state.c_julia.y);

            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_arrays(glow::TRIANGLES, 0, 6);
        }
    }
}
