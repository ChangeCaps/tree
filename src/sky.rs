use bevy::{
    app::{Events, ManualEventReader},
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
            base, CameraNode, CommandQueue, Node, PassNode, RenderGraph, RenderResourcesNode,
            ResourceSlotInfo, ResourceSlots, SystemNode, WindowSwapChainNode, WindowTextureNode,
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
    window::{WindowCreated, WindowId, WindowResized},
};
use std::sync::{Arc, Mutex};

pub const SKY_PASS_NODE: &str = "sky_pass_node";
pub const SKY_PASS_DEPTH_NODE: &str = "sky_pass_depth";
pub const SKY_PASS_TEXTURE_NODE: &str = "sky_pass_texture";
pub const VOLUME_PASS_NODE: &str = "volume_pass_node";
pub const VOLUME_PASS_TEXTURE: &str = "volume_pass_texture";
pub const VOLUME_PASS_TEXTURE_NODE: &str = "volume_pass_texture_binding";
pub const MAIN_PASS_DEPTH_NODE: &str = "main_pass_depth";
pub const MAIN_PASS_TEXTURE_NODE: &str = "main_pass_texture";
pub const POST_DATA_NODE: &str = "pass_data_node";
pub const SKY_PIPELINE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 956872324);
pub const POST_PIPELINE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 82346125);
pub const VOLUME_PIPELINE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 82345436798);

pub struct ScaledWindowTextureNode {
    window_id: WindowId,
    descriptor: TextureDescriptor,
    window_created_event_reader: ManualEventReader<WindowCreated>,
    window_resized_event_reader: ManualEventReader<WindowResized>,
    scale: u32,
}

impl ScaledWindowTextureNode {
    pub const OUT_TEXTURE: &'static str = "texture";

    pub fn new(window_id: WindowId, descriptor: TextureDescriptor, scale: u32) -> Self {
        Self {
            window_id,
            descriptor,
            window_created_event_reader: Default::default(),
            window_resized_event_reader: Default::default(),
            scale,
        }
    }
}

impl Node for ScaledWindowTextureNode {
    fn output(&self) -> &[ResourceSlotInfo] {
        &[ResourceSlotInfo {
            name: std::borrow::Cow::Borrowed(ScaledWindowTextureNode::OUT_TEXTURE),
            resource_type: RenderResourceType::Texture,
        }]
    }

    fn update(
        &mut self,
        world: &World,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        output: &mut ResourceSlots,
    ) {
        let window_created_events = world.get_resource::<Events<WindowCreated>>().unwrap();
        let window_resized_events = world.get_resource::<Events<WindowResized>>().unwrap();
        let windows = world.get_resource::<Windows>().unwrap();

        let window = windows
            .get(self.window_id)
            .expect("Window texture node refers to a non-existent window.");

        if self
            .window_created_event_reader
            .iter(&window_created_events)
            .any(|e| e.id == window.id())
            || self
                .window_resized_event_reader
                .iter(&window_resized_events)
                .any(|e| e.id == window.id())
        {
            let render_resource_context = render_context.resources_mut();
            if let Some(RenderResourceId::Texture(old_texture)) =
                output.get(ScaledWindowTextureNode::OUT_TEXTURE)
            {
                render_resource_context.remove_texture(old_texture);
            }

            self.descriptor.size.width = window.physical_width() / self.scale;
            self.descriptor.size.height = window.physical_height() / self.scale;
            let texture_resource = render_resource_context.create_texture(self.descriptor);
            output.set(
                ScaledWindowTextureNode::OUT_TEXTURE,
                RenderResourceId::Texture(texture_resource),
            );
        }
    }
}
pub struct TextureBindNode {
    texture: Option<TextureId>,
    sampler: Option<SamplerId>,
    sampler_descriptor: SamplerDescriptor,
    texture_name: &'static str,
    sampler_name: &'static str,
}

impl TextureBindNode {
    const IN_TEXTURE: &'static str = "texture";

    pub fn new(
        sampler_descriptor: SamplerDescriptor,
        texture_name: &'static str,
        sampler_name: &'static str,
    ) -> Self {
        Self {
            texture: Default::default(),
            sampler: Default::default(),
            sampler_descriptor,
            texture_name,
            sampler_name,
        }
    }
}

impl Node for TextureBindNode {
    fn input(&self) -> &[ResourceSlotInfo] {
        &[ResourceSlotInfo {
            name: std::borrow::Cow::Borrowed(Self::IN_TEXTURE),
            resource_type: RenderResourceType::Texture,
        }]
    }

    fn prepare(&mut self, world: &mut World) {
        let mut active_cameras = world.get_resource_mut::<ActiveCameras>().unwrap();

        for active_camera in active_cameras.iter_mut() {
            if let Some(texture) = self.texture {
                active_camera
                    .bindings
                    .set(self.texture_name, RenderResourceBinding::Texture(texture));
            }

            if let Some(sampler) = self.sampler {
                active_camera
                    .bindings
                    .set(self.sampler_name, RenderResourceBinding::Sampler(sampler));
            }
        }
    }

    fn update(
        &mut self,
        _world: &World,
        render_context: &mut dyn RenderContext,
        input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        let render_resource_context = render_context.resources();

        if self.sampler.is_none() {
            let sampler = render_resource_context.create_sampler(&self.sampler_descriptor);

            self.sampler = Some(sampler);
        }

        if let Some(RenderResourceId::Texture(texture)) = input.get(Self::IN_TEXTURE) {
            self.texture = Some(texture);
        }
    }
}

pub struct PostPass;
pub struct VolumePass;

#[derive(RenderResources)]
pub struct PostData {}

#[derive(Bundle)]
pub struct PostBundle {
    pub post_data: PostData,
    pub mesh: Handle<Mesh>,
    pub post_pass: PostPass,
    pub draw: Draw,
    pub visible: Visible,
    pub render_pipelines: RenderPipelines,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for PostBundle {
    fn default() -> Self {
        Self {
            post_data: PostData {},
            mesh: bevy::sprite::QUAD_HANDLE.typed(),
            post_pass: PostPass,
            draw: Default::default(),
            visible: Default::default(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                POST_PIPELINE.typed(),
            )]),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

pub struct Plugins;

impl PluginGroup for Plugins {
    fn build(&mut self, group: &mut bevy::app::PluginGroupBuilder) {
        group.add(bevy::log::LogPlugin::default());
        group.add(bevy::core::CorePlugin::default());
        group.add(bevy::transform::TransformPlugin::default());
        group.add(bevy::diagnostic::DiagnosticsPlugin::default());
        group.add(bevy::input::InputPlugin::default());
        group.add(bevy::window::WindowPlugin::default());
        group.add(bevy::asset::AssetPlugin::default());
        group.add(bevy::scene::ScenePlugin::default());
        group.add(bevy::render::RenderPlugin {
            base_render_graph_config: Some(base::BaseRenderGraphConfig {
                connect_main_pass_to_swapchain: false,
                connect_main_pass_to_main_depth_texture: false,
                ..Default::default()
            }),
        });
        group.add(bevy::sprite::SpritePlugin::default());
        group.add(bevy::pbr::PbrPlugin::default());
        group.add(bevy::ui::UiPlugin::default());
        group.add(bevy::text::TextPlugin::default());
        group.add(bevy::audio::AudioPlugin::default());
        group.add(bevy::gilrs::GilrsPlugin::default());
        group.add(bevy::gltf::GltfPlugin::default());
        group.add(bevy::winit::WinitPlugin::default());
        group.add(bevy::wgpu::WgpuPlugin::default());
        group.add(SkyPlugin);
    }
}

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app_builder: &mut AppBuilder) {
        let asset_server = app_builder.world().get_resource::<AssetServer>().unwrap();
        asset_server.watch_for_changes().unwrap();

        let vert = asset_server.load("shaders/sky.vert");
        let frag = asset_server.load("shaders/sky.frag");

        let sky_pipeline = PipelineDescriptor {
            ..PipelineDescriptor::default_config(ShaderStages {
                vertex: vert,
                fragment: Some(frag),
            })
        };

        let vert = asset_server.load("shaders/post.vert");
        let frag = asset_server.load("shaders/post.frag");

        let post_pipeline = PipelineDescriptor {
            ..PipelineDescriptor::default_config(ShaderStages {
                vertex: vert,
                fragment: Some(frag),
            })
        };

        let vert = asset_server.load("shaders/volume.vert");
        let frag = asset_server.load("shaders/volume.frag");

        let volume_pipeline = PipelineDescriptor {
            depth_stencil: None,
            ..PipelineDescriptor::default_config(ShaderStages {
                vertex: vert,
                fragment: Some(frag),
            })
        };

        app_builder
            .world_mut()
            .get_resource_mut::<Assets<PipelineDescriptor>>()
            .unwrap()
            .set_untracked(SKY_PIPELINE, sky_pipeline);

        app_builder
            .world_mut()
            .get_resource_mut::<Assets<PipelineDescriptor>>()
            .unwrap()
            .set_untracked(POST_PIPELINE, post_pipeline);

        app_builder
            .world_mut()
            .get_resource_mut::<Assets<PipelineDescriptor>>()
            .unwrap()
            .set_untracked(VOLUME_PIPELINE, volume_pipeline);

        let msaa = app_builder.world().get_resource::<Msaa>().unwrap();
        let samples = msaa.samples;

        let mut sky_pass_node = PassNode::<&PostPass>::new(PassDescriptor {
            color_attachments: vec![msaa.color_attachment_descriptor(
                TextureAttachment::Input("color_attachment".to_string()),
                TextureAttachment::Input("color_resolve_target".to_string()),
                Operations {
                    load: LoadOp::Load,
                    store: true,
                },
            )],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachmentDescriptor {
                attachment: TextureAttachment::Input("depth".to_string()),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
            sample_count: msaa.samples,
        });

        sky_pass_node.add_camera(base::camera::CAMERA_3D);

        let mut volume_pass_node = PassNode::<&VolumePass>::new(PassDescriptor {
            color_attachments: vec![RenderPassColorAttachmentDescriptor {
                attachment: TextureAttachment::Input("color_attachment".to_string()),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::rgba(0.0, 0.0, 0.0, 0.0)),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
            sample_count: 1,
        });

        volume_pass_node.add_camera(base::camera::CAMERA_3D);

        let mut render_graph = app_builder
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap();

        render_graph.add_node(SKY_PASS_NODE, sky_pass_node);
        render_graph.add_node(VOLUME_PASS_NODE, volume_pass_node);

        render_graph.add_node(
            VOLUME_PASS_TEXTURE,
            ScaledWindowTextureNode::new(
                WindowId::primary(),
                TextureDescriptor {
                    size: Extent3d {
                        depth: 1,
                        width: 1,
                        height: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::default(),
                    usage: TextureUsage::SAMPLED | TextureUsage::OUTPUT_ATTACHMENT,
                },
                2,
            ),
        );

        render_graph.add_node(
            SKY_PASS_DEPTH_NODE,
            WindowTextureNode::new(
                WindowId::primary(),
                TextureDescriptor {
                    size: Extent3d {
                        depth: 1,
                        width: 1,
                        height: 1,
                    },
                    mip_level_count: 1,
                    sample_count: samples,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Depth32Float,
                    usage: TextureUsage::SAMPLED | TextureUsage::OUTPUT_ATTACHMENT,
                },
            ),
        );

        render_graph.add_node(
            SKY_PASS_TEXTURE_NODE,
            WindowTextureNode::new(
                WindowId::primary(),
                TextureDescriptor {
                    size: Extent3d::new(1, 1, 1),
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::default(),
                    usage: TextureUsage::SAMPLED | TextureUsage::OUTPUT_ATTACHMENT,
                },
            ),
        );

        let sampler_descriptor = SamplerDescriptor {
            ..Default::default()
        };

        render_graph.add_node(
            MAIN_PASS_TEXTURE_NODE,
            TextureBindNode::new(
                sampler_descriptor,
                "SkyPassTexture",
                "SkyPassTextureSampler",
            ),
        );

        render_graph.add_node(
            VOLUME_PASS_TEXTURE_NODE,
            TextureBindNode::new(
                sampler_descriptor,
                "VolumePassTexture",
                "VolumePassTextureSampler",
            ),
        );

        render_graph.add_node(
            MAIN_PASS_DEPTH_NODE,
            TextureBindNode::new(sampler_descriptor, "SkyPassDepth", "SkyPassDepthSampler"),
        );

        render_graph.add_system_node(POST_DATA_NODE, RenderResourcesNode::<PostData>::new(false));

        render_graph
            .add_node_edge(POST_DATA_NODE, SKY_PASS_NODE)
            .unwrap();

        render_graph
            .add_slot_edge(
                VOLUME_PASS_TEXTURE,
                ScaledWindowTextureNode::OUT_TEXTURE,
                VOLUME_PASS_NODE,
                "color_attachment",
            )
            .unwrap();

        render_graph
            .add_slot_edge(
                SKY_PASS_DEPTH_NODE,
                WindowTextureNode::OUT_TEXTURE,
                base::node::MAIN_PASS,
                "depth",
            )
            .unwrap();

        render_graph
            .add_slot_edge(
                VOLUME_PASS_TEXTURE,
                ScaledWindowTextureNode::OUT_TEXTURE,
                VOLUME_PASS_TEXTURE_NODE,
                TextureBindNode::IN_TEXTURE,
            )
            .unwrap();

        render_graph
            .add_node_edge(VOLUME_PASS_TEXTURE_NODE, SKY_PASS_NODE)
            .unwrap();

        render_graph
            .add_slot_edge(
                SKY_PASS_DEPTH_NODE,
                WindowTextureNode::OUT_TEXTURE,
                MAIN_PASS_DEPTH_NODE,
                TextureBindNode::IN_TEXTURE,
            )
            .unwrap();

        render_graph
            .add_node_edge(MAIN_PASS_DEPTH_NODE, VOLUME_PASS_NODE)
            .unwrap();

        render_graph
            .add_slot_edge(
                SKY_PASS_TEXTURE_NODE,
                WindowTextureNode::OUT_TEXTURE,
                MAIN_PASS_TEXTURE_NODE,
                TextureBindNode::IN_TEXTURE,
            )
            .unwrap();

        render_graph
            .add_node_edge(MAIN_PASS_TEXTURE_NODE, SKY_PASS_NODE)
            .unwrap();

        render_graph
            .add_node_edge(base::node::MAIN_PASS, VOLUME_PASS_NODE)
            .unwrap();
        render_graph
            .add_node_edge(VOLUME_PASS_NODE, SKY_PASS_NODE)
            .unwrap();
        render_graph
            .add_node_edge(SKY_PASS_NODE, bevy::ui::node::UI_PASS)
            .unwrap();

        render_graph
            .add_slot_edge(
                SKY_PASS_TEXTURE_NODE,
                WindowTextureNode::OUT_TEXTURE,
                base::node::MAIN_PASS,
                "color_attachment",
            )
            .unwrap();

        render_graph
            .add_slot_edge(
                base::node::PRIMARY_SWAP_CHAIN,
                WindowSwapChainNode::OUT_TEXTURE,
                SKY_PASS_NODE,
                "color_attachment",
            )
            .unwrap();

        render_graph
            .add_slot_edge(
                base::node::MAIN_DEPTH_TEXTURE,
                WindowSwapChainNode::OUT_TEXTURE,
                SKY_PASS_NODE,
                "depth",
            )
            .unwrap();
    }
}
