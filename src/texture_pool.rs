use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

/// Unique identifier for a texture in the pool
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureId(usize);

/// Key for identifying texture requirements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureKey {
    pub width: u32,
    pub height: u32,
    pub format: PixelFormatEnum,
}

impl TextureKey {
    pub fn new(width: u32, height: u32, format: PixelFormatEnum) -> Self {
        Self {
            width,
            height,
            format,
        }
    }
}

/// Metadata for a pooled texture
#[derive(Debug)]
struct PooledTextureMetadata {
    id: TextureId,
    key: TextureKey,
    in_use: bool,
    last_used: Instant,
    created_at: Instant,
}

/// Wrapper for a pooled texture with its metadata
pub struct PooledTexture {
    metadata: PooledTextureMetadata,
    texture: Texture<'static>,
}

impl PooledTexture {
    /// Get the texture ID
    pub fn id(&self) -> TextureId {
        self.metadata.id
    }

    /// Get the texture key (dimensions and format)
    pub fn key(&self) -> TextureKey {
        self.metadata.key
    }

    /// Get a reference to the underlying SDL texture
    pub fn texture(&self) -> &Texture<'static> {
        &self.texture
    }

    /// Get a mutable reference to the underlying SDL texture
    pub fn texture_mut(&mut self) -> &mut Texture<'static> {
        self.metadata.last_used = Instant::now();
        &mut self.texture
    }

    /// Check if this texture is in use
    pub fn is_in_use(&self) -> bool {
        self.metadata.in_use
    }

    /// Get the time when this texture was last used
    pub fn last_used(&self) -> Instant {
        self.metadata.last_used
    }

    /// Get the time when this texture was created
    pub fn created_at(&self) -> Instant {
        self.metadata.created_at
    }
}

/// Configuration for the texture pool
#[derive(Debug, Clone)]
pub struct TexturePoolConfig {
    /// Maximum number of textures to keep in the pool
    pub max_pool_size: usize,
    /// Maximum number of textures per size category
    pub max_per_size: usize,
    /// Time before unused textures are eligible for cleanup (in seconds)
    pub cleanup_threshold_secs: u64,
}

impl Default for TexturePoolConfig {
    fn default() -> Self {
        Self {
            max_pool_size: 100,
            max_per_size: 10,
            cleanup_threshold_secs: 60,
        }
    }
}

/// Statistics for the texture pool
#[derive(Debug, Clone)]
pub struct TexturePoolStats {
    /// Total number of textures in the pool
    pub total_textures: usize,
    /// Number of textures currently in use
    pub textures_in_use: usize,
    /// Number of textures available for reuse
    pub textures_available: usize,
    /// Number of different texture sizes in the pool
    pub unique_sizes: usize,
    /// Total memory usage estimate (in bytes)
    pub estimated_memory_usage: usize,
    /// Number of cache hits (texture reused)
    pub cache_hits: usize,
    /// Number of cache misses (new texture created)
    pub cache_misses: usize,
}

/// LRU entry for texture eviction
#[derive(Debug)]
struct LruEntry {
    texture_id: TextureId,
    last_used: Instant,
}

/// Texture pool for efficient texture management
pub struct TexturePool {
    /// All textures indexed by their ID
    textures: HashMap<TextureId, PooledTexture>,
    /// Textures grouped by size for efficient lookup
    size_groups: HashMap<TextureKey, VecDeque<TextureId>>,
    /// LRU queue for eviction policy
    lru_queue: VecDeque<LruEntry>,
    /// Configuration
    config: TexturePoolConfig,
    /// Statistics
    stats: TexturePoolStats,
    /// Next texture ID to assign
    next_id: usize,
    /// Texture creator for creating new textures
    texture_creator: TextureCreator<WindowContext>,
}

impl TexturePool {
    /// Create a new texture pool
    pub fn new(texture_creator: TextureCreator<WindowContext>, config: TexturePoolConfig) -> Self {
        Self {
            textures: HashMap::new(),
            size_groups: HashMap::new(),
            lru_queue: VecDeque::new(),
            config,
            stats: TexturePoolStats {
                total_textures: 0,
                textures_in_use: 0,
                textures_available: 0,
                unique_sizes: 0,
                estimated_memory_usage: 0,
                cache_hits: 0,
                cache_misses: 0,
            },
            next_id: 0,
            texture_creator,
        }
    }

    /// Create a new texture pool with default configuration
    pub fn new_default(texture_creator: TextureCreator<WindowContext>) -> Self {
        Self::new(texture_creator, TexturePoolConfig::default())
    }

    /// Get or create a texture with the specified dimensions and format
    pub fn get_texture(&mut self, key: TextureKey) -> Result<TextureId, String> {
        // Try to find a reusable texture
        if let Some(texture_id) = self.find_reusable_texture(key) {
            self.mark_texture_in_use(texture_id);
            self.stats.cache_hits += 1;
            return Ok(texture_id);
        }

        // Create a new texture
        self.create_new_texture(key)
    }

    /// Get a mutable reference to a texture by ID
    pub fn get_texture_mut(&mut self, texture_id: TextureId) -> Option<&mut PooledTexture> {
        if let Some(texture) = self.textures.get_mut(&texture_id) {
            texture.metadata.last_used = Instant::now();
            Some(texture)
        } else {
            None
        }
    }

    /// Get a reference to a texture by ID
    pub fn get_texture_ref(&self, texture_id: TextureId) -> Option<&PooledTexture> {
        self.textures.get(&texture_id)
    }

    /// Release a texture back to the pool
    pub fn release_texture(&mut self, texture_id: TextureId) {
        if let Some(texture) = self.textures.get_mut(&texture_id) {
            texture.metadata.in_use = false;
            texture.metadata.last_used = Instant::now();

            // Update LRU queue
            self.update_lru_queue(texture_id);

            // Update statistics
            self.stats.textures_in_use = self.stats.textures_in_use.saturating_sub(1);
            self.stats.textures_available += 1;
        }
    }

    /// Find a reusable texture with the given key
    fn find_reusable_texture(&mut self, key: TextureKey) -> Option<TextureId> {
        let size_group = self.size_groups.get_mut(&key)?;

        // Find the first available texture in this size group
        while let Some(texture_id) = size_group.pop_front() {
            if let Some(texture) = self.textures.get(&texture_id) {
                if !texture.metadata.in_use {
                    // Move to back of queue for better LRU behavior
                    size_group.push_back(texture_id);
                    return Some(texture_id);
                }
            }
        }

        None
    }

    /// Create a new texture with the specified key
    fn create_new_texture(&mut self, key: TextureKey) -> Result<TextureId, String> {
        // Check if we need to cleanup before creating
        if self.should_cleanup() {
            self.cleanup_unused_textures();
        }

        // Create the texture
        let texture = self
            .texture_creator
            .create_texture_streaming(key.format, key.width, key.height)
            .map_err(|e| e.to_string())?;

        // Convert to 'static lifetime using unsafe transmute (matching existing pattern)
        let static_texture: Texture<'static> = unsafe { std::mem::transmute(texture) };

        // Generate new ID
        let texture_id = TextureId(self.next_id);
        self.next_id += 1;

        // Create metadata
        let now = Instant::now();
        let metadata = PooledTextureMetadata {
            id: texture_id,
            key,
            in_use: true,
            last_used: now,
            created_at: now,
        };

        // Create pooled texture
        let pooled_texture = PooledTexture {
            metadata,
            texture: static_texture,
        };

        // Add to pool
        self.textures.insert(texture_id, pooled_texture);

        // Add to size group
        self.size_groups
            .entry(key)
            .or_default()
            .push_back(texture_id);

        // Update LRU queue
        self.lru_queue.push_back(LruEntry {
            texture_id,
            last_used: now,
        });

        // Update statistics
        self.stats.total_textures += 1;
        self.stats.textures_in_use += 1;
        self.stats.cache_misses += 1;
        self.stats.unique_sizes = self.size_groups.len();
        self.stats.estimated_memory_usage += Self::estimate_texture_memory(key);

        Ok(texture_id)
    }

    /// Mark a texture as in use
    fn mark_texture_in_use(&mut self, texture_id: TextureId) {
        if let Some(texture) = self.textures.get_mut(&texture_id) {
            if !texture.metadata.in_use {
                texture.metadata.in_use = true;
                texture.metadata.last_used = Instant::now();

                // Update statistics
                self.stats.textures_in_use += 1;
                self.stats.textures_available = self.stats.textures_available.saturating_sub(1);

                // Update LRU queue
                self.update_lru_queue(texture_id);
            }
        }
    }

    /// Update the LRU queue for a texture
    fn update_lru_queue(&mut self, texture_id: TextureId) {
        let now = Instant::now();

        // Remove existing entry
        self.lru_queue
            .retain(|entry| entry.texture_id != texture_id);

        // Add to back (most recently used)
        self.lru_queue.push_back(LruEntry {
            texture_id,
            last_used: now,
        });
    }

    /// Check if we should perform cleanup
    fn should_cleanup(&self) -> bool {
        self.stats.total_textures >= self.config.max_pool_size
    }

    /// Cleanup unused textures based on LRU policy
    fn cleanup_unused_textures(&mut self) {
        let now = Instant::now();
        let threshold = std::time::Duration::from_secs(self.config.cleanup_threshold_secs);

        // Find textures to remove
        let mut to_remove = Vec::new();

        for entry in &self.lru_queue {
            if let Some(texture) = self.textures.get(&entry.texture_id) {
                if !texture.metadata.in_use && now.duration_since(entry.last_used) > threshold {
                    to_remove.push(entry.texture_id);
                }
            }
        }

        // Remove old textures
        for texture_id in to_remove {
            self.remove_texture(texture_id);
        }

        // If still too many textures, remove oldest unused ones
        while self.stats.total_textures > self.config.max_pool_size {
            if let Some(oldest_unused) = self.find_oldest_unused_texture() {
                self.remove_texture(oldest_unused);
            } else {
                break; // All remaining textures are in use
            }
        }
    }

    /// Find the oldest unused texture
    fn find_oldest_unused_texture(&self) -> Option<TextureId> {
        for entry in &self.lru_queue {
            if let Some(texture) = self.textures.get(&entry.texture_id) {
                if !texture.metadata.in_use {
                    return Some(entry.texture_id);
                }
            }
        }
        None
    }

    /// Remove a texture from the pool
    fn remove_texture(&mut self, texture_id: TextureId) {
        if let Some(texture) = self.textures.remove(&texture_id) {
            let key = texture.metadata.key;

            // Remove from size group
            if let Some(size_group) = self.size_groups.get_mut(&key) {
                size_group.retain(|&id| id != texture_id);
                if size_group.is_empty() {
                    self.size_groups.remove(&key);
                }
            }

            // Remove from LRU queue
            self.lru_queue
                .retain(|entry| entry.texture_id != texture_id);

            // Update statistics
            self.stats.total_textures = self.stats.total_textures.saturating_sub(1);
            if texture.metadata.in_use {
                self.stats.textures_in_use = self.stats.textures_in_use.saturating_sub(1);
            } else {
                self.stats.textures_available = self.stats.textures_available.saturating_sub(1);
            }
            self.stats.unique_sizes = self.size_groups.len();
            self.stats.estimated_memory_usage = self
                .stats
                .estimated_memory_usage
                .saturating_sub(Self::estimate_texture_memory(key));
        }
    }

    /// Estimate memory usage for a texture
    fn estimate_texture_memory(key: TextureKey) -> usize {
        let bytes_per_pixel = match key.format {
            PixelFormatEnum::RGB24 => 3,
            PixelFormatEnum::RGBA32 => 4,
            PixelFormatEnum::RGB565 => 2,
            PixelFormatEnum::RGB555 => 2,
            _ => 4, // Default to 4 bytes per pixel
        };

        (key.width as usize) * (key.height as usize) * bytes_per_pixel
    }

    /// Get current statistics
    pub fn get_stats(&self) -> TexturePoolStats {
        self.stats.clone()
    }

    /// Get the pool configuration
    pub fn get_config(&self) -> &TexturePoolConfig {
        &self.config
    }

    /// Update the pool configuration
    pub fn set_config(&mut self, config: TexturePoolConfig) {
        self.config = config;

        // Trigger cleanup if needed
        if self.should_cleanup() {
            self.cleanup_unused_textures();
        }
    }

    /// Force cleanup of all unused textures
    pub fn force_cleanup(&mut self) {
        let unused_textures: Vec<TextureId> = self
            .textures
            .values()
            .filter(|texture| !texture.metadata.in_use)
            .map(|texture| texture.metadata.id)
            .collect();

        for texture_id in unused_textures {
            self.remove_texture(texture_id);
        }
    }

    /// Get the number of textures in the pool
    pub fn len(&self) -> usize {
        self.textures.len()
    }

    /// Check if the pool is empty
    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }

    /// Get all texture IDs currently in the pool
    pub fn get_all_texture_ids(&self) -> Vec<TextureId> {
        self.textures.keys().copied().collect()
    }

    /// Check if a texture ID exists in the pool
    pub fn contains_texture(&self, texture_id: TextureId) -> bool {
        self.textures.contains_key(&texture_id)
    }
}

impl Drop for TexturePool {
    fn drop(&mut self) {
        // Clean up all textures
        self.textures.clear();
        self.size_groups.clear();
        self.lru_queue.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests would require SDL2 context to run, so they're mainly for documentation
    // In a real test environment, you would set up SDL2 context first

    #[test]
    #[ignore = "Requires SDL2 context"]
    fn test_texture_pool_creation() {
        // This test would create a texture pool and verify basic functionality
        // Ignored because it requires SDL2 setup
    }

    #[test]
    fn test_texture_key_equality() {
        let key1 = TextureKey::new(100, 100, PixelFormatEnum::RGB24);
        let key2 = TextureKey::new(100, 100, PixelFormatEnum::RGB24);
        let key3 = TextureKey::new(200, 100, PixelFormatEnum::RGB24);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_texture_id_equality() {
        let id1 = TextureId(1);
        let id2 = TextureId(1);
        let id3 = TextureId(2);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_config_defaults() {
        let config = TexturePoolConfig::default();
        assert_eq!(config.max_pool_size, 100);
        assert_eq!(config.max_per_size, 10);
        assert_eq!(config.cleanup_threshold_secs, 60);
    }

    #[test]
    fn test_memory_estimation() {
        let key_rgb24 = TextureKey::new(100, 100, PixelFormatEnum::RGB24);
        let key_rgba32 = TextureKey::new(100, 100, PixelFormatEnum::RGBA32);

        assert_eq!(
            TexturePool::estimate_texture_memory(key_rgb24),
            100 * 100 * 3
        );
        assert_eq!(
            TexturePool::estimate_texture_memory(key_rgba32),
            100 * 100 * 4
        );
    }
}
