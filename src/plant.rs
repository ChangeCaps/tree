use crate::shadow_render_resources::*;
use crate::sun::*;
use bevy::{
    prelude::*,
    reflect::TypeUuid,
    render::{
        mesh::Indices,
        pipeline::{CullMode, PipelineDescriptor, PrimitiveState, RenderPipeline},
        render_graph::{base, RenderGraph, RenderResourcesNode},
        renderer::RenderResources,
        shader::ShaderStages,
    },
};
use rand::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, TypeUuid)]
#[uuid = "4192226a-c387-4719-a0e3-cbc936bf9961"]
pub struct Genome {
    pub seed: Option<u64>,
    pub max_splits: usize,
    pub branches_per_split: std::ops::Range<usize>,
    pub starting_radius: f32,
    pub radial_segments: usize,
    pub branch_length: f32,
    pub segments_per_branch: usize,
    pub radius_sustain: f32,
    pub leaf_start: usize,
    pub leaf_density: f32,
    pub leaf_size: f32,
    pub leaf_length: f32,
    pub leaf_offset: f32,
    pub branch_decay: usize,
    pub branch_bend: f32,
    pub branch_sway: f32,
    pub branch_twist: f32,
}

impl Genome {
    pub fn generate_mesh(&self) -> Mesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut sway = Vec::new();
        let mut color = Vec::new();
        let mut uv = Vec::new();
        let mut material = Vec::new();
        let mut leaves = 0;

        let mut rng = if let Some(seed) = &self.seed {
            rand::rngs::SmallRng::seed_from_u64(*seed)
        } else {
            rand::rngs::SmallRng::from_rng(thread_rng()).unwrap()
        };

        let mut ctx = PlantContext {
            vertices: &mut vertices,
            indices: &mut indices,
            sway: &mut sway,
            color: &mut color,
            uv: &mut uv,
            material: &mut material,
            leaves: &mut leaves,
            rng: &mut rng,
        };

        let ring = Ring::generate(self.starting_radius, self.radial_segments, 0.0);

        let start_loop = ctx.add_ring(ring, 0.0);

        let branch = Branch {
            start_loop,
            ..Branch::generate(self)
        };

        let mut branches = vec![branch];

        for _ in 0..self.max_splits {
            for branch in std::mem::replace(&mut branches, Vec::new()) {
                branches.append(&mut branch.generate_mesh(&mut ctx, self));
            }
        }

        let mut mesh = Mesh::new(Default::default());
        let mut normals = vec![Vec3::ZERO; vertices.len()];

        for i in 0..indices.len() / 3 {
            let i0 = indices[i * 3 + 0] as usize;
            let i1 = indices[i * 3 + 1] as usize;
            let i2 = indices[i * 3 + 2] as usize;

            let v0 = vertices[i0];
            let v1 = vertices[i1];
            let v2 = vertices[i2];

            let normal = (v1 - v0).cross(v2 - v0);

            normals[i0] += normal;
            normals[i1] += normal;
            normals[i2] += normal;
        }

        for normal in &mut normals {
            *normal = normal.normalize();
        }

        println!("Tree:");
        println!(" tris: {}", indices.len() / 3);
        println!(" verts: {}", vertices.len());
        println!(" leaves: {}", leaves);
        println!(" leaf_tris: {}", leaves * 2);

        mesh.set_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vertices
                .into_iter()
                .map(|v| v.into())
                .collect::<Vec<[f32; 3]>>(),
        );
        mesh.set_attribute(
            Mesh::ATTRIBUTE_UV_0,
            uv.into_iter().map(|v| v.into()).collect::<Vec<[f32; 2]>>(),
        );
        mesh.set_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            normals
                .into_iter()
                .map(|v| v.into())
                .collect::<Vec<[f32; 3]>>(),
        );
        mesh.set_attribute("Plant_Material", material);
        mesh.set_attribute("Plant_Sway", sway);
        mesh.set_attribute(
            "Vertex_Color",
            color
                .into_iter()
                .map(|v| v.into())
                .collect::<Vec<[f32; 4]>>(),
        );
        mesh.set_indices(Some(Indices::U32(indices)));

        mesh
    }
}

pub struct Ring {
    pub radius: f32,
    pub verts: Vec<Vec3>,
    pub sway: Vec<f32>,
}

impl Ring {
    pub fn generate(radius: f32, segments: usize, sway: f32) -> Self {
        let mut verts = Vec::with_capacity(segments);

        for i in 0..segments {
            let i = i as f32 / segments as f32 * std::f32::consts::TAU;

            let v = Vec3::new(i.cos(), 0.0, i.sin()) * radius;

            verts.push(v);
        }

        Self {
            radius,
            verts,
            sway: vec![sway; segments],
        }
    }

    pub fn translate(&mut self, vec: Vec3) {
        for v in &mut self.verts {
            *v += vec;
        }
    }

    pub fn rotate(&mut self, rot: Vec3) {
        for v in &mut self.verts {
            *v = rotate(*v, rot);
        }
    }
}

pub struct PlantContext<'a> {
    pub vertices: &'a mut Vec<Vec3>,
    pub indices: &'a mut Vec<u32>,
    pub sway: &'a mut Vec<f32>,
    pub color: &'a mut Vec<Color>,
    pub uv: &'a mut Vec<Vec2>,
    pub material: &'a mut Vec<u32>,
    pub leaves: &'a mut usize,
    pub rng: &'a mut rand::rngs::SmallRng,
}

impl PlantContext<'_> {
    pub fn add_ring(&mut self, ring: Ring, v: f32) -> Vec<u32> {
        (0..ring.verts.len()).into_iter().for_each(|i| {
            let u = (1.0 - i as f32 / ring.verts.len() as f32 * 2.0).abs() * ring.radius;

            self.uv.push(Vec2::new(u, v));
            self.color.push(Color::rgb(1.0, 1.0, 1.0));
            self.material.push(0);
        });
        ring.sway.into_iter().for_each(|s| self.sway.push(s));
        ring.verts
            .into_iter()
            .map(|v| {
                self.vertices.push(v);
                self.vertices.len() as u32 - 1
            })
            .collect()
    }
}

pub struct Branch {
    pub split: usize,
    pub branch_decay: usize,
    pub start_radius: f32,
    pub end_radius: f32,
    pub length: f32,
    pub start: Vec3,
    pub direction: Vec3,
    pub bend: Vec3,
    pub segments: usize,
    pub radial_segments: usize,
    pub start_loop: Vec<u32>,
    pub sway: f32,
}

fn rotate(vec: Vec3, rot: Vec3) -> Vec3 {
    Quat::from_rotation_ypr(rot.y, rot.x, rot.z) * vec
}

fn lerp(a: f32, b: f32, mix: f32) -> f32 {
    a * mix + b * (1.0 - mix)
}

impl Branch {
    pub fn generate(genome: &Genome) -> Self {
        Self {
            split: 0,
            branch_decay: 0,
            start_radius: genome.starting_radius,
            end_radius: genome.starting_radius * genome.radius_sustain,
            length: genome.branch_length,
            start: Vec3::ZERO,
            direction: Vec3::ZERO,
            bend: Vec3::ZERO,
            segments: genome.segments_per_branch,
            radial_segments: genome.radial_segments,
            start_loop: Vec::new(),
            sway: 0.0,
        }
    }

    pub fn generate_mesh(&self, ctx: &mut PlantContext<'_>, genome: &Genome) -> Vec<Branch> {
        let segment_length = 1.0 / self.segments as f32 * self.length;
        let mut pos = self.start;
        let mut bend = self.direction;

        let mut prev_loop = self.start_loop.clone();

        for segment in 1..=self.segments {
            let segment_lerp = segment as f32 / self.segments as f32;

            bend += self.bend * (1.0 / self.segments as f32);

            pos += rotate(Vec3::Y, bend) * segment_length;

            let radius = lerp(self.end_radius, self.start_radius, segment_lerp);
            let sway = self.sway + self.length * segment_lerp;

            let mut ring = Ring::generate(radius, self.radial_segments, sway);

            ring.rotate(bend);
            ring.translate(pos);

            if self.split >= genome.leaf_start {
                for vert in &ring.verts {
                    if ctx.rng.gen_range(0.0..1.0)
                        > genome.leaf_density / self.segments as f32 / self.radial_segments as f32
                    {
                        continue;
                    }

                    let mut o = || {
                        let r = radius.max(0.01) * genome.leaf_offset;

                        ctx.rng.gen_range(-r..r)
                    };

                    let diff = (*vert + Vec3::new(o(), o(), o())) - pos;
                    let up = Vec3::new(
                        ctx.rng.gen_range(-3.14..3.14),
                        1.0,
                        ctx.rng.gen_range(-3.14..3.14),
                    );

                    let forward = diff.normalize();
                    let right = up.cross(forward).normalize();
                    let up = forward.cross(right);

                    let rot = Quat::from_rotation_mat3(&Mat3::from_cols(right, up, forward));

                    let leaf = Leaf {
                        pos: *vert,
                        rot,
                        sway,
                        size: genome.leaf_size,
                        length: genome.leaf_length,
                    };

                    leaf.generate_mesh(ctx);
                }
            }

            let mut indices = ctx.add_ring(ring, sway);
            let len = indices.len();
            indices.rotate_left((bend.y / std::f32::consts::TAU * len as f32) as usize % len);

            self.bridge_loops(ctx, &indices, &prev_loop);

            prev_loop = indices;
        }

        let mut splits = ctx
            .rng
            .gen_range(genome.branches_per_split.start..=genome.branches_per_split.end);

        splits = splits.saturating_sub(self.branch_decay).max(1);

        (0..splits)
            .into_iter()
            .map(|i| {
                let mut new_bend = self.bend;
                let mut new_direction = bend;

                if self.split == 0 {
                    let angle = 1.0 / splits as f32 * std::f32::consts::TAU;
                    let dir = i as f32 / splits as f32 * std::f32::consts::TAU;

                    new_direction.y += dir + ctx.rng.gen_range(0.0..angle * 0.8);
                } else {
                    let angle = 1.0 / splits as f32 * genome.branch_sway * 0.8;
                    let dir = (i as f32 / splits as f32 - 0.5) * genome.branch_sway * 2.0;

                    new_direction.y += dir + ctx.rng.gen_range(-angle..angle);
                }

                if genome.branch_twist != 0.0 {
                    new_bend.z += ctx.rng.gen_range(-genome.branch_twist..genome.branch_twist);
                }

                let bend = ctx.rng.gen_range(0.0..genome.branch_bend);

                new_direction.x += bend * (2.0 / 3.0);
                new_bend.x += bend * (1.0 / 3.0);

                let end_radius = if self.split == genome.max_splits - 2 {
                    0.0
                } else {
                    self.end_radius * genome.radius_sustain
                };

                let radial_segments = if self.split == 1 && self.radial_segments % 2 == 0 {
                    self.radial_segments / 2
                } else {
                    self.radial_segments
                };

                Branch {
                    split: self.split + 1,
                    branch_decay: self.branch_decay + genome.branch_decay,
                    start: pos,
                    start_radius: self.end_radius,
                    end_radius,
                    start_loop: prev_loop.clone(),
                    direction: new_direction,
                    bend: new_bend,
                    sway: self.sway + self.length,
                    radial_segments,
                    ..Branch::generate(genome)
                }
            })
            .collect()
    }

    pub fn bridge_loops(&self, ctx: &mut PlantContext<'_>, loop_a: &Vec<u32>, loop_b: &Vec<u32>) {
        if loop_a.len() == loop_b.len() {
            for a in 0..loop_a.len() {
                let a_next = (a + 1) % loop_a.len();
                let b = a;
                let b_next = (b + 1) % loop_a.len();

                ctx.indices.push(loop_b[b]);
                ctx.indices.push(loop_a[a]);
                ctx.indices.push(loop_a[a_next]);

                ctx.indices.push(loop_a[a_next]);
                ctx.indices.push(loop_b[b_next]);
                ctx.indices.push(loop_b[b]);
            }
        } else if loop_a.len() * 2 == loop_b.len() {
            for a in 0..loop_a.len() {
                let a_1 = (a + 1) % loop_a.len();
                let b = a * 2;
                let b_1 = (b + 1) % loop_b.len();
                let b_2 = (b + 2) % loop_b.len();

                ctx.indices.push(loop_b[b]);
                ctx.indices.push(loop_a[a]);
                ctx.indices.push(loop_b[b_1]);

                ctx.indices.push(loop_a[a]);
                ctx.indices.push(loop_b[b_1]);
                ctx.indices.push(loop_a[a_1]);

                ctx.indices.push(loop_a[a_1]);
                ctx.indices.push(loop_b[b_1]);
                ctx.indices.push(loop_b[b_2]);
            }
        }
    }
}

pub struct Leaf {
    pub pos: Vec3,
    pub rot: Quat,
    pub sway: f32,
    pub size: f32,
    pub length: f32,
}

impl Leaf {
    pub fn generate_mesh(&self, ctx: &mut PlantContext<'_>) -> Vec<u32> {
        *ctx.leaves += 1;

        let mut verts = vec![
            Vec3::new(-0.5, 0.0, 0.0),
            Vec3::new(0.5, 0.0, 0.0),
            Vec3::new(-0.5, 0.0, 1.0),
            Vec3::new(0.5, 0.0, 1.0),
        ];

        ctx.uv.push(Vec2::new(1.0, 1.0));
        ctx.uv.push(Vec2::new(1.0, 0.0));
        ctx.uv.push(Vec2::new(0.0, 1.0));
        ctx.uv.push(Vec2::new(0.0, 0.0));

        verts.iter_mut().for_each(|v| {
            *v *= self.size;
            v.y *= self.length;
            *v = self.rot * *v;
            *v += self.pos;
        });

        (0..4).into_iter().for_each(|_| {
            ctx.color.push(Color::rgb(1.0, 1.0, 1.0));
            ctx.sway.push(self.sway);
            ctx.material.push(1);
        });
        let indices = verts
            .into_iter()
            .map(|v| {
                ctx.vertices.push(v);
                (ctx.vertices.len() - 1) as u32
            })
            .collect::<Vec<u32>>();

        ctx.indices.push(indices[0]);
        ctx.indices.push(indices[1]);
        ctx.indices.push(indices[2]);

        ctx.indices.push(indices[1]);
        ctx.indices.push(indices[3]);
        ctx.indices.push(indices[2]);

        indices
    }
}

#[derive(Default, RenderResources, TypeUuid)]
#[uuid = "5739c0cc-eefb-4e41-b2fc-0e8d937fcff7"]
pub struct PlantMaterial {
    pub time: f32,
    pub growth: f32,
    pub texture: Handle<Texture>,
    pub leaf_front: Handle<Texture>,
}

impl PlantMaterial {
    pub fn new(texture: Handle<Texture>, leaf_front: Handle<Texture>) -> Self {
        Self {
            texture,
            leaf_front,
            ..Default::default()
        }
    }
}

#[derive(Bundle)]
pub struct PlantBundle {
    pub material: PlantMaterial,
    pub main_pass: base::MainPass,
    pub draw: Draw,
    pub visible: Visible,
    pub render_pipelines: RenderPipelines,
    pub shadow_caster: ShadowCaster,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for PlantBundle {
    fn default() -> Self {
        Self {
            material: Default::default(),
            main_pass: Default::default(),
            draw: Default::default(),
            visible: Default::default(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                PIPELINE.typed(),
            )]),
            shadow_caster: ShadowCaster::new(RenderPipelines::from_pipelines(vec![
                RenderPipeline::new(SHADOW_PIPELINE.typed()),
            ])),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

pub fn plant_material_system(time: Res<Time>, mut query: Query<&mut PlantMaterial>) {
    for mut plant_material in query.iter_mut() {
        plant_material.time += time.delta_seconds();
    }
}

pub struct GenomeLoader;

impl bevy::asset::AssetLoader for GenomeLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let asset = ron::de::from_bytes::<Genome>(bytes).map_err(|e| {
                anyhow::Error::msg(format!(
                    "'{}': {}",
                    load_context.path().to_string_lossy(),
                    e
                ))
            })?;

            load_context.set_default_asset(bevy::asset::LoadedAsset::new(asset));

            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["gno"]
    }
}

pub const PIPELINE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 562348753649);
pub const SHADOW_PIPELINE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 69349823467);

pub struct PlantPlugin;

impl Plugin for PlantPlugin {
    fn build(&self, app_builder: &mut AppBuilder) {
        app_builder.add_asset::<Genome>();
        app_builder.add_asset::<PlantMaterial>();
        app_builder.add_asset_loader(GenomeLoader);
        app_builder.add_system(plant_material_system.system());

        let asset_server = app_builder.world().get_resource::<AssetServer>().unwrap();
        asset_server.watch_for_changes().unwrap();

        let vert = asset_server.load("shaders/plant.vert");
        let frag = asset_server.load("shaders/plant.frag");

        let pipeline = PipelineDescriptor {
            primitive: PrimitiveState {
                cull_mode: CullMode::None,
                ..Default::default()
            },
            ..PipelineDescriptor::default_config(ShaderStages {
                vertex: vert,
                fragment: Some(frag),
            })
        };

        let vert = asset_server.load("shaders/plant_shadow.vert");
        let frag = asset_server.load("shaders/plant_shadow.frag");

        let shadow_pipeline = shadow_pipeline(ShaderStages {
            vertex: vert,
            fragment: Some(frag),
        });

        app_builder
            .world_mut()
            .get_resource_mut::<Assets<PipelineDescriptor>>()
            .unwrap()
            .set_untracked(PIPELINE, pipeline);

        app_builder
            .world_mut()
            .get_resource_mut::<Assets<PipelineDescriptor>>()
            .unwrap()
            .set_untracked(SHADOW_PIPELINE, shadow_pipeline);

        let mut render_graph = app_builder
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap();

        render_graph.add_system_node(
            "plant_shadow_material",
            ShadowRenderResourcesNode::<PlantMaterial>::new(true),
        );
        render_graph
            .add_node_edge("plant_shadow_material", SHADOWS_NODE)
            .unwrap();

        render_graph.add_system_node(
            "plant_material",
            RenderResourcesNode::<PlantMaterial>::new(true),
        );
        render_graph
            .add_node_edge("plant_material", base::node::MAIN_PASS)
            .unwrap();
    }
}
