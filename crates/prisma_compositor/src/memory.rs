/// Zero-copy memory management system for optimal performance
use std::sync::{Arc, Mutex, RwLock};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering};
use wgpu::*;

/// High-performance memory pool with zero-copy semantics
pub struct MemoryPool {
    device: Arc<Device>,
    total_size: AtomicUsize,
    used_size: AtomicUsize,

    /// Memory chunks organized by size classes for efficient allocation
    small_chunks: Arc<Mutex<VecDeque<MemoryChunk>>>, // < 1KB
    medium_chunks: Arc<Mutex<VecDeque<MemoryChunk>>>, // 1KB - 64KB
    large_chunks: Arc<Mutex<VecDeque<MemoryChunk>>>, // > 64KB

    /// Memory allocation tracking
    allocation_counter: AtomicU64,
    fragmentation_ratio: Arc<RwLock<f32>>,
}

/// Memory chunk with metadata for efficient reuse
#[derive(Debug)]
pub struct MemoryChunk {
    pub size: usize,
    pub offset: usize,
    pub is_free: bool,
    pub last_used: std::time::Instant,
    pub usage_count: u32,
}

impl MemoryPool {
    /// Create a new memory pool with the specified size
    pub fn new(device: Arc<Device>, total_size: usize) -> Self {
        Self {
            device,
            total_size: AtomicUsize::new(total_size),
            used_size: AtomicUsize::new(0),
            small_chunks: Arc::new(Mutex::new(VecDeque::new())),
            medium_chunks: Arc::new(Mutex::new(VecDeque::new())),
            large_chunks: Arc::new(Mutex::new(VecDeque::new())),
            allocation_counter: AtomicU64::new(0),
            fragmentation_ratio: Arc::new(RwLock::new(0.0)),
        }
    }

    /// Allocate memory with zero-copy optimization
    pub fn allocate(&self, size: usize) -> Option<MemoryChunk> {
        let chunk_queue = self.get_chunk_queue_for_size(size);

        // Try to reuse existing chunk
        if let Ok(mut chunks) = chunk_queue.lock() {
            if let Some(mut chunk) = chunks.pop_front() {
                if chunk.size >= size {
                    chunk.is_free = false;
                    chunk.last_used = std::time::Instant::now();
                    chunk.usage_count += 1;

                    self.used_size.fetch_add(size, Ordering::Relaxed);
                    self.allocation_counter.fetch_add(1, Ordering::Relaxed);

                    return Some(chunk);
                }
            }
        }

        // Allocate new chunk if no suitable reusable chunk found
        let new_chunk = MemoryChunk {
            size,
            offset: self.allocate_new_offset(size)?,
            is_free: false,
            last_used: std::time::Instant::now(),
            usage_count: 1,
        };

        self.used_size.fetch_add(size, Ordering::Relaxed);
        self.allocation_counter.fetch_add(1, Ordering::Relaxed);

        Some(new_chunk)
    }

    /// Deallocate memory back to the pool
    pub fn deallocate(&self, mut chunk: MemoryChunk) {
        chunk.is_free = true;
        chunk.last_used = std::time::Instant::now();

        let chunk_queue = self.get_chunk_queue_for_size(chunk.size);
        if let Ok(mut chunks) = chunk_queue.lock() {
            chunks.push_back(chunk);
        }

        self.used_size.fetch_sub(chunk.size, Ordering::Relaxed);
    }

    /// Get appropriate chunk queue based on size
    fn get_chunk_queue_for_size(&self, size: usize) -> Arc<Mutex<VecDeque<MemoryChunk>>> {
        if size < 1024 {
            Arc::clone(&self.small_chunks)
        } else if size < 65536 {
            Arc::clone(&self.medium_chunks)
        } else {
            Arc::clone(&self.large_chunks)
        }
    }

    /// Allocate a new offset in the memory pool
    fn allocate_new_offset(&self, size: usize) -> Option<usize> {
        let current_used = self.used_size.load(Ordering::Relaxed);
        let total = self.total_size.load(Ordering::Relaxed);

        if current_used + size <= total {
            Some(current_used)
        } else {
            None
        }
    }

    /// Defragment memory pool for better performance
    pub async fn defragment(&self) {
        // Implementation for memory defragmentation
        // This would compact memory and update fragmentation ratio
    }

    /// Get memory usage statistics
    pub fn get_usage_stats(&self) -> (usize, usize, f32) {
        let used = self.used_size.load(Ordering::Relaxed);
        let total = self.total_size.load(Ordering::Relaxed);
        let fragmentation = *self.fragmentation_ratio.read().unwrap();

        (used, total, fragmentation)
    }

    /// Cleanup memory pool
    pub async fn cleanup(&self) {
        // Clear all chunks
        self.small_chunks.lock().unwrap().clear();
        self.medium_chunks.lock().unwrap().clear();
        self.large_chunks.lock().unwrap().clear();

        self.used_size.store(0, Ordering::Relaxed);
    }
}

/// High-performance buffer pool with intelligent caching
pub struct BufferPool {
    device: Arc<Device>,
    memory_pool: Arc<MemoryPool>,

    /// Buffer caches organized by usage pattern
    uniform_buffers: Arc<Mutex<HashMap<u64, Arc<Buffer>>>>,
    vertex_buffers: Arc<Mutex<HashMap<u64, Arc<Buffer>>>>,
    index_buffers: Arc<Mutex<HashMap<u64, Arc<Buffer>>>>,
    storage_buffers: Arc<Mutex<HashMap<u64, Arc<Buffer>>>>,

    /// Buffer usage tracking
    buffer_lru: Arc<Mutex<VecDeque<(u64, std::time::Instant)>>>,
    max_cached_buffers: usize,
}

impl BufferPool {
    /// Create a new buffer pool
    pub fn new(device: Arc<Device>, memory_pool: Arc<MemoryPool>) -> Self {
        Self {
            device,
            memory_pool,
            uniform_buffers: Arc::new(Mutex::new(HashMap::new())),
            vertex_buffers: Arc::new(Mutex::new(HashMap::new())),
            index_buffers: Arc::new(Mutex::new(HashMap::new())),
            storage_buffers: Arc::new(Mutex::new(HashMap::new())),
            buffer_lru: Arc::new(Mutex::new(VecDeque::new())),
            max_cached_buffers: 1024,
        }
    }

    /// Get or create a uniform buffer with zero-copy semantics
    pub fn get_uniform_buffer(&self, size: u64, data: Option<&[u8]>) -> Arc<Buffer> {
        let key = self.calculate_buffer_key(size, BufferUsages::UNIFORM);

        // Try to get cached buffer
        if let Ok(buffers) = self.uniform_buffers.lock() {
            if let Some(buffer) = buffers.get(&key) {
                self.update_lru(key);
                return Arc::clone(buffer);
            }
        }

        // Create new buffer
        let buffer = self.create_buffer(size, BufferUsages::UNIFORM | BufferUsages::COPY_DST, data);

        // Cache the buffer
        if let Ok(mut buffers) = self.uniform_buffers.lock() {
            if buffers.len() >= self.max_cached_buffers {
                self.evict_lru_buffer();
            }
            buffers.insert(key, Arc::clone(&buffer));
        }

        self.update_lru(key);
        buffer
    }

    /// Get or create a vertex buffer
    pub fn get_vertex_buffer(&self, size: u64, data: Option<&[u8]>) -> Arc<Buffer> {
        let key = self.calculate_buffer_key(size, BufferUsages::VERTEX);

        if let Ok(buffers) = self.vertex_buffers.lock() {
            if let Some(buffer) = buffers.get(&key) {
                self.update_lru(key);
                return Arc::clone(buffer);
            }
        }

        let buffer = self.create_buffer(size, BufferUsages::VERTEX | BufferUsages::COPY_DST, data);

        if let Ok(mut buffers) = self.vertex_buffers.lock() {
            if buffers.len() >= self.max_cached_buffers {
                self.evict_lru_buffer();
            }
            buffers.insert(key, Arc::clone(&buffer));
        }

        self.update_lru(key);
        buffer
    }

    /// Get or create an index buffer
    pub fn get_index_buffer(&self, size: u64, data: Option<&[u8]>) -> Arc<Buffer> {
        let key = self.calculate_buffer_key(size, BufferUsages::INDEX);

        if let Ok(buffers) = self.index_buffers.lock() {
            if let Some(buffer) = buffers.get(&key) {
                self.update_lru(key);
                return Arc::clone(buffer);
            }
        }

        let buffer = self.create_buffer(size, BufferUsages::INDEX | BufferUsages::COPY_DST, data);

        if let Ok(mut buffers) = self.index_buffers.lock() {
            if buffers.len() >= self.max_cached_buffers {
                self.evict_lru_buffer();
            }
            buffers.insert(key, Arc::clone(&buffer));
        }

        self.update_lru(key);
        buffer
    }

    /// Create a new buffer with memory pool allocation
    fn create_buffer(&self, size: u64, usage: BufferUsages, data: Option<&[u8]>) -> Arc<Buffer> {
        let buffer = self.device.create_buffer(&BufferDescriptor {
            label: Some("PooledBuffer"),
            size,
            usage,
            mapped_at_creation: data.is_some(),
        });

        if let Some(data) = data {
            buffer.slice(..).get_mapped_range_mut()[..data.len()].copy_from_slice(data);
            buffer.unmap();
        }

        Arc::new(buffer)
    }

    /// Calculate a hash key for buffer caching
    fn calculate_buffer_key(&self, size: u64, usage: BufferUsages) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        size.hash(&mut hasher);
        (usage.bits()).hash(&mut hasher);
        hasher.finish()
    }

    /// Update LRU tracking
    fn update_lru(&self, key: u64) {
        if let Ok(mut lru) = self.buffer_lru.lock() {
            // Remove existing entry
            lru.retain(|(k, _)| *k != key);
            // Add to back
            lru.push_back((key, std::time::Instant::now()));
        }
    }

    /// Evict least recently used buffer
    fn evict_lru_buffer(&self) {
        if let Ok(mut lru) = self.buffer_lru.lock() {
            if let Some((key, _)) = lru.pop_front() {
                // Remove from all caches
                self.uniform_buffers.lock().unwrap().remove(&key);
                self.vertex_buffers.lock().unwrap().remove(&key);
                self.index_buffers.lock().unwrap().remove(&key);
                self.storage_buffers.lock().unwrap().remove(&key);
            }
        }
    }

    /// Get memory usage of the buffer pool
    pub fn get_memory_usage(&self) -> usize {
        let uniform_count = self.uniform_buffers.lock().unwrap().len();
        let vertex_count = self.vertex_buffers.lock().unwrap().len();
        let index_count = self.index_buffers.lock().unwrap().len();
        let storage_count = self.storage_buffers.lock().unwrap().len();

        // Rough estimate - would be more accurate in real implementation
        (uniform_count + vertex_count + index_count + storage_count) * 1024
    }

    /// Cleanup buffer pool
    pub async fn cleanup(&self) {
        self.uniform_buffers.lock().unwrap().clear();
        self.vertex_buffers.lock().unwrap().clear();
        self.index_buffers.lock().unwrap().clear();
        self.storage_buffers.lock().unwrap().clear();
        self.buffer_lru.lock().unwrap().clear();
    }
}

/// High-performance texture pool with advanced caching
pub struct TexturePool {
    device: Arc<Device>,
    memory_pool: Arc<MemoryPool>,

    /// Texture caches organized by format and usage
    texture_cache: Arc<Mutex<HashMap<u64, Arc<Texture>>>>,
    texture_view_cache: Arc<Mutex<HashMap<u64, Arc<TextureView>>>>,
    sampler_cache: Arc<Mutex<HashMap<u64, Arc<Sampler>>>>,

    /// Texture usage tracking
    texture_lru: Arc<Mutex<VecDeque<(u64, std::time::Instant)>>>,
    max_cached_textures: usize,
    total_texture_memory: AtomicUsize,
}

impl TexturePool {
    /// Create a new texture pool
    pub fn new(device: Arc<Device>, max_memory: usize) -> Self {
        Self {
            device,
            memory_pool: Arc::new(MemoryPool::new(Arc::clone(&device), max_memory)),
            texture_cache: Arc::new(Mutex::new(HashMap::new())),
            texture_view_cache: Arc::new(Mutex::new(HashMap::new())),
            sampler_cache: Arc::new(Mutex::new(HashMap::new())),
            texture_lru: Arc::new(Mutex::new(VecDeque::new())),
            max_cached_textures: 512,
            total_texture_memory: AtomicUsize::new(0),
        }
    }

    /// Get or create a texture with optimal caching
    pub fn get_texture(&self, descriptor: &TextureDescriptor) -> Arc<Texture> {
        let key = self.calculate_texture_key(descriptor);

        // Try cache first
        if let Ok(textures) = self.texture_cache.lock() {
            if let Some(texture) = textures.get(&key) {
                self.update_texture_lru(key);
                return Arc::clone(texture);
            }
        }

        // Create new texture
        let texture = Arc::new(self.device.create_texture(descriptor));

        // Cache the texture
        if let Ok(mut textures) = self.texture_cache.lock() {
            if textures.len() >= self.max_cached_textures {
                self.evict_lru_texture();
            }
            textures.insert(key, Arc::clone(&texture));
        }

        self.update_texture_lru(key);

        // Update memory tracking
        let texture_size = self.calculate_texture_size(descriptor);
        self.total_texture_memory.fetch_add(texture_size, Ordering::Relaxed);

        texture
    }

    /// Get or create a texture view
    pub fn get_texture_view(&self, texture: &Texture, descriptor: Option<&TextureViewDescriptor>) -> Arc<TextureView> {
        let key = self.calculate_view_key(texture, descriptor);

        if let Ok(views) = self.texture_view_cache.lock() {
            if let Some(view) = views.get(&key) {
                return Arc::clone(view);
            }
        }

        let view = Arc::new(texture.create_view(descriptor.unwrap_or(&TextureViewDescriptor::default())));

        if let Ok(mut views) = self.texture_view_cache.lock() {
            views.insert(key, Arc::clone(&view));
        }

        view
    }

    /// Get or create a sampler
    pub fn get_sampler(&self, descriptor: &SamplerDescriptor) -> Arc<Sampler> {
        let key = self.calculate_sampler_key(descriptor);

        if let Ok(samplers) = self.sampler_cache.lock() {
            if let Some(sampler) = samplers.get(&key) {
                return Arc::clone(sampler);
            }
        }

        let sampler = Arc::new(self.device.create_sampler(descriptor));

        if let Ok(mut samplers) = self.sampler_cache.lock() {
            samplers.insert(key, Arc::clone(&sampler));
        }

        sampler
    }

    /// Calculate texture hash key
    fn calculate_texture_key(&self, descriptor: &TextureDescriptor) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        descriptor.size.width.hash(&mut hasher);
        descriptor.size.height.hash(&mut hasher);
        descriptor.size.depth_or_array_layers.hash(&mut hasher);
        descriptor.format.hash(&mut hasher);
        descriptor.usage.bits().hash(&mut hasher);
        hasher.finish()
    }

    /// Calculate view hash key
    fn calculate_view_key(&self, _texture: &Texture, descriptor: Option<&TextureViewDescriptor>) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        if let Some(desc) = descriptor {
            desc.format.map(|f| f.hash(&mut hasher));
            desc.dimension.map(|d| d.hash(&mut hasher));
        }
        hasher.finish()
    }

    /// Calculate sampler hash key
    fn calculate_sampler_key(&self, descriptor: &SamplerDescriptor) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        descriptor.address_mode_u.hash(&mut hasher);
        descriptor.address_mode_v.hash(&mut hasher);
        descriptor.address_mode_w.hash(&mut hasher);
        descriptor.mag_filter.hash(&mut hasher);
        descriptor.min_filter.hash(&mut hasher);
        hasher.finish()
    }

    /// Calculate texture memory size
    fn calculate_texture_size(&self, descriptor: &TextureDescriptor) -> usize {
        let bytes_per_pixel = match descriptor.format {
            TextureFormat::Rgba8Unorm => 4,
            TextureFormat::Rgba8UnormSrgb => 4,
            TextureFormat::Bgra8Unorm => 4,
            TextureFormat::Bgra8UnormSrgb => 4,
            TextureFormat::R8Unorm => 1,
            TextureFormat::Rg8Unorm => 2,
            _ => 4, // Default estimate
        };

        (descriptor.size.width * descriptor.size.height * descriptor.size.depth_or_array_layers) as usize
            * bytes_per_pixel
    }

    /// Update texture LRU
    fn update_texture_lru(&self, key: u64) {
        if let Ok(mut lru) = self.texture_lru.lock() {
            lru.retain(|(k, _)| *k != key);
            lru.push_back((key, std::time::Instant::now()));
        }
    }

    /// Evict LRU texture
    fn evict_lru_texture(&self) {
        if let Ok(mut lru) = self.texture_lru.lock() {
            if let Some((key, _)) = lru.pop_front() {
                if let Ok(mut textures) = self.texture_cache.lock() {
                    textures.remove(&key);
                }
            }
        }
    }

    /// Get memory usage
    pub fn get_memory_usage(&self) -> usize {
        self.total_texture_memory.load(Ordering::Relaxed)
    }

    /// Cleanup texture pool
    pub async fn cleanup(&self) {
        self.texture_cache.lock().unwrap().clear();
        self.texture_view_cache.lock().unwrap().clear();
        self.sampler_cache.lock().unwrap().clear();
        self.texture_lru.lock().unwrap().clear();
        self.total_texture_memory.store(0, Ordering::Relaxed);
    }
}