use bevy::{
    core::AsBytes,
    ecs::system::BoxedSystem,
    prelude::*,
    reflect::TypeUuid,
    render::{
        camera::{ActiveCameras, Camera},
        pass::{
            LoadOp, Operations, PassDescriptor, RenderPassColorAttachmentDescriptor,
            RenderPassDepthStencilAttachmentDescriptor, TextureAttachment,
        },
        pipeline::{DepthStencilState, MultisampleState, PipelineDescriptor, RenderPipeline},
        render_graph::{
            base, AssetRenderResourcesNode, CameraNode, CommandQueue, Node, PassNode, RenderGraph,
            ResourceSlotInfo, ResourceSlots, SystemNode,
        },
        renderer::{
            BufferId, BufferInfo, BufferMapMode, BufferUsage, RenderContext, RenderResourceBinding,
            RenderResourceContext, RenderResourceId, RenderResourceType, RenderResources,
            SamplerId, TextureId,
        },
        shader::ShaderStages,
        texture::{
            Extent3d, SamplerDescriptor, TextureDescriptor, TextureDimension, TextureFormat,
            TextureUsage,
        },
    },
};
use std::sync::{Arc, Mutex};

const DEPTH_TEXTURE: &str = "shadow_depth";
const SHADOW_PIPELINE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 15348723);

pub struct SunNode {
    color: TextureDescriptor,
    attachment: TextureDescriptor,
    depth: TextureDescriptor,
}

impl SunNode {
    pub fn new(color: TextureDescriptor, attachment: TextureDescriptor, depth: TextureDescriptor) -> Self {
        Self { color, attachment, depth }
    }
}

impl Node for SunNode {
    fn output(&self) -> &[ResourceSlotInfo] {
        static OUTPUT: &[ResourceSlotInfo] = &[
            ResourceSlotInfo {
                name: std::borrow::Cow::Borrowed(DEPTH_TEXTURE),
                resource_type: RenderResourceType::Texture,
            },
            ResourceSlotInfo {
                name: std::borrow::Cow::Borrowed("color"),
                resource_type: RenderResourceType::Texture,
            },
            ResourceSlotInfo {
                name: std::borrow::Cow::Borrowed("attachment"),
                resource_type: RenderResourceType::Texture,
            },
        ];
        OUTPUT
    }

    fn update(
        &mut self,
        _world: &World,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        output: &mut ResourceSlots,
    ) {
        if output.get(DEPTH_TEXTURE).is_none() {
            let depth_texture = render_context.resources_mut().create_texture(self.depth);

            println!("init shadow_texture");

            output.set(DEPTH_TEXTURE, RenderResourceId::Texture(depth_texture));
        }

        if output.get("color").is_none() {
            let color_texture = render_context.resources_mut().create_texture(self.color);

            println!("init shadow_texture");

            output.set("color", RenderResourceId::Texture(color_texture));
        }

        if output.get("attachment").is_none() {
            let color_texture = render_context.resources_mut().create_texture(self.attachment);

            println!("init shadow_texture");

            output.set("attachment", RenderResourceId::Texture(color_texture));
        }
    }
}

pub struct ShadowPipelineSetNode;

impl Node for ShadowPipelineSetNode {
    fn prepare(&mut self, world: &mut World) {
        println!("Setting");

        let mut query = world.query::<(&mut RenderPipelines, &mut ShadowPipeline)>();

        for (mut render_pipelines, mut shadow_pipeline) in query.iter_mut(world) {
            let rp = std::mem::replace(
                &mut *render_pipelines,
                RenderPipelines::from_pipelines(vec![RenderPipeline::new(SHADOW_PIPELINE.typed())]),
            );

            if shadow_pipeline.0.is_none() {
                shadow_pipeline.0 = Some(rp);
            }
        }
    }

    fn update(
        &mut self,
        _world: &World,
        _render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        println!("Set update");
    }
}

#[derive(Default)]
pub struct ShadowPipeline(Option<RenderPipelines>);

pub struct ShadowPipelineUnsetNode;

impl Node for ShadowPipelineUnsetNode {
    fn prepare(&mut self, world: &mut World) {
        println!("Unsetting");

        let mut query = world.query::<(&mut RenderPipelines, &mut ShadowPipeline)>();

        for (mut render_pipelines, shadow_pipeline) in query.iter_mut(world) {
            if let Some(rp) = shadow_pipeline.0.clone() {
                *render_pipelines = rp;
            }
        }
    }

    fn update(
        &mut self,
        _world: &World,
        _render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        println!("Unset update");
    }
}

#[derive(Default)]
pub struct ShadowPass;

pub struct ShadowNode {
    command_queue: CommandQueue,
    shadow_texture: Arc<Mutex<Option<TextureId>>>,
}

#[derive(Debug, Default)]
pub struct ShadowNodeState {
    command_queue: CommandQueue,
    staging_buffer: Option<BufferId>,
    shadow_texture: Arc<Mutex<Option<TextureId>>>,
    shadow_texture_sampler: Option<SamplerId>,
}

impl ShadowNode {
    pub fn new() -> Self {
        Self {
            command_queue: Default::default(),
            shadow_texture: Arc::new(Mutex::new(None)),
        }
    }
}

impl Node for ShadowNode {
    fn input(&self) -> &[ResourceSlotInfo] {
        &[ResourceSlotInfo {
            name: std::borrow::Cow::Borrowed(DEPTH_TEXTURE),
            resource_type: RenderResourceType::Texture,
        }]
    }

    fn update(
        &mut self,
        _world: &World,
        render_context: &mut dyn RenderContext,
        input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        *self.shadow_texture.lock().unwrap() = input.get(DEPTH_TEXTURE).unwrap().get_texture();

        self.command_queue.execute(render_context);
    }
}

impl SystemNode for ShadowNode {
    fn get_system(&self) -> BoxedSystem {
        let system = shadow_node_system.system().config(|config| {
            config.0 = Some(ShadowNodeState {
                command_queue: self.command_queue.clone(),
                staging_buffer: None,
                shadow_texture: self.shadow_texture.clone(),
                shadow_texture_sampler: None,
            })
        });
        Box::new(system)
    }
}

const MATRIX_SIZE: usize = std::mem::size_of::<[[f32; 4]; 4]>();

pub fn shadow_node_system(
    mut state: Local<ShadowNodeState>,
    mut active_cameras: ResMut<ActiveCameras>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    query: Query<(&Camera, &GlobalTransform)>,
) {
    let (camera, transform) = if let Some(active_camera) = active_cameras.get("sun_camera") {
        if let Some(entity) = active_camera.entity {
            query.get(entity).unwrap().clone()
        } else {
            return;
        }
    } else {
        return;
    };

    let bindings = if let Some(active_camera) = active_cameras.get_mut(base::camera::CAMERA_3D) {
        &mut active_camera.bindings
    } else {
        return;
    };

    if let Some(shadow_texture) = *state.shadow_texture.lock().unwrap() {
        bindings.set(
            "ShadowTexture",
            RenderResourceBinding::Texture(shadow_texture),
        );
    }

    let sampler = if let Some(sampler) = state.shadow_texture_sampler {
        sampler
    } else {
        let descriptor = SamplerDescriptor::default();

        let sampler = render_resource_context.create_sampler(&descriptor);
        state.shadow_texture_sampler = Some(sampler);

        sampler
    };

    bindings.set(
        "ShadowTexture_sampler",
        RenderResourceBinding::Sampler(sampler),
    );

    let staging_buffer = if let Some(staging_buffer) = state.staging_buffer {
        render_resource_context.map_buffer(staging_buffer, BufferMapMode::Write);
        staging_buffer
    } else {
        let staging_buffer = render_resource_context.create_buffer(BufferInfo {
            size: MATRIX_SIZE,
            buffer_usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
            mapped_at_creation: true,
        });

        state.staging_buffer = Some(staging_buffer);
        staging_buffer
    };

    let buffer = if let Some(buffer) = bindings.get("SunCameraViewProj") {
        buffer.get_buffer().unwrap()
    } else {
        let buffer = render_resource_context.create_buffer(BufferInfo {
            size: MATRIX_SIZE,
            buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
            ..Default::default()
        });

        bindings.set(
            "SunCameraViewProj",
            RenderResourceBinding::Buffer {
                buffer,
                range: 0..MATRIX_SIZE as u64,
                dynamic_index: None,
            },
        );

        buffer
    };

    let view = transform.compute_matrix();
    let view_proj = camera.projection_matrix * view.inverse();

    render_resource_context.write_mapped_buffer(
        staging_buffer,
        0..MATRIX_SIZE as u64,
        &mut |data, _renderer| {
            data[0..MATRIX_SIZE].copy_from_slice(view_proj.to_cols_array_2d().as_bytes())
        },
    );
    state
        .command_queue
        .copy_buffer_to_buffer(staging_buffer, 0, buffer, 0, MATRIX_SIZE as u64);

    render_resource_context.unmap_buffer(staging_buffer);
}

#[derive(Default)]
pub struct Shadow(Option<Entity>);

pub fn shadow_spawn_system(
    mut commands: Commands,
    mut query: Query<(Entity, &Handle<Mesh>, &mut Shadow)>,
) {
    for (entity, mesh, mut shadow) in query.iter_mut() {
        if shadow.0.is_none() {
            let shadow_entity = commands
                .spawn()
                .insert(mesh.clone())
                .insert(RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                    SHADOW_PIPELINE.typed(),
                )]))
                .insert(Draw::default())
                .insert(Visible::default())
                .insert(ShadowPass)
                .insert(Transform::default())
                .insert(GlobalTransform::default())
                .insert(Parent(entity))
                .id();

            shadow.0 = Some(shadow_entity);
        }
    }
}

pub struct SunPlugin;

impl Plugin for SunPlugin {
    fn build(&self, app_builder: &mut AppBuilder) {
        app_builder.add_system(shadow_spawn_system.system());

        let asset_server = app_builder.world().get_resource::<AssetServer>().unwrap();

        let vert = asset_server.load("shaders/shadow.vert");
        let frag = asset_server.load("shaders/shadow.frag");

        let pipeline = PipelineDescriptor {
            ..PipelineDescriptor::default_config(ShaderStages {
                vertex: vert,
                fragment: Some(frag),
            })
        };

        app_builder
            .world_mut()
            .get_resource_mut::<Assets<PipelineDescriptor>>()
            .unwrap()
            .set_untracked(SHADOW_PIPELINE, pipeline);

        let mut active_cameras = app_builder
            .world_mut()
            .get_resource_mut::<ActiveCameras>()
            .unwrap();
        active_cameras.add("sun_camera");

        let mut render_graph = app_builder
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap();

        let color_descriptor = TextureDescriptor {
            size: Extent3d::new(4096, 4096, 1),
            mip_level_count: 1,
            sample_count: 8,
            dimension: TextureDimension::D2,
            format: TextureFormat::default(),
            usage: TextureUsage::SAMPLED | TextureUsage::OUTPUT_ATTACHMENT,
        };

        let attachment_descriptor = TextureDescriptor {
            size: Extent3d::new(4096, 4096, 1),
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::default(),
            usage: TextureUsage::SAMPLED | TextureUsage::OUTPUT_ATTACHMENT,
        };

        let depth_descriptor = TextureDescriptor {
            size: Extent3d::new(4096, 4096, 1),
            mip_level_count: 1,
            sample_count: 8,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsage::SAMPLED | TextureUsage::OUTPUT_ATTACHMENT,
        };

        let pass_descriptor = PassDescriptor {
            color_attachments: vec![RenderPassColorAttachmentDescriptor {
                attachment: TextureAttachment::Input("color".to_string()),
                resolve_target: Some(TextureAttachment::Input("attachment".to_string())),
                ops: Operations {
                    load: LoadOp::Clear(Color::rgba(0.0, 0.0, 0.0, 0.0)),
                    store: true,
                },
            }],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachmentDescriptor {
                attachment: TextureAttachment::Input("depth".to_string()),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
            sample_count: 1,
        };

        let mut sun_pass_node = PassNode::<&ShadowPass>::new(pass_descriptor);

        sun_pass_node.add_camera("sun_camera");

        render_graph.add_node("sun_node", SunNode::new(color_descriptor, attachment_descriptor, depth_descriptor));
        render_graph.add_node("sun_pass_node", sun_pass_node);
        render_graph.add_system_node("sun_camera_node", CameraNode::new("sun_camera"));
        render_graph.add_system_node("shadow_node", ShadowNode::new());

        render_graph.add_node_edge("transform", "sun_pass_node").unwrap();

        render_graph
            .add_node_edge("sun_pass_node", "shadow_node")
            .unwrap();
        render_graph
            .add_node_edge("shadow_node", base::node::MAIN_PASS)
            .unwrap();

        render_graph
            .add_node_edge(base::node::SHARED_BUFFERS, "sun_pass_node")
            .unwrap();
        render_graph
            .add_node_edge(base::node::TEXTURE_COPY, "sun_pass_node")
            .unwrap();

        render_graph
            .add_node_edge("sun_camera_node", "sun_pass_node")
            .unwrap();

        render_graph
            .add_slot_edge("sun_node", DEPTH_TEXTURE, "sun_pass_node", "depth")
            .unwrap();
        render_graph
            .add_slot_edge("sun_node", "color", "sun_pass_node", "color")
            .unwrap();
            render_graph
            .add_slot_edge("sun_node", "attachment", "sun_pass_node", "attachment")
            .unwrap();

        render_graph
            .add_slot_edge("sun_node", "attachment", "shadow_node", DEPTH_TEXTURE)
            .unwrap();
    }
}
