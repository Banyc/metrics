use primitive::{
    dyn_ref::DynRef,
    stable_vec::{SafePtr24, SafeStableVec},
    sync::spmc::{MpmcQueue, MpmcQueueReader},
};

type Writer<Value, const QUEUE_SIZE: usize> = SafePtr24<MpmcQueue<Value, QUEUE_SIZE>>;
type Reader<Value, const QUEUE_SIZE: usize> =
    MpmcQueueReader<Value, QUEUE_SIZE, Writer<Value, QUEUE_SIZE>>;

#[derive(Debug)]
pub struct QueueStore<Key, Value: 'static, const CHUNK_SIZE: usize, const QUEUE_SIZE: usize> {
    store: SafeStableVec<MpmcQueue<Value, QUEUE_SIZE>, CHUNK_SIZE>,
    readers: Vec<(Key, Reader<Value, QUEUE_SIZE>)>,
}
impl<Key, Value: 'static, const CHUNK_SIZE: usize, const QUEUE_SIZE: usize>
    QueueStore<Key, Value, CHUNK_SIZE, QUEUE_SIZE>
{
    pub fn new() -> Self {
        let store = SafeStableVec::new();
        let readers = vec![];
        Self { store, readers }
    }
    pub fn readers(&self) -> &[(Key, Reader<Value, QUEUE_SIZE>)] {
        &self.readers
    }
}
impl<Key, Value: 'static, const CHUNK_SIZE: usize, const QUEUE_SIZE: usize> Default
    for QueueStore<Key, Value, CHUNK_SIZE, QUEUE_SIZE>
{
    fn default() -> Self {
        Self::new()
    }
}
impl<Key, Value: 'static, const CHUNK_SIZE: usize, const QUEUE_SIZE: usize>
    QueueStore<Key, Value, CHUNK_SIZE, QUEUE_SIZE>
where
    Value: Clone,
{
    pub fn create(&mut self, key: Key) -> Writer<Value, QUEUE_SIZE> {
        let queue = MpmcQueue::new();
        let ptr = self.store.push24(queue).into_ref();
        let reader = MpmcQueueReader::new(DynRef::new(ptr.clone(), |queue| queue));
        self.readers.push((key, reader));
        ptr
    }
}
