use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{Mutex, mpsc},
    time::sleep,
};

#[derive(Debug)]
enum EventType {
    HighPriority, // event1
    LowPriority,  // event2
}

#[derive(Clone)]
struct EventManager {
    high_priority_tx: mpsc::Sender<Box<dyn FnOnce() + Send + 'static>>,
    low_priority_tx: mpsc::Sender<Box<dyn FnOnce() + Send + 'static>>,
}

impl EventManager {
    fn new() -> Self {
        let (high_priority_tx, high_priority_rx) = mpsc::channel::<Box<dyn FnOnce() + Send>>(100);
        let (low_priority_tx, low_priority_rx) = mpsc::channel::<Box<dyn FnOnce() + Send>>(1000);

        let high_priority_rx = Arc::new(Mutex::new(high_priority_rx));
        let low_priority_rx = Arc::new(Mutex::new(low_priority_rx));

        tokio::spawn(async move {
            Self::process_events(high_priority_rx, low_priority_rx).await;
        });

        Self {
            high_priority_tx,
            low_priority_tx,
        }
    }

    async fn process_events(
        high_priority_rx: Arc<Mutex<mpsc::Receiver<Box<dyn FnOnce() + Send>>>>,
        low_priority_rx: Arc<Mutex<mpsc::Receiver<Box<dyn FnOnce() + Send>>>>,
    ) {
        loop {
            // First check for any high priority events
            let high_priority_task = {
                let mut rx = high_priority_rx.lock().await;
                rx.try_recv().ok()
            };

            if let Some(task) = high_priority_task {
                // Process high priority task immediately
                task();
                sleep(Duration::from_secs(1)).await;
                continue; // Continue to prioritize high priority tasks
            }

            // Process low-priority events in batches of 20
            let mut processed_low_priority = false;
            for _ in 0..20 {
                let low_priority_task = {
                    let mut rx = low_priority_rx.lock().await;
                    match rx.try_recv() {
                        Ok(task) => Some(task),
                        Err(_) => None,
                    }
                };

                if let Some(task) = low_priority_task {
                    task();
                    sleep(Duration::from_secs(1)).await;
                    processed_low_priority = true;
                } else {
                    break;
                }
            }

            // If no events were processed, sleep briefly to avoid CPU spinning
            if !processed_low_priority {
                sleep(Duration::from_millis(10)).await;
            }
        }
    }

    async fn emit_event<F>(&self, event_type: EventType, callback: F)
    where
        F: FnOnce() + Send + 'static,
    {
        match event_type {
            EventType::HighPriority => {
                self.high_priority_tx
                    .send(Box::new(callback))
                    .await
                    .unwrap();
            }
            EventType::LowPriority => {
                self.low_priority_tx.send(Box::new(callback)).await.unwrap();
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let event_manager = EventManager::new();

    // Emit event2 (low priority) with 1000 subscribers
    for i in 0..1000 {
        let manager = event_manager.clone();
        tokio::spawn(async move {
            manager
                .emit_event(EventType::LowPriority, move || {
                    println!("Low Priority Subscriber {} executed", i);
                })
                .await;
        });
    }

    // Wait 10 seconds before emitting event1 (high priority)
    sleep(Duration::from_secs(10)).await;
    for i in 0..10 {
        let manager = event_manager.clone();
        tokio::spawn(async move {
            manager
                .emit_event(EventType::HighPriority, move || {
                    println!("High Priority Subscriber {} executed", i);
                })
                .await;
        });
    }

    // Wait 20 seconds and emit another high priority event
    sleep(Duration::from_secs(20)).await;
    for i in 10..20 {
        let manager = event_manager.clone();
        tokio::spawn(async move {
            manager
                .emit_event(EventType::HighPriority, move || {
                    println!("High Priority Subscriber {} executed", i);
                })
                .await;
        });
    }

    // Wait 50 seconds and emit another high priority event
    sleep(Duration::from_secs(50)).await;
    for i in 20..30 {
        let manager = event_manager.clone();
        tokio::spawn(async move {
            manager
                .emit_event(EventType::HighPriority, move || {
                    println!("High Priority Subscriber {} executed", i);
                })
                .await;
        });
    }

    // Keep the main thread alive to allow events to be processed
    sleep(Duration::from_secs(100)).await;
}
