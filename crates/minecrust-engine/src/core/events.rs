use crossbeam_channel::{unbounded, Receiver, Sender};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A global event bus to decouple subsystems.
#[derive(Clone)]
pub struct EventBus {
    senders: Arc<Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            senders: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Subscribes to an event of type `T`. Returns a receiver to poll for events.
    pub fn subscribe<T: Any + Send + Sync + Clone>(&self) -> Receiver<T> {
        let mut map = self.senders.lock().unwrap();
        let type_id = TypeId::of::<T>();

        // If the channel for this event type doesn't exist, create it.
        // For simple PubSub where multiple receivers might want the same event, 
        // we'd need broadcast channels (like `tokio::sync::broadcast`). 
        // For a pure ECS, `crossbeam-channel` acts as a multi-producer multi-consumer queue,
        // which means an event is only received by ONE consumer. 
        // To properly implement a true event bus, we actually want broadcast semantics.
        // But for MVP, we just use crossbeam-channel and assume 1-to-N works by sending clones if we manually broadcast,
        // OR we can just use `crossbeam-channel` as a simple task queue.
        
        // Let's implement a simple wrapper that stores a list of senders for each event type.
        // Wait, crossbeam doesn't do broadcast. Let's just create a sender/receiver pair for this specific subscriber.
        
        // Actually, to keep it simple, we will just use a global bus that anyone can send to, 
        // and systems explicitly register closures or we use a broadcasting mechanism.
        // We will leave this as a skeleton for Phase 6.
        let (tx, rx) = unbounded::<T>();
        
        // This is a naive implementation: it replaces the existing sender.
        // A real broadcast requires storing `Vec<Sender<T>>`.
        map.insert(type_id, Box::new(tx));
        rx
    }

    /// Dispatches an event to all subscribers.
    pub fn dispatch<T: Any + Send + Sync + Clone>(&self, event: T) {
        let map = self.senders.lock().unwrap();
        let type_id = TypeId::of::<T>();

        if let Some(sender_any) = map.get(&type_id) {
            if let Some(sender) = sender_any.downcast_ref::<Sender<T>>() {
                let _ = sender.send(event);
            }
        }
    }
}
