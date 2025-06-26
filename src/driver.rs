use futures::task::Waker;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::pin::Pin;
use std::task::{Context, Poll};

thread_local! {
    static CURRENT_CYCLE: RefCell<usize> = RefCell::new(0);
    static NEXT_WAKE_CYCLE: RefCell<Option<usize>> = RefCell::new(None);
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

pub struct Driver {
    clock: usize,
    futures_queue: BTreeMap<usize, Vec<Pin<Box<dyn Future<Output = ()>>>>>,
}

impl Driver {
    pub fn new() -> Self {
        Self {
            clock: 0,
            futures_queue: BTreeMap::new(),
        }
    }

    pub fn spawn<F>(&mut self, future: F)
    where
        F: Future<Output = ()> + 'static,
    {
        self.futures_queue
            .entry(self.clock)
            .or_insert_with(Vec::new)
            .push(Box::pin(future));
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
                        .or_insert_with(Vec::new)
                        .push(future);
                }
            }
        }
    }
}
