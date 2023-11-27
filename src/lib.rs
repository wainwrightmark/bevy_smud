#![warn(missing_docs)]
#![doc = include_str!("../README.md")]
#![allow(clippy::too_many_arguments)]

use std::ops::Range;

use bevy::{
    core_pipeline::core_2d::Transparent2d,
    ecs::{
        query::ROQueryItem,
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        },
    },
    math::Vec3Swizzles,
    prelude::*,
    render::{
        globals::{GlobalsBuffer, GlobalsUniform},
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult,
            RenderPhase, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutDescriptor,
            BindGroupLayoutEntry, BindingType, BlendState, BufferBindingType, BufferUsages,
            BufferVec, CachedRenderPipelineId, ColorTargetState, ColorWrites, Face, FragmentState,
            FrontFace, MultisampleState, PipelineCache, PolygonMode, PrimitiveState,
            PrimitiveTopology, RenderPipelineDescriptor, ShaderImport, ShaderStages, ShaderType,
            SpecializedRenderPipeline, SpecializedRenderPipelines, TextureFormat, VertexAttribute,
            VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::BevyDefault,
        view::{
            ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms,
            VisibleEntities,
        },
        Extract, MainWorld, Render, RenderApp, RenderSet,
    },
    utils::{EntityHashMap, FloatOrd, HashMap},
};
use bytemuck::{Pod, Zeroable};
use fixedbitset::FixedBitSet;
use param_usage::ShaderParamUsage;
use shader_loading::*;
// use ui::UiShapePlugin;

pub use bundle::ShapeBundle;
pub use components::*;
pub use shader_loading::{DEFAULT_FILL_HANDLE, SIMPLE_FILL_HANDLE};

use crate::util::generate_shader_id;

mod bundle;
mod components;
pub mod param_usage;
mod sdf_assets;
mod shader_loading;
mod util;
mod vertex_shader;
// mod ui;

/// Re-export of the essentials needed for rendering shapes
///
/// Intended to be included at the top of your file to minimize the amount of import noise.
/// ```
/// use bevy_smud::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{
        sdf_assets::SdfAssets,
        Frame,
        ShapeBundle,
        SmudPlugin,
        SmudShape,
        // UiShapeBundle,
        DEFAULT_FILL_HANDLE,
        SIMPLE_FILL_HANDLE,
    };
}

#[derive(Default)]
/// Main plugin for enabling rendering of Sdf shapes
pub struct SmudPlugin<const PARAMS: usize>;

impl<const PARAMS: usize> Plugin for SmudPlugin<PARAMS> {
    fn build(&self, app: &mut App) {
        // All the messy boiler-plate for loading a bunch of shaders
        app.add_plugins(ShaderLoadingPlugin::<PARAMS>);
        // app.add_plugins(UiShapePlugin);

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Transparent2d, DrawSmudShape<PARAMS>>()
                .init_resource::<ExtractedShapes<PARAMS>>()
                .init_resource::<ShapeMeta<PARAMS>>()
                .init_resource::<SpecializedRenderPipelines<SmudPipeline<PARAMS>>>()
                .add_systems(
                    ExtractSchedule,
                    (extract_shapes::<PARAMS>, extract_sdf_shaders::<PARAMS>),
                )
                .add_systems(
                    Render,
                    (
                        queue_shapes::<PARAMS>.in_set(RenderSet::Queue),
                        prepare_shapes::<PARAMS>.in_set(RenderSet::PrepareBindGroups),
                    ),
                );
        }

        app.register_type::<SmudShape<PARAMS>>();
    }

    fn finish(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<SmudPipeline<PARAMS>>();
    }
}

type DrawSmudShape<const PARAMS: usize> = (
    SetItemPipeline,
    SetShapeViewBindGroup<0, PARAMS>,
    DrawShapeBatch<PARAMS>,
);

struct SetShapeViewBindGroup<const I: usize, const PARAMS: usize>;
impl<P: PhaseItem, const I: usize, const PARAMS: usize> RenderCommand<P>
    for SetShapeViewBindGroup<I, PARAMS>
{
    type Param = SRes<ShapeMeta<PARAMS>>;
    type ViewWorldQuery = Read<ViewUniformOffset>;
    type ItemWorldQuery = ();

    fn render<'w>(
        _item: &P,
        view_uniform: ROQueryItem<'w, Self::ViewWorldQuery>,
        _view: (),
        shape_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(
            I,
            shape_meta.into_inner().view_bind_group.as_ref().unwrap(),
            &[view_uniform.offset],
        );
        RenderCommandResult::Success
    }
}

struct DrawShapeBatch<const PARAMS: usize>;
impl<P: PhaseItem, const PARAMS: usize> RenderCommand<P> for DrawShapeBatch<PARAMS> {
    type Param = SRes<ShapeMeta<PARAMS>>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<ShapeBatch>;

    fn render<'w>(
        _item: &P,
        _view: (),
        batch: &'_ ShapeBatch,
        shape_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let shape_meta = shape_meta.into_inner();
        pass.set_vertex_buffer(0, shape_meta.vertices.buffer().unwrap().slice(..));
        pass.draw(0..4, batch.range.clone());
        RenderCommandResult::Success
    }
}

#[derive(Resource)]
struct SmudPipeline<const PARAMS: usize> {
    view_layout: BindGroupLayout,
    shaders: ShapeShaders,
}

impl<const PARAMS: usize> FromWorld for SmudPipeline<PARAMS> {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();

        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: Some(ViewUniform::min_size()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(GlobalsUniform::min_size()),
                    },
                    count: None,
                },
            ],
            label: Some("shape_view_layout"),
        });

        Self {
            view_layout,
            shaders: default(),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct SmudPipelineKey {
    mesh: PipelineKey,
    shader: ShaderKey,
    hdr: bool,
}

impl<const PARAMS: usize> SpecializedRenderPipeline for SmudPipeline<PARAMS> {
    type Key = SmudPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let fragment_shader = self.shaders.fragment_shaders.get(&key.shader).unwrap();
        debug!("specializing for {fragment_shader:?}");

        // an f32 is 4 bytes
        const WORD_LENGTH: u64 = 4;
        const WORDS_PER_PARAM: u64 = 1;

        const COLOR_WORDS: u64 = 4;
        const FRAME_WORDS: u64 = 1;
        const POSITION_WORDS: u64 = 3;
        const ROTATION_WORDS: u64 = 2;
        const SCALE_WORDS: u64 = 1;

        // (GOTCHA! attributes are sorted alphabetically, and offsets need to reflect this)

        let pre_param_attributes: [VertexAttribute; 2] = [
            // Color
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 0,
                shader_location: 1,
            },
            // Frame
            VertexAttribute {
                format: VertexFormat::Float32,
                offset: (COLOR_WORDS) * WORD_LENGTH,
                shader_location: 4 + PARAMS as u32,
            },
        ];

        let mut param_attributes: [VertexAttribute; PARAMS] = [
            // perf: Maybe it's possible to pack this more efficiently?
            // Params
            VertexAttribute {
                format: VertexFormat::Float32,
                offset: (4 + 1) * WORD_LENGTH,
                shader_location: 2,
            };
             PARAMS];

        for (index, attribute) in param_attributes.iter_mut().enumerate() {
            attribute.offset += index as u64 * WORD_LENGTH * WORDS_PER_PARAM;
            attribute.shader_location += index as u32;
        }

        let post_param_attributes: [VertexAttribute; 3] = [
            // Position
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: (COLOR_WORDS + FRAME_WORDS + (WORDS_PER_PARAM * PARAMS as u64))
                    * WORD_LENGTH,
                shader_location: 0,
            },
            // Rotation
            VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: (COLOR_WORDS
                    + FRAME_WORDS
                    + (WORDS_PER_PARAM * PARAMS as u64)
                    + POSITION_WORDS)
                    * WORD_LENGTH,
                shader_location: 2 + PARAMS as u32,
            },
            // Scale
            VertexAttribute {
                format: VertexFormat::Float32,
                offset: (COLOR_WORDS
                    + FRAME_WORDS
                    + (WORDS_PER_PARAM * PARAMS as u64)
                    + POSITION_WORDS
                    + ROTATION_WORDS)
                    * WORD_LENGTH,
                shader_location: 3 + PARAMS as u32,
            },
        ];

        // Customize how to store the meshes' vertex attributes in the vertex buffer
        // Our meshes only have position, color and params
        let mut vertex_attributes = Vec::with_capacity(
            pre_param_attributes.len() + param_attributes.len() + post_param_attributes.len(),
        );

        vertex_attributes.extend_from_slice(&pre_param_attributes);
        vertex_attributes.extend_from_slice(&param_attributes);
        vertex_attributes.extend_from_slice(&post_param_attributes);

        // This is the sum of the size of the attributes above
        let vertex_array_stride = (COLOR_WORDS
            + FRAME_WORDS
            + (WORDS_PER_PARAM * PARAMS as u64)
            + POSITION_WORDS
            + ROTATION_WORDS
            + SCALE_WORDS)
            * WORD_LENGTH;

        info!("{vertex_attributes:?}");
        info!("stride: {vertex_array_stride}");


        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: shader_loading::get_vertex_handle::<PARAMS>().clone_weak(),
                entry_point: "vertex".into(),
                shader_defs: Vec::new(),
                buffers: vec![VertexBufferLayout {
                    array_stride: vertex_array_stride,
                    step_mode: VertexStepMode::Instance,
                    attributes: vertex_attributes,
                }],
            },
            fragment: Some(FragmentState {
                shader: fragment_shader.clone_weak(),
                entry_point: "fragment".into(),
                shader_defs: Vec::new(),
                targets: vec![Some(ColorTargetState {
                    format: if key.hdr {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: vec![
                // Bind group 0 is the view uniform
                self.view_layout.clone(),
            ],
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false, // What is this?
                polygon_mode: PolygonMode::Fill,
                conservative: false, // What is this?
                topology: key.mesh.primitive_topology(),
                strip_index_format: None, // TODO: what does this do?
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.mesh.msaa_samples(),
                mask: !0,                         // what does the mask do?
                alpha_to_coverage_enabled: false, // what is this?
            },
            label: Some("bevy_smud_pipeline".into()),
            push_constant_ranges: Vec::new(),
        }
    }
}

#[derive(Default)]
struct ShapeShaders {
    fragment_shaders: HashMap<ShaderKey, Handle<Shader>>,
}

// TODO: do some of this work in the main world instead, so we don't need to take a mutable
// reference to MainWorld.
fn extract_sdf_shaders<const PARAMS: usize>(
    mut main_world: ResMut<MainWorld>,
    mut pipeline: ResMut<SmudPipeline<PARAMS>>,
) {
    main_world.resource_scope(|world, mut shaders: Mut<Assets<Shader>>| {
        let mut shapes = world.query::<&SmudShape<PARAMS>>();

        for shape in shapes.iter(world) {
            let shader_key = ShaderKey {
                sdf_shader: shape.sdf.id(),
                fill_shader: shape.fill.id(),
                sdf_params_usage: shape.sdf_param_usage,
                fill_params_usage: shape.fill_param_usage,
            };
            if pipeline.shaders.fragment_shaders.contains_key(&shader_key) {
                continue;
            }

            // todo use asset events instead?
            let sdf_import_path = match shaders.get_mut(&shape.sdf.clone()) {
                Some(shader) => match shader.import_path() {
                    ShaderImport::Custom(p) => p.to_owned(),
                    _ => {
                        let id = generate_shader_id();
                        let path = format!("smud::generated::{id}");
                        shader.set_import_path(&path);
                        path
                    }
                },
                None => {
                    debug!("Waiting for sdf to load");
                    continue;
                }
            };

            let fill_import_path = match shaders.get_mut(&shape.fill.clone()) {
                Some(shader) => match shader.import_path() {
                    ShaderImport::Custom(p) => p.to_owned(),
                    _ => {
                        let id = generate_shader_id();
                        let path = format!("smud::generated::{id}");
                        shader.set_import_path(&path);
                        path
                    }
                },
                None => {
                    debug!("Waiting for fill to load");
                    continue;
                }
            };

            debug!("Generating shader");
            let params_locations = vertex_shader::format_params_locations::<PARAMS>();
            let sdf_params = shader_key.sdf_params_usage.in_params_str();
            let fill_params = shader_key.fill_params_usage.in_params_str();

            let source = format!(
                r#"
#import bevy_render::globals::Globals
@group(0) @binding(1)
var<uniform> globals: Globals;
#import {sdf_import_path} as sdf
#import {fill_import_path} as fill

struct FragmentInput {{
@location(0) color: vec4<f32>,
@location(1) pos: vec2<f32>,
{params_locations}
}};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {{
let d = sdf::sdf(in.pos{sdf_params});
return fill::fill(d, in.color{fill_params});
}}
"#
            );

            info!("{source}");
            let generated_shader =
                Shader::from_wgsl(source, format!("smud::generated::{shader_key:?}"));

            // todo does this work, or is it too late?
            let generated_shader_handle = shaders.add(generated_shader);

            pipeline
                .shaders
                .fragment_shaders
                .insert(shader_key, generated_shader_handle);
        }
    });
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ShaderKey {
    sdf_shader: AssetId<Shader>,
    fill_shader: AssetId<Shader>,
    sdf_params_usage: ShaderParamUsage,
    fill_params_usage: ShaderParamUsage,
}

impl ShaderKey {
    /// To be used as a placeholder value
    pub const INVALID: Self = Self {
        sdf_shader: AssetId::invalid(),
        fill_shader: AssetId::invalid(),
        sdf_params_usage: ShaderParamUsage::NO_PARAMS,
        fill_params_usage: ShaderParamUsage::NO_PARAMS,
    };
}

impl<const PARAMS: usize> From<&SmudShape<PARAMS>> for ShaderKey {
    fn from(value: &SmudShape<PARAMS>) -> Self {
        Self {
            sdf_shader: value.sdf.id(),
            fill_shader: value.fill.id(),
            sdf_params_usage: value.sdf_param_usage,
            fill_params_usage: value.fill_param_usage,
        }
    }
}

#[derive(Component, Clone, Debug)]
struct ExtractedShape<const PARAMS: usize> {
    color: Color,
    params: [f32; PARAMS],
    frame: f32,
    transform: GlobalTransform,
    shader_key: ShaderKey,
}

#[derive(Resource, Default, Debug)]
struct ExtractedShapes<const PARAMS: usize> {
    shapes: EntityHashMap<Entity, ExtractedShape<PARAMS>>,
}

fn extract_shapes<const PARAMS: usize>(
    mut extracted_shapes: ResMut<ExtractedShapes<PARAMS>>,
    shape_query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &SmudShape<PARAMS>,
            &GlobalTransform,
        )>,
    >,
) {
    extracted_shapes.shapes.clear();

    for (entity, view_visibility, shape, transform) in shape_query.iter() {
        if !view_visibility.get() {
            continue;
        }

        let Frame::Quad(frame) = shape.frame;

        extracted_shapes.shapes.insert(
            entity,
            ExtractedShape {
                color: shape.color,
                params: shape.params,
                transform: *transform,
                shader_key: shape.into(),
                frame,
            },
        );
    }
}

// fork of Mesh2DPipelineKey (in order to remove bevy_sprite dependency)
// todo: merge with SmudPipelineKey?
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    struct PipelineKey: u32 {
        const MSAA_RESERVED_BITS                = Self::MSAA_MASK_BITS << Self::MSAA_SHIFT_BITS;
        const PRIMITIVE_TOPOLOGY_RESERVED_BITS  = Self::PRIMITIVE_TOPOLOGY_MASK_BITS << Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS;
    }
}

impl PipelineKey {
    const MSAA_MASK_BITS: u32 = 0b111;
    const MSAA_SHIFT_BITS: u32 = 32 - Self::MSAA_MASK_BITS.count_ones();
    const PRIMITIVE_TOPOLOGY_MASK_BITS: u32 = 0b111;
    const PRIMITIVE_TOPOLOGY_SHIFT_BITS: u32 = Self::MSAA_SHIFT_BITS - 3;

    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits =
            (msaa_samples.trailing_zeros() & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        Self::from_bits(msaa_bits).unwrap()
    }

    pub fn msaa_samples(&self) -> u32 {
        1 << ((self.bits() >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS)
    }

    pub fn from_primitive_topology(primitive_topology: PrimitiveTopology) -> Self {
        let primitive_topology_bits = ((primitive_topology as u32)
            & Self::PRIMITIVE_TOPOLOGY_MASK_BITS)
            << Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS;
        Self::from_bits(primitive_topology_bits).unwrap()
    }

    pub fn primitive_topology(&self) -> PrimitiveTopology {
        let primitive_topology_bits = (self.bits() >> Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS)
            & Self::PRIMITIVE_TOPOLOGY_MASK_BITS;
        match primitive_topology_bits {
            x if x == PrimitiveTopology::PointList as u32 => PrimitiveTopology::PointList,
            x if x == PrimitiveTopology::LineList as u32 => PrimitiveTopology::LineList,
            x if x == PrimitiveTopology::LineStrip as u32 => PrimitiveTopology::LineStrip,
            x if x == PrimitiveTopology::TriangleList as u32 => PrimitiveTopology::TriangleList,
            x if x == PrimitiveTopology::TriangleStrip as u32 => PrimitiveTopology::TriangleStrip,
            _ => PrimitiveTopology::default(),
        }
    }
}

fn queue_shapes<const PARAMS: usize>(
    mut view_entities: Local<FixedBitSet>,
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    smud_pipeline: Res<SmudPipeline<PARAMS>>,
    mut pipelines: ResMut<SpecializedRenderPipelines<SmudPipeline<PARAMS>>>,
    pipeline_cache: ResMut<PipelineCache>,
    msaa: Res<Msaa>,
    extracted_shapes: ResMut<ExtractedShapes<PARAMS>>,
    mut views: Query<(
        &mut RenderPhase<Transparent2d>,
        &VisibleEntities,
        &ExtractedView,
    )>,
    // ?
) {
    let draw_smud_shape_function = draw_functions
        .read()
        .get_id::<DrawSmudShape<PARAMS>>()
        .unwrap();

    // Iterate over each view (a camera is a view)
    for (mut transparent_phase, visible_entities, view) in &mut views {
        // todo: bevy_sprite does some hdr stuff, should we?
        // let mut view_key = SpritePipelineKey::from_hdr(view.hdr) | msaa_key;

        let mesh_key = PipelineKey::from_msaa_samples(msaa.samples())
            | PipelineKey::from_primitive_topology(PrimitiveTopology::TriangleStrip);

        view_entities.clear();
        view_entities.extend(visible_entities.entities.iter().map(|e| e.index() as usize));

        transparent_phase
            .items
            .reserve(extracted_shapes.shapes.len());

        for (entity, extracted_shape) in extracted_shapes.shapes.iter() {
            let mut pipeline = CachedRenderPipelineId::INVALID;

            if let Some(_shader) = smud_pipeline
                .shaders
                .fragment_shaders
                .get(&extracted_shape.shader_key)
            {
                // todo pass the shader into specialize
                let specialize_key = SmudPipelineKey {
                    mesh: mesh_key,
                    shader: extracted_shape.shader_key.clone(),
                    hdr: view.hdr,
                };
                pipeline = pipelines.specialize(&pipeline_cache, &smud_pipeline, specialize_key);
            }

            if pipeline == CachedRenderPipelineId::INVALID {
                debug!("Shape not ready yet, skipping");
                continue; // skip shapes that are not ready yet
            }

            // These items will be sorted by depth with other phase items
            let sort_key = FloatOrd(extracted_shape.transform.translation().z);

            // Add the item to the render phase
            transparent_phase.add(Transparent2d {
                draw_function: draw_smud_shape_function,
                pipeline,
                entity: *entity,
                sort_key,
                // batch_range and dynamic_offset will be calculated in prepare_shapes
                batch_range: 0..0,
                dynamic_offset: None,
            });
        }
    }
}

fn prepare_shapes<const PARAMS: usize>(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut shape_meta: ResMut<ShapeMeta<PARAMS>>,
    view_uniforms: Res<ViewUniforms>,
    smud_pipeline: Res<SmudPipeline<PARAMS>>,
    extracted_shapes: Res<ExtractedShapes<PARAMS>>,
    mut phases: Query<&mut RenderPhase<Transparent2d>>,
    globals_buffer: Res<GlobalsBuffer>,
) {
    let globals = globals_buffer.buffer.binding().unwrap(); // todo if-let

    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        let mut batches: Vec<(Entity, ShapeBatch)> = Vec::with_capacity(*previous_len);

        // Clear the vertex buffer
        shape_meta.vertices.clear();

        shape_meta.view_bind_group = Some(render_device.create_bind_group(
            "smud_shape_view_bind_group",
            &smud_pipeline.view_layout,
            &BindGroupEntries::sequential((view_binding, globals.clone())),
        ));

        // Vertex buffer index
        let mut index = 0;

        for mut transparent_phase in &mut phases {
            let mut batch_item_index = 0;
            // let mut batch_image_size = Vec2::ZERO;
            // let mut batch_image_handle = AssetId::invalid();
            let mut batch_shader_key: ShaderKey = ShaderKey::INVALID;

            // Iterate through the phase items and detect when successive shapes that can be batched.
            // Spawn an entity with a `ShapeBatch` component for each possible batch.
            // Compatible items share the same entity.
            for item_index in 0..transparent_phase.items.len() {
                let item = &transparent_phase.items[item_index];
                let Some(extracted_shape) = extracted_shapes.shapes.get(&item.entity) else {
                    // If there is a phase item that is not a shape, then we must start a new
                    // batch to draw the other phase item(s) and to respect draw order. This can be
                    // done by invalidating the batch_shader_handles
                    batch_shader_key = ShaderKey::INVALID;
                    continue;
                };

                let shader_handles = extracted_shape.shader_key.clone();

                let batch_shader_changed = batch_shader_key != shader_handles;

                let color = extracted_shape.color.as_linear_rgba_f32();

                let position = extracted_shape.transform.translation();
                let position = position.into();

                let rotation_and_scale = extracted_shape
                    .transform
                    .affine()
                    .transform_vector3(Vec3::X)
                    .xy();

                let scale = rotation_and_scale.length();
                let rotation = (rotation_and_scale / scale).into();

                let vertex = ShapeVertex {
                    position,
                    color,
                    params: extracted_shape.params,
                    rotation,
                    scale,
                    frame: extracted_shape.frame,
                };

                shape_meta.vertices.push(vertex);

                if batch_shader_changed {
                    batch_item_index = item_index;

                    batches.push((
                        item.entity,
                        ShapeBatch {
                            shader: (shader_handles.sdf_shader, shader_handles.fill_shader),
                            range: index..index,
                        },
                    ));
                }

                transparent_phase.items[batch_item_index]
                    .batch_range_mut()
                    .end += 1;

                batches.last_mut().unwrap().1.range.end += 1;
                index += 1;
            }
        }

        shape_meta
            .vertices
            .write_buffer(&render_device, &render_queue);

        *previous_len = batches.len();
        commands.insert_or_spawn_batch(batches);
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct ShapeVertex<const PARAMS: usize> {
    pub color: [f32; 4],
    pub frame: f32,
    pub params: [f32; PARAMS],
    pub position: [f32; 3],
    pub rotation: [f32; 2],
    pub scale: f32,
}

unsafe impl<const PARAMS: usize> Zeroable for ShapeVertex<PARAMS> {}

unsafe impl<const PARAMS: usize> Pod for ShapeVertex<PARAMS> {}

#[derive(Resource)]
pub(crate) struct ShapeMeta<const PARAMS: usize> {
    vertices: BufferVec<ShapeVertex<PARAMS>>,
    view_bind_group: Option<BindGroup>,
}

impl<const PARAMS: usize> Default for ShapeMeta<PARAMS> {
    fn default() -> Self {
        Self {
            vertices: BufferVec::new(BufferUsages::VERTEX),
            view_bind_group: None,
        }
    }
}

#[derive(Component, Eq, PartialEq, Clone)]
pub(crate) struct ShapeBatch {
    shader: (AssetId<Shader>, AssetId<Shader>), //todo is this field needed
    range: Range<u32>,
}
