use futures::task::Waker;
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

// Type alias for complex future queue type
type FutureQueue = BTreeMap<usize, Vec<Pin<Box<dyn Future<Output = ()>>>>>;

// Events that can be emitted by the driver for the host emulator to handle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    VBlank,          // Frame complete, display framebuffer to screen
    AudioBufferFull, // Audio buffer full, drain to audio system
    MaxCycles,       // Reached cycle limit
}

// Result returned from run_for method
#[derive(Debug)]
pub struct ExecutionResult {
    pub event: Event,
    pub cycles_executed: usize,
}

thread_local! {
    static CURRENT_CYCLE: RefCell<usize> = const { RefCell::new(0) };
    static NEXT_WAKE_CYCLE: RefCell<Option<usize>> = const { RefCell::new(None) };
    static PENDING_EVENT: RefCell<Option<Event>> = const { RefCell::new(None) };
}

#[derive(Debug)]
pub struct Sleep {
    cycles: usize,
    target_cycle: Option<usize>,
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        // Get the current cycle.
        let current_cycle = CURRENT_CYCLE.with(|cell| *cell.borrow());

        if this.target_cycle.is_none() {
            this.target_cycle = Some(current_cycle + this.cycles);

            // Write the target cycle to thread_local variable for driver to read.
            NEXT_WAKE_CYCLE.with(|cell| {
                *cell.borrow_mut() = Some(this.target_cycle.unwrap());
            });

            Poll::Pending
        } else {
            let target_cycle = this.target_cycle.unwrap();
            assert_eq!(
                current_cycle, target_cycle,
                "Future polled on incorrect cycle. Target cycle {} != current cycle {}",
                target_cycle, current_cycle
            );

            Poll::Ready(())
        }
    }
}

pub fn sleep(cycles: usize) -> Sleep {
    Sleep {
        cycles,
        target_cycle: None,
    }
}

// Function for components to emit events
pub fn emit_event(event: Event) {
    PENDING_EVENT.with(|cell| {
        *cell.borrow_mut() = Some(event);
    });
}

pub struct Driver {
    clock: usize,
    futures_queue: FutureQueue,
    events_queue: VecDeque<Event>,
}

impl Default for Driver {
    fn default() -> Self {
        Self::new()
    }
}

impl Driver {
    pub fn new() -> Self {
        Self {
            clock: 0,
            futures_queue: BTreeMap::new(),
            events_queue: VecDeque::new(),
        }
    }

    pub fn spawn<F>(&mut self, future: F)
    where
        F: Future<Output = ()> + 'static,
    {
        self.futures_queue
            .entry(self.clock)
            .or_default()
            .push(Box::pin(future));
    }

    pub fn current_cycle(&self) -> usize {
        self.clock
    }

    pub fn run_for(&mut self, max_cycles: usize) -> ExecutionResult {
        // If any events are yet to be handled, do that first
        if let Some(event) = self.events_queue.pop_front() {
            // Exit early due to unhandled event
            return ExecutionResult {
                event,
                cycles_executed: 0,
            };
        }

        let start_cycle = self.clock;
        let target_cycle = start_cycle + max_cycles;

        while !self.futures_queue.is_empty() && self.clock < target_cycle {
            let next_cycle = *self.futures_queue.keys().next().unwrap();

            // Don't exceed our cycle limit
            if next_cycle >= target_cycle {
                break;
            }

            // Advance clock directly to this cycle
            self.clock = next_cycle;

            // Reflect change in current cycle
            CURRENT_CYCLE.with(|cell| {
                *cell.borrow_mut() = self.clock;
            });

            // Get futures to poll during this cycle
            let futures = self.futures_queue.remove(&next_cycle).unwrap();
            let mut cx = Context::from_waker(Waker::noop());

            for mut future in futures {
                // Reset next wake cycle
                NEXT_WAKE_CYCLE.with(|cell| {
                    *cell.borrow_mut() = None;
                });

                // If not done, re-queue
                if future.as_mut().poll(&mut cx).is_pending() {
                    let wake_cycle = NEXT_WAKE_CYCLE
                        .with(|cell| *cell.borrow())
                        .unwrap_or(self.clock + 1);

                    self.futures_queue
                        .entry(wake_cycle)
                        .or_default()
                        .push(future);
                }

                // Check for events after each future poll
                let pending_event = PENDING_EVENT.with(|cell| *cell.borrow());
                if let Some(event) = pending_event {
                    // Clear the event and add to queue
                    PENDING_EVENT.with(|cell| {
                        *cell.borrow_mut() = None;
                    });
                    self.events_queue.push_back(event);
                }
            }

            if let Some(event) = self.events_queue.pop_front() {
                // Exit early due to unhandled event
                return ExecutionResult {
                    event,
                    cycles_executed: self.clock - start_cycle,
                };
            }
        }

        // If we reach here, we either ran out of futures or hit the cycle limit
        ExecutionResult {
            event: Event::MaxCycles,
            cycles_executed: self.clock - start_cycle,
        }
    }

    pub fn run(&mut self) {
        while !self.futures_queue.is_empty() {
            let next_cycle = *self.futures_queue.keys().next().unwrap();

            // Advance clock directly to this cycle.
            self.clock = next_cycle;

            // Reflect change in current cycle.
            CURRENT_CYCLE.with(|cell| {
                *cell.borrow_mut() = self.clock;
            });

            // Get futures to poll during this cycle.
            let futures = self.futures_queue.remove(&next_cycle).unwrap();

            let mut cx = Context::from_waker(Waker::noop());

            for mut future in futures {
                // Reset next wake cycle.
                NEXT_WAKE_CYCLE.with(|cell| {
                    *cell.borrow_mut() = None;
                });

                // If not done, re-queue.
                if future.as_mut().poll(&mut cx).is_pending() {
                    let wake_cycle = NEXT_WAKE_CYCLE
                        .with(|cell| *cell.borrow())
                        .unwrap_or(self.clock + 1);

                    self.futures_queue
                        .entry(wake_cycle)
                        .or_default()
                        .push(future);
                }
            }
        }
    }
}
