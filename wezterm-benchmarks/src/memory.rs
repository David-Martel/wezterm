//! Memory management and optimization utilities

use dashmap::DashMap;
use object_pool::{Pool, Reusable};
use parking_lot::{Mutex, RwLock};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

/// Memory pool for efficient allocation
pub struct MemoryPool {
    total_size: AtomicUsize,
    max_size: usize,
    allocations: DashMap<usize, Vec<u8>>,
    free_list: Arc<Mutex<Vec<usize>>>,
}

impl MemoryPool {
    pub fn new(max_size: usize) -> Self {
        Self {
            total_size: AtomicUsize::new(0),
            max_size,
            allocations: DashMap::new(),
            free_list: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn allocate(&self, size: usize) -> PooledAllocation {
        // Check if we can reuse an existing allocation
        let mut free_list = self.free_list.lock();
        if let Some(id) = free_list.pop() {
            if let Some(entry) = self.allocations.get_mut(&id) {
                if entry.len() >= size {
                    return PooledAllocation {
                        id,
                        pool: self as *const Self,
                    };
                }
            }
        }

        // Allocate new — check capacity before committing
        let current = self.total_size.load(Ordering::Relaxed);
        assert!(
            current + size <= self.max_size,
            "Memory pool exhausted: requested {size} bytes but only {} available",
            self.max_size.saturating_sub(current)
        );
        self.total_size.fetch_add(size, Ordering::Relaxed);

        let id = self.allocations.len();
        self.allocations.insert(id, vec![0u8; size]);

        PooledAllocation {
            id,
            pool: self as *const Self,
        }
    }

    fn release(&self, id: usize) {
        let mut free_list = self.free_list.lock();
        free_list.push(id);
    }
}

pub struct PooledAllocation {
    id: usize,
    pool: *const MemoryPool,
}

impl PooledAllocation {
    pub fn release(self) {
        unsafe {
            (*self.pool).release(self.id);
        }
    }
}

/// Buffer pool for reusable byte buffers
pub struct BufferPool {
    pool: Arc<Pool<Vec<u8>>>,
    buffer_size: usize,
}

impl BufferPool {
    pub fn new(capacity: usize, buffer_size: usize) -> Self {
        Self {
            pool: Arc::new(Pool::new(capacity, move || vec![0u8; buffer_size])),
            buffer_size,
        }
    }

    pub fn acquire(&self) -> Reusable<'_, Vec<u8>> {
        let buf_size = self.buffer_size;
        let mut reusable = self.pool.pull(move || vec![0u8; buf_size]);
        reusable.clear();
        let cap = reusable.capacity();
        reusable.resize(cap, 0);
        reusable
    }

    pub fn acquire_sized(&self, size: usize) -> Reusable<'_, Vec<u8>> {
        let mut reusable = self.pool.pull(move || vec![0u8; size]);
        reusable.clear();
        reusable.resize(size, 0);
        reusable
    }
}

/// Object pool for complex objects
pub struct ObjectPool<T: Default + Send + 'static> {
    pool: Arc<Pool<T>>,
}

impl<T: Default + Send + 'static> ObjectPool<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            pool: Arc::new(Pool::new(capacity, T::default)),
        }
    }

    pub fn acquire(&self) -> Reusable<'_, T> {
        self.pool.pull(T::default)
    }
}

/// Async object pool with semaphore-based limiting
pub struct AsyncObjectPool<T: Default + Send + Sync + 'static> {
    objects: Arc<RwLock<Vec<T>>>,
    semaphore: Arc<Semaphore>,
}

impl<T: Default + Send + Sync + 'static> AsyncObjectPool<T> {
    pub fn new(capacity: usize) -> Self {
        let mut objects = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            objects.push(T::default());
        }

        Self {
            objects: Arc::new(RwLock::new(objects)),
            semaphore: Arc::new(Semaphore::new(capacity)),
        }
    }

    pub async fn acquire(&self) -> PooledObject<T> {
        let permit = self
            .semaphore
            .acquire()
            .await
            .expect("acquire semaphore permit for async object pool");
        permit.forget(); // We'll release manually

        let obj = {
            let mut objects = self.objects.write();
            objects.pop().unwrap_or_default()
        };

        PooledObject {
            object: Some(obj),
            pool: self.objects.clone(),
            semaphore: self.semaphore.clone(),
        }
    }
}

pub struct PooledObject<T> {
    object: Option<T>,
    pool: Arc<RwLock<Vec<T>>>,
    semaphore: Arc<Semaphore>,
}

impl<T> Drop for PooledObject<T> {
    fn drop(&mut self) {
        if let Some(obj) = self.object.take() {
            let mut objects = self.pool.write();
            objects.push(obj);
            self.semaphore.add_permits(1);
        }
    }
}

impl<T> std::ops::Deref for PooledObject<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.object
            .as_ref()
            .expect("PooledObject inner value is present until drop")
    }
}

impl<T> std::ops::DerefMut for PooledObject<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.object
            .as_mut()
            .expect("PooledObject inner value is present until drop")
    }
}

/// Size-limited cache with automatic eviction
pub struct SizeLimitedCache {
    cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    size_tracker: Arc<AtomicUsize>,
    max_size: usize,
    eviction_queue: Arc<Mutex<VecDeque<String>>>,
}

use std::collections::HashMap;

impl SizeLimitedCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            size_tracker: Arc::new(AtomicUsize::new(0)),
            max_size,
            eviction_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn put(&self, key: String, value: Vec<u8>) {
        let value_size = value.len();

        // Check if we need to evict
        while self.size_tracker.load(Ordering::Relaxed) + value_size > self.max_size {
            self.evict_oldest();
        }

        // Insert new value
        {
            let mut cache = self.cache.write();
            if let Some(old_value) = cache.insert(key.clone(), value) {
                self.size_tracker
                    .fetch_sub(old_value.len(), Ordering::Relaxed);
            }
        }

        self.size_tracker.fetch_add(value_size, Ordering::Relaxed);

        let mut queue = self.eviction_queue.lock();
        queue.push_back(key);
    }

    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let cache = self.cache.read();
        cache.get(key).cloned()
    }

    pub fn size(&self) -> usize {
        self.size_tracker.load(Ordering::Relaxed)
    }

    fn evict_oldest(&self) {
        let mut queue = self.eviction_queue.lock();
        if let Some(key) = queue.pop_front() {
            let mut cache = self.cache.write();
            if let Some(value) = cache.remove(&key) {
                self.size_tracker.fetch_sub(value.len(), Ordering::Relaxed);
            }
        }
    }
}

/// Memory tracker for leak detection
pub struct MemoryTracker {
    baseline: AtomicUsize,
    current: AtomicUsize,
    allocations: DashMap<usize, AllocationInfo>,
    next_id: AtomicUsize,
}

#[derive(Clone)]
struct AllocationInfo {
    size: usize,
    timestamp: Instant,
    location: String,
}

impl Default for MemoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryTracker {
    pub fn new() -> Self {
        Self {
            baseline: AtomicUsize::new(Self::get_current_memory()),
            current: AtomicUsize::new(0),
            allocations: DashMap::new(),
            next_id: AtomicUsize::new(0),
        }
    }

    pub fn record_allocation(&self, size: usize, location: &str) -> usize {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.allocations.insert(
            id,
            AllocationInfo {
                size,
                timestamp: Instant::now(),
                location: location.to_string(),
            },
        );
        self.current.fetch_add(size, Ordering::Relaxed);
        id
    }

    pub fn record_deallocation(&self, id: usize) {
        if let Some((_, info)) = self.allocations.remove(&id) {
            self.current.fetch_sub(info.size, Ordering::Relaxed);
        }
    }

    pub fn check_for_leak(&self) -> bool {
        let current = Self::get_current_memory();
        let baseline = self.baseline.load(Ordering::Relaxed);

        // Check if memory has grown significantly
        current > baseline + 10_000_000 // 10MB threshold
    }

    pub fn get_leaked_allocations(&self) -> Vec<(String, usize, Duration)> {
        let now = Instant::now();
        let mut leaks = Vec::new();

        for entry in self.allocations.iter() {
            let info = entry.value();
            let age = now - info.timestamp;

            // Consider allocations older than 30 seconds as potential leaks
            if age > Duration::from_secs(30) {
                leaks.push((info.location.clone(), info.size, age));
            }
        }

        leaks
    }

    fn get_current_memory() -> usize {
        // Platform-specific memory query
        #[cfg(windows)]
        {
            use windows::Win32::System::ProcessStatus::GetProcessMemoryInfo;
            use windows::Win32::System::ProcessStatus::PROCESS_MEMORY_COUNTERS;
            use windows::Win32::System::Threading::GetCurrentProcess;

            unsafe {
                let mut pmc = PROCESS_MEMORY_COUNTERS::default();
                let process = GetCurrentProcess();
                if GetProcessMemoryInfo(
                    process,
                    &mut pmc,
                    std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
                )
                .is_ok()
                {
                    pmc.WorkingSetSize
                } else {
                    0
                }
            }
        }

        #[cfg(not(windows))]
        {
            // Unix/Linux implementation would go here
            0
        }
    }
}

/// Async memory tracker
#[derive(Clone)]
pub struct AsyncMemoryTracker {
    allocations: Arc<RwLock<HashMap<usize, usize>>>,
    total: Arc<AtomicUsize>,
    next_id: Arc<AtomicUsize>,
}

impl Default for AsyncMemoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncMemoryTracker {
    pub fn new() -> Self {
        Self {
            allocations: Arc::new(RwLock::new(HashMap::new())),
            total: Arc::new(AtomicUsize::new(0)),
            next_id: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn record_allocation(&self, size: usize) -> usize {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let mut allocations = self.allocations.write();
        allocations.insert(id, size);

        self.total.fetch_add(size, Ordering::Relaxed);
        id
    }

    pub async fn record_deallocation(&self, id: usize) {
        let mut allocations = self.allocations.write();
        if let Some(size) = allocations.remove(&id) {
            self.total.fetch_sub(size, Ordering::Relaxed);
        }
    }

    pub async fn get_current_usage(&self) -> usize {
        self.total.load(Ordering::Relaxed)
    }
}

/// Allocation patterns for testing
pub enum AllocationPattern {
    Sequential,
    Random,
    Fragmented,
    BurstySmall,
    BurstyLarge,
}

impl AllocationPattern {
    pub fn generate_allocations(&self, count: usize) -> Vec<usize> {
        match self {
            Self::Sequential => (0..count).map(|i| 1024 + i * 128).collect(),
            Self::Random => {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                (0..count).map(|_| rng.gen_range(512..4096)).collect()
            }
            Self::Fragmented => (0..count)
                .map(|i| if i % 2 == 0 { 512 } else { 2048 })
                .collect(),
            Self::BurstySmall => vec![256; count],
            Self::BurstyLarge => vec![8192; count],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_pool() {
        let pool = BufferPool::new(10, 1024);

        let buf1 = pool.acquire();
        assert_eq!(buf1.len(), 1024);

        let buf2 = pool.acquire_sized(512);
        assert_eq!(buf2.len(), 512);
    }

    #[test]
    fn test_size_limited_cache() {
        let cache = SizeLimitedCache::new(1024);

        cache.put("key1".to_string(), vec![0u8; 512]);
        cache.put("key2".to_string(), vec![0u8; 512]);
        cache.put("key3".to_string(), vec![0u8; 512]); // Should evict key1

        assert!(cache.get("key1").is_none());
        assert!(cache.get("key2").is_some());
        assert!(cache.get("key3").is_some());
    }

    #[tokio::test]
    async fn test_async_object_pool() {
        #[derive(Default)]
        struct TestObject {
            data: Vec<u8>,
        }

        let pool = AsyncObjectPool::<TestObject>::new(5);

        let obj1 = pool.acquire().await;
        let _obj2 = pool.acquire().await;

        drop(obj1);
        let _obj3 = pool.acquire().await; // Should reuse obj1
    }
}
