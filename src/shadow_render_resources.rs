use crate::sun::*;
use bevy::{
    ecs::system::BoxedSystem,
    prelude::*,
    render::{
        render_graph::{CommandQueue, Node, ResourceSlots, SystemNode},
        renderer::{
            BufferId, BufferInfo, BufferMapMode, BufferUsage, RenderContext, RenderResourceBinding,
            RenderResourceBindings, RenderResourceContext, RenderResourceHints, RenderResourceType,
            RenderResources,
        },
        texture::{SAMPLER_ASSET_INDEX, TEXTURE_ASSET_INDEX},
    },
    utils::HashMap,
};
use std::ops::DerefMut;

#[derive(Debug)]
struct QueuedBufferWrite {
    buffer: BufferId,
    target_offset: usize,
    source_offset: usize,
    size: usize,
}

/// Used to track items in a gpu buffer in an "array" style
#[derive(Debug)]
struct BufferArray<I> {
    item_size: usize,
    buffer_capacity: usize,
    min_capacity: usize,
    len: usize,
    buffer: Option<BufferId>,
    free_indices: Vec<usize>,
    indices: HashMap<I, usize>,
}

impl<I: std::hash::Hash + Eq> BufferArray<I> {
    pub fn new(item_size: usize, min_capacity: usize) -> Self {
        BufferArray {
            item_size,
            len: 0,
            buffer_capacity: 0,
            min_capacity,
            buffer: None,
            free_indices: Vec::new(),
            indices: HashMap::default(),
        }
    }

    fn get_or_assign_index(&mut self, id: I) -> usize {
        if let Some(index) = self.indices.get(&id) {
            *index
        } else if let Some(index) = self.free_indices.pop() {
            self.indices.insert(id, index);
            self.len += 1;
            index
        } else {
            let index = self.len;
            self.indices.insert(id, index);
            self.len += 1;
            index
        }
    }

    pub fn get_binding(&self, id: I) -> Option<RenderResourceBinding> {
        self.indices
            .get(&id)
            .map(|index| RenderResourceBinding::Buffer {
                buffer: self.buffer.unwrap(),
                dynamic_index: Some((index * self.item_size) as u32),
                range: 0..self.item_size as u64,
            })
    }

    pub fn remove_binding(&mut self, id: I) {
        if let Some(index) = self.indices.remove(&id) {
            self.free_indices.push(index);
            self.len -= 1;
        }
    }

    pub fn resize(&mut self, render_resource_context: &dyn RenderResourceContext) -> bool {
        if self.len <= self.buffer_capacity {
            return false;
        }

        self.allocate_buffer(render_resource_context);
        // TODO: allow shrinking
        true
    }

    pub fn allocate_buffer(&mut self, render_resource_context: &dyn RenderResourceContext) {
        if let Some(old_buffer) = self.buffer.take() {
            render_resource_context.remove_buffer(old_buffer);
        }

        let new_len = if self.buffer_capacity == 0 {
            self.min_capacity.max(self.len)
        } else {
            self.min_capacity.max(self.len * 2)
        };

        let size = new_len * self.item_size;
        let buffer = render_resource_context.create_buffer(BufferInfo {
            size,
            buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
            ..Default::default()
        });

        self.buffer = Some(buffer);
        self.buffer_capacity = new_len;
    }
}

struct UniformBufferArrays<I, T>
where
    T: RenderResources,
{
    buffer_arrays: Vec<Option<BufferArray<I>>>,
    staging_buffer: Option<BufferId>,
    staging_buffer_size: usize,
    required_staging_buffer_size: usize,
    current_staging_buffer_offset: usize,
    queued_buffer_writes: Vec<QueuedBufferWrite>,
    _marker: std::marker::PhantomData<T>,
}

impl<I, T> Default for UniformBufferArrays<I, T>
where
    T: RenderResources,
{
    fn default() -> Self {
        Self {
            buffer_arrays: Default::default(),
            staging_buffer: Default::default(),
            staging_buffer_size: 0,
            current_staging_buffer_offset: 0,
            queued_buffer_writes: Vec::new(),
            required_staging_buffer_size: 0,
            _marker: Default::default(),
        }
    }
}

impl<I, T> UniformBufferArrays<I, T>
where
    I: std::hash::Hash + Eq + Copy,
    T: RenderResources,
{
    /// Initialize this UniformBufferArrays using information from a RenderResources value.
    fn initialize(
        &mut self,
        render_resources: &T,
        render_resource_context: &dyn RenderResourceContext,
    ) {
        if self.buffer_arrays.len() != render_resources.render_resources_len() {
            let mut buffer_arrays = Vec::with_capacity(render_resources.render_resources_len());
            for render_resource in render_resources.iter() {
                if let Some(RenderResourceType::Buffer) = render_resource.resource_type() {
                    let size = render_resource.buffer_byte_len().unwrap();
                    let aligned_size = render_resource_context.get_aligned_uniform_size(size, true);
                    buffer_arrays.push(Some(BufferArray::new(aligned_size, 10)));
                } else {
                    buffer_arrays.push(None);
                }
            }

            self.buffer_arrays = buffer_arrays;
        }
    }

    /// Resets staging buffer tracking information
    fn begin_update(&mut self) {
        self.required_staging_buffer_size = 0;
        self.current_staging_buffer_offset = 0;
    }

    /// Find a spot for the given RenderResources in each uniform's BufferArray and prepare space in
    /// the staging buffer
    fn prepare_uniform_buffers(&mut self, id: I, render_resources: &T) {
        for (i, render_resource) in render_resources.iter().enumerate() {
            if let Some(RenderResourceType::Buffer) = render_resource.resource_type() {
                let size = render_resource.buffer_byte_len().unwrap();
                if let Some(buffer_array) = &mut self.buffer_arrays[i] {
                    buffer_array.get_or_assign_index(id);
                    self.required_staging_buffer_size += size;
                }
            }
        }
    }

    /// Resize BufferArray buffers if they aren't large enough
    fn resize_buffer_arrays(
        &mut self,
        render_resource_context: &dyn RenderResourceContext,
    ) -> bool {
        let mut resized = false;
        for buffer_array in self.buffer_arrays.iter_mut().flatten() {
            resized |= buffer_array.resize(render_resource_context);
        }

        resized
    }

    fn set_required_staging_buffer_size_to_max(&mut self) {
        let mut new_size = 0;
        for buffer_array in self.buffer_arrays.iter().flatten() {
            new_size += buffer_array.item_size * buffer_array.len;
        }

        if new_size > self.required_staging_buffer_size {
            self.required_staging_buffer_size = new_size;
        }
    }

    /// Update the staging buffer to provide enough space to copy data to target buffers.
    fn resize_staging_buffer(&mut self, render_resource_context: &dyn RenderResourceContext) {
        // TODO: allow staging buffer to scale down
        if self.required_staging_buffer_size > self.staging_buffer_size {
            if let Some(staging_buffer) = self.staging_buffer {
                render_resource_context.remove_buffer(staging_buffer);
            }

            if self.required_staging_buffer_size > 0 {
                let staging_buffer = render_resource_context.create_buffer(BufferInfo {
                    buffer_usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
                    size: self.required_staging_buffer_size,
                    ..Default::default()
                });
                self.staging_buffer = Some(staging_buffer);
            } else {
                self.staging_buffer = None;
            }

            self.staging_buffer_size = self.required_staging_buffer_size;
        }
    }

    fn remove_bindings(&mut self, id: I) {
        for buffer_array in self.buffer_arrays.iter_mut().flatten() {
            buffer_array.remove_binding(id);
        }
    }

    fn write_uniform_buffers(
        &mut self,
        id: I,
        uniforms: &T,
        dynamic_uniforms: bool,
        render_resource_context: &dyn RenderResourceContext,
        render_resource_bindings: &mut RenderResourceBindings,
        staging_buffer: &mut [u8],
    ) {
        for (i, render_resource) in uniforms.iter().enumerate() {
            if let Some(RenderResourceType::Buffer) = render_resource.resource_type() {
                let size = render_resource.buffer_byte_len().unwrap();
                let render_resource_name = uniforms.get_render_resource_name(i).unwrap();
                let aligned_size = render_resource_context.get_aligned_uniform_size(size, false);
                let buffer_array = self.buffer_arrays[i].as_mut().unwrap();
                let range = 0..aligned_size as u64;
                let (target_buffer, target_offset) = if dynamic_uniforms {
                    let binding = buffer_array.get_binding(id).unwrap();
                    let dynamic_index = if let RenderResourceBinding::Buffer {
                        dynamic_index: Some(dynamic_index),
                        ..
                    } = binding
                    {
                        dynamic_index
                    } else {
                        panic!("Dynamic index should always be set.");
                    };
                    render_resource_bindings.set(render_resource_name, binding);
                    (buffer_array.buffer.unwrap(), dynamic_index)
                } else {
                    let mut matching_buffer = None;
                    if let Some(binding) = render_resource_bindings.get(render_resource_name) {
                        let buffer_id = binding.get_buffer().unwrap();
                        if let Some(BufferInfo {
                            size: current_size, ..
                        }) = render_resource_context.get_buffer_info(buffer_id)
                        {
                            if aligned_size == current_size {
                                matching_buffer = Some(buffer_id);
                            } else {
                                render_resource_context.remove_buffer(buffer_id);
                            }
                        }
                    }

                    let resource = if let Some(matching_buffer) = matching_buffer {
                        matching_buffer
                    } else {
                        let mut usage = BufferUsage::UNIFORM;
                        if let Some(render_resource_hints) = uniforms.get_render_resource_hints(i) {
                            if render_resource_hints.contains(RenderResourceHints::BUFFER) {
                                usage = BufferUsage::STORAGE
                            }
                        }

                        let buffer = render_resource_context.create_buffer(BufferInfo {
                            size: aligned_size,
                            buffer_usage: BufferUsage::COPY_DST | usage,
                            ..Default::default()
                        });

                        render_resource_bindings.set(
                            render_resource_name,
                            RenderResourceBinding::Buffer {
                                buffer,
                                range,
                                dynamic_index: None,
                            },
                        );
                        buffer
                    };

                    (resource, 0)
                };

                render_resource.write_buffer_bytes(
                    &mut staging_buffer[self.current_staging_buffer_offset
                        ..(self.current_staging_buffer_offset + size)],
                );

                self.queued_buffer_writes.push(QueuedBufferWrite {
                    buffer: target_buffer,
                    target_offset: target_offset as usize,
                    source_offset: self.current_staging_buffer_offset,
                    size,
                });
                self.current_staging_buffer_offset += size;
            }
        }
    }

    fn copy_staging_buffer_to_final_buffers(
        &mut self,
        command_queue: &mut CommandQueue,
        staging_buffer: BufferId,
    ) {
        for queued_buffer_write in self.queued_buffer_writes.drain(..) {
            command_queue.copy_buffer_to_buffer(
                staging_buffer,
                queued_buffer_write.source_offset as u64,
                queued_buffer_write.buffer,
                queued_buffer_write.target_offset as u64,
                queued_buffer_write.size as u64,
            )
        }
    }
}

#[derive(Default)]
pub struct ShadowRenderResourcesNode<T>
where
    T: RenderResources,
{
    command_queue: CommandQueue,
    dynamic_uniforms: bool,
    _marker: std::marker::PhantomData<T>,
}

impl<T> ShadowRenderResourcesNode<T>
where
    T: RenderResources,
{
    pub fn new(dynamic_uniforms: bool) -> Self {
        ShadowRenderResourcesNode {
            command_queue: CommandQueue::default(),
            dynamic_uniforms,
            _marker: std::marker::PhantomData::default(),
        }
    }
}

impl<T> Node for ShadowRenderResourcesNode<T>
where
    T: RenderResources,
{
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

impl<T> SystemNode for ShadowRenderResourcesNode<T>
where
    T: RenderResources,
{
    fn get_system(&self) -> BoxedSystem {
        let system = render_resources_node_system::<T>.system().config(|config| {
            config.0 = Some(ShadowRenderResourcesNodeState {
                command_queue: self.command_queue.clone(),
                uniform_buffer_arrays: UniformBufferArrays::<Entity, T>::default(),
                dynamic_uniforms: self.dynamic_uniforms,
            })
        });

        Box::new(system)
    }
}

struct ShadowRenderResourcesNodeState<I, T: RenderResources> {
    command_queue: CommandQueue,
    uniform_buffer_arrays: UniformBufferArrays<I, T>,
    dynamic_uniforms: bool,
}

impl<I, T: RenderResources> Default for ShadowRenderResourcesNodeState<I, T> {
    fn default() -> Self {
        Self {
            command_queue: Default::default(),
            uniform_buffer_arrays: Default::default(),
            dynamic_uniforms: Default::default(),
        }
    }
}

fn render_resources_node_system<T: RenderResources>(
    mut state: Local<ShadowRenderResourcesNodeState<Entity, T>>,
    mut entities_waiting_for_textures: Local<Vec<Entity>>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    removed: RemovedComponents<T>,
    mut queries: QuerySet<(
        Query<(Entity, &T, &Visible, &mut ShadowCaster), Or<(Changed<T>, Changed<Visible>)>>,
        Query<(Entity, &T, &Visible, &mut ShadowCaster)>,
    )>,
) {
    let state = state.deref_mut();
    let uniform_buffer_arrays = &mut state.uniform_buffer_arrays;
    let render_resource_context = &**render_resource_context;
    uniform_buffer_arrays.begin_update();
    // initialize uniform buffer arrays using the first RenderResources
    if let Some((_, first, _, _)) = queries.q0_mut().iter_mut().next() {
        uniform_buffer_arrays.initialize(first, render_resource_context);
    }

    for entity in removed.iter() {
        uniform_buffer_arrays.remove_bindings(entity);
    }

    // handle entities that were waiting for texture loads on the last update
    for entity in std::mem::take(&mut *entities_waiting_for_textures) {
        if let Ok((entity, uniforms, _visible, mut shadow_caster)) =
            queries.q1_mut().get_mut(entity)
        {
            if !setup_uniform_texture_resources::<T>(
                &uniforms,
                render_resource_context,
                &mut shadow_caster.render_pipelines.bindings,
            ) {
                entities_waiting_for_textures.push(entity);
            }
        }
    }

    for (entity, uniforms, visible, mut shadow_caster) in queries.q0_mut().iter_mut() {
        if !visible.is_visible {
            continue;
        }
        uniform_buffer_arrays.prepare_uniform_buffers(entity, uniforms);
        if !setup_uniform_texture_resources::<T>(
            &uniforms,
            render_resource_context,
            &mut shadow_caster.render_pipelines.bindings,
        ) {
            entities_waiting_for_textures.push(entity);
        }
    }

    let resized = uniform_buffer_arrays.resize_buffer_arrays(render_resource_context);
    if resized {
        uniform_buffer_arrays.set_required_staging_buffer_size_to_max()
    }
    uniform_buffer_arrays.resize_staging_buffer(render_resource_context);

    if let Some(staging_buffer) = state.uniform_buffer_arrays.staging_buffer {
        render_resource_context.map_buffer(staging_buffer, BufferMapMode::Write);
        render_resource_context.write_mapped_buffer(
            staging_buffer,
            0..state.uniform_buffer_arrays.staging_buffer_size as u64,
            &mut |mut staging_buffer, _render_resource_context| {
                // if the buffer array was resized, write all entities to the new buffer, otherwise
                // only write changes
                if resized {
                    for (entity, uniforms, visible, mut shadow_caster) in
                        queries.q1_mut().iter_mut()
                    {
                        if !visible.is_visible {
                            continue;
                        }

                        state.uniform_buffer_arrays.write_uniform_buffers(
                            entity,
                            &uniforms,
                            state.dynamic_uniforms,
                            render_resource_context,
                            &mut shadow_caster.render_pipelines.bindings,
                            &mut staging_buffer,
                        );
                    }
                } else {
                    for (entity, uniforms, visible, mut shadow_caster) in
                        queries.q0_mut().iter_mut()
                    {
                        if !visible.is_visible {
                            continue;
                        }

                        state.uniform_buffer_arrays.write_uniform_buffers(
                            entity,
                            &uniforms,
                            state.dynamic_uniforms,
                            render_resource_context,
                            &mut shadow_caster.render_pipelines.bindings,
                            &mut staging_buffer,
                        );
                    }
                }
            },
        );
        render_resource_context.unmap_buffer(staging_buffer);

        state
            .uniform_buffer_arrays
            .copy_staging_buffer_to_final_buffers(&mut state.command_queue, staging_buffer);
    }
}

fn setup_uniform_texture_resources<T>(
    uniforms: &T,
    render_resource_context: &dyn RenderResourceContext,
    render_resource_bindings: &mut RenderResourceBindings,
) -> bool
where
    T: RenderResources,
{
    let mut success = true;
    for (i, render_resource) in uniforms.iter().enumerate() {
        if let Some(RenderResourceType::Texture) = render_resource.resource_type() {
            let render_resource_name = uniforms.get_render_resource_name(i).unwrap();
            let sampler_name = format!("{}_sampler", render_resource_name);
            if let Some(texture_handle) = render_resource.texture() {
                if let Some(texture_resource) =
                    render_resource_context.get_asset_resource(texture_handle, TEXTURE_ASSET_INDEX)
                {
                    let sampler_resource = render_resource_context
                        .get_asset_resource(texture_handle, SAMPLER_ASSET_INDEX)
                        .unwrap();

                    render_resource_bindings.set(
                        render_resource_name,
                        RenderResourceBinding::Texture(texture_resource.get_texture().unwrap()),
                    );
                    render_resource_bindings.set(
                        &sampler_name,
                        RenderResourceBinding::Sampler(sampler_resource.get_sampler().unwrap()),
                    );
                    continue;
                } else {
                    success = false;
                }
            }
        }
    }

    success
}
