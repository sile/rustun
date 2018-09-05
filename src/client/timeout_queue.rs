use fibers::time::timer::{self, Timeout};
use futures::{Async, Future};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub struct TimeoutQueue<T> {
    queue: BinaryHeap<Item<T>>,
    timeout: Option<Timeout>,
}
impl<T> TimeoutQueue<T> {
    pub fn new() -> Self {
        TimeoutQueue {
            queue: BinaryHeap::new(),
            timeout: None,
        }
    }

    pub fn push(&mut self, item: T, timeout: Duration) {
        let expiry_time = SystemTime::now() + timeout;
        self.queue.push(Item { expiry_time, item });
        if self.queue.peek().map(|x| x.expiry_time) == Some(expiry_time) {
            self.timeout = None;
        }
        self.poll_timeout();
    }

    pub fn pop_expired<F>(&mut self, contains: F) -> Option<T>
    where
        F: Fn(&T) -> bool,
    {
        let now = SystemTime::now();
        while let Some(x) = self.queue.pop() {
            if !contains(&x.item) {
                continue;
            }
            if x.expiry_time > now {
                self.queue.push(x);
                break;
            }
            return Some(x.item);
        }
        self.poll_timeout();
        None
    }

    fn poll_timeout(&mut self) {
        if let Ok(Async::Ready(_)) = self.timeout.poll() {
            if let Some(timeout) = self
                .queue
                .peek()
                .and_then(|x| x.expiry_time.duration_since(SystemTime::now()).ok())
            {
                self.timeout = Some(timer::timeout(timeout));
            } else {
                self.timeout = None;
            }
        }
    }
}

#[derive(Debug)]
struct Item<T> {
    expiry_time: SystemTime,
    item: T,
}
impl<T> PartialOrd for Item<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.expiry_time.partial_cmp(&self.expiry_time)
    }
}
impl<T> Ord for Item<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        other.expiry_time.cmp(&self.expiry_time)
    }
}
impl<T> PartialEq for Item<T> {
    fn eq(&self, other: &Self) -> bool {
        self.expiry_time == other.expiry_time
    }
}
impl<T> Eq for Item<T> {}
