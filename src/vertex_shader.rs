use bevy::render::render_resource::Shader;

/// Creates a vertex shader with the correct number of arguments
pub(crate) fn create_vertex_shader<const F_PARAMS: usize, const U_PARAMS: usize>() -> Shader {
    // TODO Create this string at compile time?

    let rotation_location = (F_PARAMS + U_PARAMS) + 2;
    let scale_location = (F_PARAMS + U_PARAMS) + 3;
    let frame_location = (F_PARAMS + U_PARAMS) + 4;

    let params_locations = format_params_locations::<F_PARAMS, U_PARAMS>();

    let mut params_assignments = "".to_string();
    for index in 0..F_PARAMS {
        params_assignments
            .push_str(format!("    out.param_f_{index} = vertex.param_f_{index};\n").as_str());
    }

    for index in 0..U_PARAMS{
        params_assignments
            .push_str(format!("    out.param_u_{index} = vertex.param_u_{index};\n").as_str());
    }

    let source = format!(
        r##"
#define_import_path smud::vertex_params_{F_PARAMS}_{U_PARAMS}

struct View {{
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
}};
@group(0) @binding(0)
var<uniform> view: View;

// as specified in `specialize()`
struct Vertex {{
@location(0) position: vec3<f32>,
@location(1) color: vec4<f32>,
{params_locations}
@location({rotation_location}) rotation: vec2<f32>,
@location({scale_location}) scale: f32,
@location({frame_location}) frame: f32,
}};

struct VertexOutput {{
@builtin(position) clip_position: vec4<f32>,
@location(0) color: vec4<f32>,
@location(1) pos: vec2<f32>,
{params_locations}
}};

@vertex
fn vertex(
    vertex: Vertex,
    @builtin(vertex_index) i: u32
) -> VertexOutput {{
var out: VertexOutput;
let x = select(-1., 1., i % 2u == 0u);
let y = select(-1., 1., (i / 2u) % 2u == 0u);
let c = vertex.rotation.x;
let s = vertex.rotation.y;
let rotated = vec2<f32>(x * c - y * s, x * s + y * c);
let pos = vertex.position + vec3<f32>(rotated * vertex.scale * vertex.frame, vertex.position.z);
// Project the world position of the mesh into screen position
out.clip_position = view.view_proj * vec4<f32>(pos, 1.);
out.color = vertex.color;
{params_assignments}
out.pos = vec2<f32>(x, y) * vertex.frame;
return out;
}}
"##
    );
    //bevy::log::info!(source);
    let path = file!();
    Shader::from_wgsl(source, path)
}

pub(crate) fn format_params_locations<const F_PARAMS: usize, const U_PARAMS: usize>() -> String {
    let mut result = "".to_string();
    for index in 0..F_PARAMS {
        result.push_str(
            format!("@location({loc}) param_f_{index}: f32,\n", loc = index + 2).as_str(),
        );
    }
    for index in 0..U_PARAMS {
        result.push_str(
            format!("@location({loc}) param_u_{index}: u32,\n", loc = index + F_PARAMS + 2).as_str(),
        );
    }
    result
}
