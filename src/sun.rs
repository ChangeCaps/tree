use bevy::{
    core::AsBytes,
    ecs::{
        query::{QueryState, ReadOnlyFetch, WorldQuery},
        system::BoxedSystem,
    },
    prelude::*,
    reflect::TypeUuid,
    render::{
        camera::{ActiveCameras, CameraProjection, OrthographicProjection, PerspectiveProjection},
        draw::{DrawContext, RenderCommand},
        mesh::{Indices, INDEX_BUFFER_ASSET_INDEX, VERTEX_ATTRIBUTE_BUFFER_ID},
        pass::{
            LoadOp, Operations, PassDescriptor, RenderPassColorAttachmentDescriptor,
            RenderPassDepthStencilAttachmentDescriptor, TextureAttachment,
        },
        pipeline::{
            BlendFactor, BlendOperation, BlendState, ColorTargetState, ColorWrite, CompareFunction,
            CullMode, DepthBiasState, DepthStencilState, IndexFormat, MultisampleState,
            PipelineDescriptor, PipelineSpecialization, PrimitiveState, PrimitiveTopology,
            RenderPipeline, StencilFaceState, StencilState,
        },
        render_graph::{
            base, AssetRenderResourcesNode, CameraNode, CommandQueue, Node, PassNode, RenderGraph,
            ResourceSlotInfo, ResourceSlots, SystemNode,
        },
        renderer::{
            BindGroupId, BufferId, BufferInfo, BufferMapMode, BufferUsage, RenderContext,
            RenderResourceBinding, RenderResourceBindings, RenderResourceContext, RenderResourceId,
            RenderResourceType, RenderResources, SamplerId, TextureId,
        },
        shader::ShaderStages,
        texture::{
            AddressMode, Extent3d, FilterMode, SamplerDescriptor, TextureDescriptor,
            TextureDimension, TextureFormat, TextureUsage, SAMPLER_ASSET_INDEX,
            TEXTURE_ASSET_INDEX,
        },
    },
    utils::HashSet,
};
use std::ops::Deref;
use std::sync::{Arc, Mutex};

pub const SUN_NODE: &str = "sun_node";
pub const SHADOW_MAP_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Texture::TYPE_UUID, 16478324);
pub const SHADOW_MAP_NODE: &str = "shadow_map_node";
pub const SHADOW_MAP_TEXTURE: &str = "shadow_map_texture";
pub const SHADOW_MAP_SAMPLER: &str = "shadow_map_sampler";
pub const SHADOWS_NODE: &str = "shadows_node";
pub const SHADOW_TEXTURE_NODE: &str = "shadow_texture_node";
pub const SHADOW_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 5437868423);
pub const SHADED_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 53849761);

pub struct TextureNode {
    texture_descriptor: TextureDescriptor,
    sampler_descriptor: SamplerDescriptor,
    handle: HandleUntyped,
}

impl TextureNode {
    pub fn new(
        texture_descriptor: TextureDescriptor,
        sampler_descriptor: SamplerDescriptor,
        handle: HandleUntyped,
    ) -> Self {
        Self {
            texture_descriptor,
            sampler_descriptor,
            handle,
        }
    }
}

impl Node for TextureNode {
    fn output(&self) -> &[ResourceSlotInfo] {
        &[
            ResourceSlotInfo {
                name: std::borrow::Cow::Borrowed("texture"),
                resource_type: RenderResourceType::Texture,
            },
            ResourceSlotInfo {
                name: std::borrow::Cow::Borrowed("sampler"),
                resource_type: RenderResourceType::Sampler,
            },
        ]
    }

    fn update(
        &mut self,
        _world: &World,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        output: &mut ResourceSlots,
    ) {
        if output.get("texture").is_none() {
            let texture_id = render_context
                .resources()
                .create_texture(self.texture_descriptor);

            output.set("texture", RenderResourceId::Texture(texture_id));

            render_context.resources().set_asset_resource_untyped(
                self.handle.clone(),
                RenderResourceId::Texture(texture_id),
                TEXTURE_ASSET_INDEX,
            );
        }

        if output.get("sampler").is_none() {
            let sampler_id = render_context
                .resources()
                .create_sampler(&self.sampler_descriptor);

            output.set("sampler", RenderResourceId::Sampler(sampler_id));

            render_context.resources().set_asset_resource_untyped(
                self.handle.clone(),
                RenderResourceId::Sampler(sampler_id),
                SAMPLER_ASSET_INDEX,
            );
        }
    }
}

pub struct ShadowsNode<Q: WorldQuery> {
    command_queue: CommandQueue,
    draw: Arc<Mutex<Draw>>,
    query_state: Option<QueryState<Q>>,
    pass_descriptor: PassDescriptor,
}

impl<Q: WorldQuery> ShadowsNode<Q> {
    pub fn new() -> Self {
        Self {
            command_queue: CommandQueue::default(),
            query_state: None,
            draw: Default::default(),
            pass_descriptor: PassDescriptor {
                color_attachments: vec![],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachmentDescriptor {
                    attachment: TextureAttachment::Input("depth".into()),
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
                sample_count: 1,
            },
        }
    }
}

impl<Q: WorldQuery + Send + Sync + 'static> Node for ShadowsNode<Q>
where
    Q::Fetch: ReadOnlyFetch,
{
    fn input(&self) -> &[ResourceSlotInfo] {
        &[ResourceSlotInfo {
            name: std::borrow::Cow::Borrowed(SHADOW_MAP_TEXTURE),
            resource_type: RenderResourceType::Texture,
        }]
    }

    fn prepare(&mut self, world: &mut World) {
        self.query_state.get_or_insert_with(|| world.query());
    }

    fn update(
        &mut self,
        world: &World,
        render_context: &mut dyn RenderContext,
        input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        let shadow_texture = TextureAttachment::Id(
            input
                .get(SHADOW_MAP_TEXTURE)
                .unwrap()
                .get_texture()
                .unwrap(),
        );
        self.pass_descriptor
            .depth_stencil_attachment
            .as_mut()
            .unwrap()
            .attachment = shadow_texture;

        let render_resource_bindings = world.get_resource::<RenderResourceBindings>().unwrap();
        let pipelines = world.get_resource::<Assets<PipelineDescriptor>>().unwrap();

        self.command_queue.execute(render_context);

        let mut draw = self.draw.lock().unwrap();
        let mut draw_state = DrawState::default();

        render_context.begin_pass(
            &self.pass_descriptor,
            render_resource_bindings,
            &mut |render_pass| {
                for render_command in draw.render_commands.drain(..) {
                    match render_command {
                        RenderCommand::SetPipeline { pipeline } => {
                            if draw_state.is_pipeline_set(pipeline.clone_weak()) {
                                continue;
                            }
                            render_pass.set_pipeline(&pipeline);
                            let descriptor = pipelines.get(&pipeline).unwrap();
                            draw_state.set_pipeline(&pipeline, descriptor);
                        }
                        RenderCommand::DrawIndexed {
                            base_vertex,
                            indices,
                            instances,
                        } => {
                            if draw_state.can_draw_indexed() {
                                render_pass.draw_indexed(
                                    indices.clone(),
                                    base_vertex,
                                    instances.clone(),
                                );
                            } else {
                                //dbg!(&draw_state);
                                debug!("Could not draw indexed because the pipeline layout wasn't fully set for pipeline: {:?}", draw_state.pipeline);
                                //panic!();
                            }
                        }
                        RenderCommand::Draw {
                            vertices,
                            instances,
                        } => {
                            if draw_state.can_draw() {
                                render_pass.draw(vertices.clone(), instances.clone());
                            } else {
                                //dbg!(&draw_state);
                                debug!("Could not draw because the pipeline layout wasn't fully set for pipeline: {:?}", draw_state.pipeline);
                                //panic!();
                            }
                        }
                        RenderCommand::SetVertexBuffer {
                            buffer,
                            offset,
                            slot,
                        } => {
                            if draw_state.is_vertex_buffer_set(slot, buffer, offset) {
                                continue;
                            }
                            render_pass.set_vertex_buffer(slot, buffer, offset);
                            draw_state.set_vertex_buffer(slot, buffer, offset);
                        }
                        RenderCommand::SetIndexBuffer {
                            buffer,
                            offset,
                            index_format,
                        } => {
                            if draw_state.is_index_buffer_set(buffer, offset, index_format) {
                                continue;
                            }
                            render_pass.set_index_buffer(buffer, offset, index_format);
                            draw_state.set_index_buffer(buffer, offset, index_format);
                        }
                        RenderCommand::SetBindGroup {
                            index,
                            bind_group,
                            dynamic_uniform_indices,
                        } => {
                            if dynamic_uniform_indices.is_none()
                                && draw_state.is_bind_group_set(index, bind_group)
                            {
                                continue;
                            }
                            let pipeline = pipelines
                                .get(draw_state.pipeline.as_ref().unwrap())
                                .unwrap();
                            let layout = pipeline.get_layout().unwrap();
                            let bind_group_descriptor = layout.get_bind_group(index).unwrap();
                            render_pass.set_bind_group(
                                index,
                                bind_group_descriptor.id,
                                bind_group,
                                dynamic_uniform_indices
                                    .as_ref()
                                    .map(|indices| indices.deref()),
                            );
                            draw_state.set_bind_group(index, bind_group);
                        }
                    }
                }
            },
        )
    }
}

impl<Q: WorldQuery + Send + Sync + 'static> SystemNode for ShadowsNode<Q>
where
    Q::Fetch: ReadOnlyFetch,
{
    fn get_system(&self) -> BoxedSystem {
        let system = shadows_node_system.system().config(|config| {
            config.0 = Some(ShadowsNodeSystemState {
                command_queue: self.command_queue.clone(),
                staging_buffer: None,
                buffer: None,
                draw: self.draw.clone(),
                render_pipeline: RenderPipeline::new(SHADOW_PIPELINE_HANDLE.typed()),
            })
        });
        Box::new(system)
    }
}

#[derive(Debug, Default)]
pub struct ShadowsNodeSystemState {
    staging_buffer: Option<BufferId>,
    buffer: Option<BufferId>,
    command_queue: CommandQueue,
    draw: Arc<Mutex<Draw>>,
    render_pipeline: RenderPipeline,
}

pub fn shadows_node_system(
    state: Local<ShadowsNodeSystemState>,
    mut draw_context: DrawContext,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    // TODO: this write on RenderResourceBindings will prevent this system from running in parallel with other systems that do the same
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    meshes: Res<Assets<Mesh>>,
    mut shadow_casters: Query<(&mut ShadowCaster, &Handle<Mesh>)>,
) {
    let draw = state.draw.clone();
    let mut draw = draw.lock().unwrap();

    shadow_casters
        .iter_mut()
        .for_each(|(mut shadow_caster, mesh_handle)| {
            let mesh = if let Some(mesh) = meshes.get(mesh_handle) {
                mesh
            } else {
                return;
            };

            let render_pipelines = &mut shadow_caster.render_pipelines;
            for pipeline in render_pipelines.pipelines.iter_mut() {
                if pipeline.dynamic_bindings_generation
                    != render_pipelines.bindings.dynamic_bindings_generation()
                {
                    pipeline.specialization.dynamic_bindings = render_pipelines
                        .bindings
                        .iter_dynamic_bindings()
                        .map(|name| name.to_string())
                        .collect::<HashSet<String>>();

                    pipeline.dynamic_bindings_generation =
                        render_pipelines.bindings.dynamic_bindings_generation();

                    for (handle, _) in render_pipelines.bindings.iter_assets() {
                        if let Some(bindings) = draw_context
                            .asset_render_resource_bindings
                            .get_untyped(handle)
                        {
                            for binding in bindings.iter_dynamic_bindings() {
                                pipeline
                                    .specialization
                                    .dynamic_bindings
                                    .insert(binding.to_string());
                            }
                        }
                    }
                }
            }

            // set up pipelinespecialzation and bindings
            // see crates\bevy_render\src\mesh\mesh.rs:502
            let mut pipeline_specialization = render_pipelines.pipelines[0].specialization.clone();
            pipeline_specialization.primitive_topology = mesh.primitive_topology();
            pipeline_specialization.vertex_buffer_layout = mesh.get_vertex_buffer_layout();
            if let PrimitiveTopology::LineStrip | PrimitiveTopology::TriangleStrip =
                mesh.primitive_topology()
            {
                pipeline_specialization.strip_index_format =
                    mesh.indices().map(|indices| indices.into());
            }

            draw_context
                .set_pipeline(
                    &mut draw,
                    &render_pipelines.pipelines[0].pipeline,
                    &pipeline_specialization,
                )
                .unwrap();

            // for binding in &render_resource_bindings.bindings {
            //     dbg!(binding);
            // }

            // for binding in &render_pipelines.bindings.bindings {
            //     dbg!(binding);
            // }

            draw_context
                .set_bind_groups_from_bindings(
                    &mut draw,
                    &mut [
                        &mut render_resource_bindings,
                        &mut render_pipelines.bindings,
                    ],
                )
                .unwrap();

            if let Some(RenderResourceId::Buffer(index_buffer_resource)) =
                render_resource_context.get_asset_resource(mesh_handle, INDEX_BUFFER_ASSET_INDEX)
            {
                let index_format: IndexFormat = mesh.indices().unwrap().into();
                // skip draw_context because it requires a RenderPipeline
                // and doesn't actually do anything special
                draw.set_index_buffer(index_buffer_resource, 0, index_format);
            }

            if let Some(RenderResourceId::Buffer(vertex_attribute_buffer_resource)) =
                render_resource_context.get_asset_resource(mesh_handle, VERTEX_ATTRIBUTE_BUFFER_ID)
            {
                // skip draw_context because it requires a RenderPipeline
                // and doesn't actually do anything special
                draw.set_vertex_buffer(0, vertex_attribute_buffer_resource, 0);
            }

            let index_range = match mesh.indices() {
                Some(Indices::U32(indices)) => Some(0..indices.len() as u32),
                Some(Indices::U16(indices)) => Some(0..indices.len() as u32),
                None => None,
            };

            // dbg!(mesh_handle);
            if let Some(indices) = index_range.clone() {
                draw.draw_indexed(indices, 0, 0..1);
            } else {
                draw.draw(0..mesh.count_vertices() as u32, 0..1)
            }
        });
}

pub struct Sun;

#[derive(Default)]
pub struct SunNode {
    command_queue: CommandQueue,
}

impl Node for SunNode {
    fn update(
        &mut self,
        _world: &World,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        self.command_queue.execute(render_context);
    }
}

impl SystemNode for SunNode {
    fn get_system(&self) -> BoxedSystem {
        let system = sun_node_system.system().config(|config| {
            config.0 = Some(SunNodeState {
                command_queue: self.command_queue.clone(),
                sun_buffer: None,
                staging_buffer: None,
            })
        });
        Box::new(system)
    }
}

#[derive(Default)]
pub struct SunNodeState {
    command_queue: CommandQueue,
    sun_buffer: Option<BufferId>,
    staging_buffer: Option<BufferId>,
}

pub fn sun_node_system(
    mut state: Local<SunNodeState>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    mut active_cameras: ResMut<ActiveCameras>,
    query: Query<&GlobalTransform, With<Sun>>,
) {
    let transform = query.iter().next().unwrap();

    let projection = OrthographicProjection {
        left: -50.0,
        right: 50.0,
        bottom: -50.0,
        top: 50.0,
        near: 0.1,
        far: 200.0,
        ..Default::default()
    };

    let proj = projection.get_projection_matrix() * transform.compute_matrix().inverse();
    let (x, y, z) = transform.translation.into();

    let matrix = std::mem::size_of::<[[f32; 4]; 4]>();
    let vec = std::mem::size_of::<[f32; 3]>();
    let size = matrix + vec;

    let staging_buffer = if let Some(staging_buffer) = state.staging_buffer {
        render_resource_context.map_buffer(staging_buffer, BufferMapMode::Write);

        staging_buffer
    } else {
        let buffer = render_resource_context.create_buffer(BufferInfo {
            size,
            buffer_usage: BufferUsage::UNIFORM | BufferUsage::COPY_SRC | BufferUsage::COPY_DST,
            ..Default::default()
        });
        render_resource_bindings.set(
            "Sun",
            RenderResourceBinding::Buffer {
                buffer,
                range: 0..size as u64,
                dynamic_index: None,
            },
        );
        if let Some(active_camera) = active_cameras.get_mut(base::camera::CAMERA_3D) {
            active_camera.bindings.set(
                "Sun",
                RenderResourceBinding::Buffer {
                    buffer,
                    range: 0..size as u64,
                    dynamic_index: None,
                },
            );
        }
        state.sun_buffer = Some(buffer);

        let staging_buffer = render_resource_context.create_buffer(BufferInfo {
            size: size,
            buffer_usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
            mapped_at_creation: true,
        });

        state.staging_buffer = Some(staging_buffer);

        staging_buffer
    };

    render_resource_context.write_mapped_buffer(staging_buffer, 0..size as u64, &mut |data, _| {
        data[0..matrix].copy_from_slice(proj.to_cols_array_2d().as_bytes());
        data[matrix..matrix + vec].copy_from_slice([x, y, z].as_bytes());
    });

    render_resource_context.unmap_buffer(staging_buffer);
    let sun_buffer = state.sun_buffer.unwrap();
    state
        .command_queue
        .copy_buffer_to_buffer(staging_buffer, 0, sun_buffer, 0, size as u64);
}

#[derive(Default)]
pub struct ShadowMapNode {
    texture_id: Arc<Mutex<Option<TextureId>>>,
    sampler_id: Arc<Mutex<Option<SamplerId>>>,
}

impl Node for ShadowMapNode {
    fn input(&self) -> &[ResourceSlotInfo] {
        &[
            ResourceSlotInfo {
                name: std::borrow::Cow::Borrowed("texture"),
                resource_type: RenderResourceType::Texture,
            },
            ResourceSlotInfo {
                name: std::borrow::Cow::Borrowed("sampler"),
                resource_type: RenderResourceType::Sampler,
            },
        ]
    }

    fn update(
        &mut self,
        _world: &World,
        _render_context: &mut dyn RenderContext,
        input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        if let Some(RenderResourceId::Texture(texture)) = input.get("texture") {
            *self.texture_id.lock().unwrap() = Some(texture);
        }

        if let Some(RenderResourceId::Sampler(sampler)) = input.get("sampler") {
            *self.sampler_id.lock().unwrap() = Some(sampler);
        }
    }
}

impl SystemNode for ShadowMapNode {
    fn get_system(&self) -> BoxedSystem {
        let system = shadow_map_node.system().config(|config| {
            config.0 = Some(self.texture_id.clone());
            config.1 = Some(self.sampler_id.clone());
        });
        Box::new(system)
    }
}

pub fn shadow_map_node(
    texture_id: Local<Arc<Mutex<Option<TextureId>>>>,
    sampler_id: Local<Arc<Mutex<Option<SamplerId>>>>,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
) {
    if let Some(texture_id) = *texture_id.lock().unwrap() {
        render_resource_bindings.set(
            "ShadowMapTexture",
            RenderResourceBinding::Texture(texture_id),
        );
    }

    if let Some(sampler_id) = *sampler_id.lock().unwrap() {
        render_resource_bindings.set(
            "ShadowMapSampler",
            RenderResourceBinding::Sampler(sampler_id),
        );
    }
}

#[derive(Bundle)]
pub struct ShadedBundle {
    pub mesh: Handle<Mesh>,
    pub draw: Draw,
    pub visible: Visible,
    pub main_pass: base::MainPass,
    pub shadow_caster: ShadowCaster,
    pub render_pipelines: RenderPipelines,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for ShadedBundle {
    fn default() -> Self {
        Self {
            mesh: Default::default(),
            draw: Default::default(),
            visible: Default::default(),
            main_pass: base::MainPass,
            shadow_caster: ShadowCaster::default(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                SHADED_PIPELINE_HANDLE.typed(),
            )]),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

pub struct ShadowCaster {
    pub render_pipelines: RenderPipelines,
}

impl ShadowCaster {
    pub fn new(render_pipelines: RenderPipelines) -> Self {
        Self { render_pipelines }
    }
}

impl Default for ShadowCaster {
    fn default() -> Self {
        Self {
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                SHADOW_PIPELINE_HANDLE.typed(),
            )]),
        }
    }
}

pub fn shadow_pipeline(shader_stages: ShaderStages) -> PipelineDescriptor {
    PipelineDescriptor {
        color_target_states: vec![],
        depth_stencil: Some(DepthStencilState {
            format: TextureFormat::Depth24Plus,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Less,
            stencil: StencilState {
                front: StencilFaceState::IGNORE,
                back: StencilFaceState::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
            bias: DepthBiasState {
                constant: 0,
                slope_scale: 0.0,
                clamp: 0.0,
            },
            clamp_depth: false,
        }),
        primitive: PrimitiveState {
            cull_mode: CullMode::Front,
            ..Default::default()
        },
        ..PipelineDescriptor::default_config(shader_stages)
    }
}

pub struct SunPlugin;

impl Plugin for SunPlugin {
    fn build(&self, app_builder: &mut AppBuilder) {
        let asset_server = app_builder.world().get_resource::<AssetServer>().unwrap();

        let vert: Handle<Shader> = asset_server.load("shaders/shadow.vert");
        let frag: Handle<Shader> = asset_server.load("shaders/shadow.frag");

        let shadow_pipeline = shadow_pipeline(ShaderStages {
            vertex: vert,
            fragment: Some(frag),
        });

        let vert: Handle<Shader> = asset_server.load("shaders/shaded.vert");
        let frag: Handle<Shader> = asset_server.load("shaders/shaded.frag");

        let shaded_pipeline = PipelineDescriptor::default_config(ShaderStages {
            vertex: vert,
            fragment: Some(frag),
        });

        app_builder
            .world_mut()
            .get_resource_mut::<Assets<PipelineDescriptor>>()
            .unwrap()
            .set_untracked(SHADOW_PIPELINE_HANDLE, shadow_pipeline);

        app_builder
            .world_mut()
            .get_resource_mut::<Assets<PipelineDescriptor>>()
            .unwrap()
            .set_untracked(SHADED_PIPELINE_HANDLE, shaded_pipeline);

        let texture_descriptor = TextureDescriptor {
            size: Extent3d::new(1024 * 8, 1024 * 8, 1),
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth24Plus,
            usage: TextureUsage::OUTPUT_ATTACHMENT | TextureUsage::SAMPLED,
        };
        let sampler_descriptor = SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        };

        let mut render_graph = app_builder
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap();
        render_graph.add_node(
            SHADOW_TEXTURE_NODE,
            TextureNode::new(texture_descriptor, sampler_descriptor, SHADOW_MAP_HANDLE),
        );
        render_graph.add_system_node(SUN_NODE, SunNode::default());
        render_graph.add_system_node(SHADOWS_NODE, ShadowsNode::<&ShadowCaster>::new());
        render_graph.add_system_node(SHADOW_MAP_NODE, ShadowMapNode::default());
        render_graph.add_system_node(
            "shadow_transform",
            crate::shadow_render_resources::ShadowRenderResourcesNode::<GlobalTransform>::new(true),
        );

        render_graph
            .add_node_edge("shadow_transform", SHADOWS_NODE)
            .unwrap();
        render_graph.add_node_edge(SUN_NODE, SHADOWS_NODE).unwrap();
        render_graph
            .add_node_edge(SUN_NODE, base::node::MAIN_PASS)
            .unwrap();

        render_graph
            .add_node_edge(SHADOWS_NODE, SHADOW_MAP_NODE)
            .unwrap();

        render_graph
            .add_node_edge(SHADOW_MAP_NODE, base::node::MAIN_PASS)
            .unwrap();

        render_graph
            .add_node_edge("transform", SHADOWS_NODE)
            .unwrap();

        render_graph
            .add_slot_edge(
                SHADOW_TEXTURE_NODE,
                "texture",
                SHADOWS_NODE,
                SHADOW_MAP_TEXTURE,
            )
            .unwrap();

        render_graph
            .add_slot_edge(SHADOW_TEXTURE_NODE, "texture", SHADOW_MAP_NODE, "texture")
            .unwrap();
        render_graph
            .add_slot_edge(SHADOW_TEXTURE_NODE, "sampler", SHADOW_MAP_NODE, "sampler")
            .unwrap();
    }
}

#[derive(Debug, Default)]
struct DrawState {
    pipeline: Option<Handle<PipelineDescriptor>>,
    bind_groups: Vec<Option<BindGroupId>>,
    vertex_buffers: Vec<Option<(BufferId, u64)>>,
    index_buffer: Option<(BufferId, u64, IndexFormat)>,
}

impl DrawState {
    pub fn set_bind_group(&mut self, index: u32, bind_group: BindGroupId) {
        self.bind_groups[index as usize] = Some(bind_group);
    }

    pub fn is_bind_group_set(&self, index: u32, bind_group: BindGroupId) -> bool {
        self.bind_groups[index as usize] == Some(bind_group)
    }

    pub fn set_vertex_buffer(&mut self, index: u32, buffer: BufferId, offset: u64) {
        self.vertex_buffers[index as usize] = Some((buffer, offset));
    }

    pub fn is_vertex_buffer_set(&self, index: u32, buffer: BufferId, offset: u64) -> bool {
        self.vertex_buffers[index as usize] == Some((buffer, offset))
    }

    pub fn set_index_buffer(&mut self, buffer: BufferId, offset: u64, index_format: IndexFormat) {
        self.index_buffer = Some((buffer, offset, index_format));
    }

    pub fn is_index_buffer_set(
        &self,
        buffer: BufferId,
        offset: u64,
        index_format: IndexFormat,
    ) -> bool {
        self.index_buffer == Some((buffer, offset, index_format))
    }

    pub fn can_draw(&self) -> bool {
        self.bind_groups.iter().all(|b| b.is_some())
            && self.vertex_buffers.iter().all(|v| v.is_some())
    }

    pub fn can_draw_indexed(&self) -> bool {
        self.can_draw() && self.index_buffer.is_some()
    }

    pub fn is_pipeline_set(&self, pipeline: Handle<PipelineDescriptor>) -> bool {
        self.pipeline == Some(pipeline)
    }

    pub fn set_pipeline(
        &mut self,
        handle: &Handle<PipelineDescriptor>,
        descriptor: &PipelineDescriptor,
    ) {
        self.bind_groups.clear();
        self.vertex_buffers.clear();
        self.index_buffer = None;

        self.pipeline = Some(handle.clone_weak());
        let layout = descriptor.get_layout().unwrap();
        self.bind_groups.resize(layout.bind_groups.len(), None);
        self.vertex_buffers
            .resize(layout.vertex_buffer_descriptors.len(), None);
    }
}
