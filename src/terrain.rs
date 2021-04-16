use bevy::prelude::*;

pub const CHUNK_SIZE: f32 = 5.0;
pub const CHUNK_RESOLUTION: usize = 32;
pub const TERRAIN_CHUNK_DEPTH: usize = 3;

pub fn terrain_system(
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain_query: Query<(&mut Terrain, &mut Transform, &Handle<Mesh>)>,
) {
    for (mut terrain, mut transform, mesh) in terrain_query.iter_mut() {
        let chunk = terrain.generate(
            Vec2::new(transform.translation.x, transform.translation.x),
            CHUNK_SIZE,
        );

        transform.translation.x = chunk.x;
        transform.translation.z = chunk.y;
    }
}

pub struct Terrain {
    height_fn: Box<dyn Fn(Vec2) -> f32 + Send + Sync>,
    chunks: [[TerrainChunk; 3]; 3],
    child: TerrainNode,
    position: Option<Vec2>,
}

impl Terrain {
    pub fn new(height_fn: impl Fn(Vec2) -> f32 + Send + Sync + 'static) -> Self {
        Self {
            height_fn: Box::new(height_fn),
            chunks: [[TerrainChunk::new(); 3]; 3],
            child: TerrainNode::new(0),
            position: None,
        }
    }

    pub fn generate(&mut self, position: Vec2, size: f32) -> Vec2 {
        let chunk = (position / size).floor() * size;

        if self.position == Some(chunk) {
            return chunk;
        }

        self.position = Some(chunk);

        for (x, column) in self.chunks.iter_mut().enumerate() {
            for (y, chunk) in column.iter_mut().enumerate() {
                chunk.generate(
                    position + Vec2::new(x as f32 * size - size, y as f32 * size - size),
                    size,
                    &self.height_fn,
                );
            }
        }

        self.child.generate(position, size, &self.height_fn);

        chunk
    }

    pub fn generate_mesh(&self, ) {

    }
}

pub struct TerrainNode {
    child: Box<Option<TerrainNode>>,
    top_left: TerrainChunk,
    top_mid: TerrainChunk,
    top_right: TerrainChunk,
    mid_left: TerrainChunk,
    mid_right: TerrainChunk,
    bottom_left: TerrainChunk,
    bottom_mid: TerrainChunk,
    bottom_right: TerrainChunk,
}

impl TerrainNode {
    pub fn new(depth: usize) -> Self {
        Self {
            child: if depth <= TERRAIN_CHUNK_DEPTH {
                Box::new(Some(TerrainNode::new(depth + 1)))
            } else {
                Box::new(None)
            },
            top_left: TerrainChunk::new(),
            top_mid: TerrainChunk::new(),
            top_right: TerrainChunk::new(),
            mid_left: TerrainChunk::new(),
            mid_right: TerrainChunk::new(),
            bottom_left: TerrainChunk::new(),
            bottom_mid: TerrainChunk::new(),
            bottom_right: TerrainChunk::new(),
        }
    }

    pub fn generate(
        &mut self,
        position: Vec2,
        mut size: f32,
        height_fn: &Box<dyn Fn(Vec2) -> f32 + Send + Sync>,
    ) {
        size *= 3.0;

        self.bottom_left
            .generate(position + Vec2::new(-size, -size), size, height_fn);
        self.bottom_mid
            .generate(position + Vec2::new(0.0, -size), size, height_fn);
        self.bottom_right
            .generate(position + Vec2::new(size, -size), size, height_fn);
        self.mid_left
            .generate(position + Vec2::new(-size, 0.0), size, height_fn);
        self.mid_right
            .generate(position + Vec2::new(size, 0.0), size, height_fn);
        self.top_left
            .generate(position + Vec2::new(-size, size), size, height_fn);
        self.top_mid
            .generate(position + Vec2::new(0.0, size), size, height_fn);
        self.top_right
            .generate(position + Vec2::new(size, size), size, height_fn);

        if let Some(child) = &mut *self.child {
            child.generate(position - Vec2::splat(size), size, height_fn);
        }
    }
}

#[derive(Clone, Copy)]
pub struct TerrainChunk {
    vertices: [[f32; CHUNK_RESOLUTION]; CHUNK_RESOLUTION],
    indices: Option<[[u32; CHUNK_RESOLUTION]; CHUNK_RESOLUTION]>,
}

impl TerrainChunk {
    pub fn new() -> Self {
        Self {
            vertices: [[0.0; CHUNK_RESOLUTION]; CHUNK_RESOLUTION],
            indices: None,
        }
    }

    pub fn generate(
        &mut self,
        position: Vec2,
        size: f32,
        height_fn: &Box<dyn Fn(Vec2) -> f32 + Send + Sync>,
    ) {
        for (x, column) in self.vertices.iter_mut().enumerate() {
            for (y, vertex) in column.iter_mut().enumerate() {
                *vertex = height_fn(
                    position
                        + Vec2::new(
                            x as f32 / CHUNK_RESOLUTION as f32 * size,
                            y as f32 / CHUNK_RESOLUTION as f32 * size,
                        ),
                );
            }
        }
    }
}
