//! Lock-free SPSC ring buffer for audio transport.
//!
//! The real-time audio callback (Swift AVAudioEngine tap) only calls
//! `push_batch` which does a single `rtrb` memcpy without allocation.
//! The processing pipeline drains via `pop_batch` on a background thread.
//! No audio is ever written to disk.

use rtrb::{Consumer, Producer, RingBuffer};

/// Create a paired SPSC ring. Capacity is rounded to next power of two.
pub fn new_pair(capacity_samples: usize) -> (AudioRingProducer, AudioRingConsumer) {
    let cap = capacity_samples.next_power_of_two().max(1024);
    let (prod, cons) = RingBuffer::new(cap);
    (
        AudioRingProducer {
            producer: prod,
            capacity: cap,
        },
        AudioRingConsumer { consumer: cons },
    )
}

/// Producer half — used from the real-time callback (Swift → Rust via FFI).
pub struct AudioRingProducer {
    producer: Producer<f32>,
    capacity: usize,
}

impl AudioRingProducer {
    /// Push a batch without allocation. Returns number actually pushed;
    /// drops remainder if full (never blocks).
    pub fn push_batch(&mut self, samples: &[f32]) -> usize {
        let mut pushed = 0;
        for &s in samples {
            match self.producer.push(s) {
                Ok(()) => pushed += 1,
                Err(_) => break,
            }
        }
        pushed
    }

    pub fn free_slots(&self) -> usize {
        self.producer.slots()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

/// Consumer half — used from processing thread.
pub struct AudioRingConsumer {
    consumer: Consumer<f32>,
}

impl AudioRingConsumer {
    /// Pop up to `max` samples into `out` (cleared first). Returns popped count.
    pub fn pop_batch(&mut self, out: &mut Vec<f32>, max: usize) -> usize {
        out.clear();
        out.reserve(max);
        let mut count = 0;
        while count < max {
            match self.consumer.pop() {
                Ok(v) => {
                    out.push(v);
                    count += 1;
                }
                Err(_) => break,
            }
        }
        count
    }

    /// Pop all available samples.
    pub fn pop_all(&mut self, out: &mut Vec<f32>) -> usize {
        out.clear();
        while let Ok(v) = self.consumer.pop() {
            out.push(v);
        }
        out.len()
    }

    pub fn is_empty(&self) -> bool {
        self.consumer.is_empty()
    }

    pub fn slots(&self) -> usize {
        self.consumer.slots()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_push_pop() {
        let (mut prod, mut cons) = new_pair(1024);
        let data: Vec<f32> = (0..100).map(|i| i as f32 * 0.01).collect();
        let n = prod.push_batch(&data);
        assert_eq!(n, 100);
        let mut out = Vec::new();
        let m = cons.pop_batch(&mut out, 50);
        assert_eq!(m, 50);
        assert_eq!(out[0], 0.0);
        let k = cons.pop_all(&mut out);
        assert_eq!(k, 50);
    }

    #[test]
    fn ring_overflow_drops() {
        // capacity min is 1024, so push more than that
        let (mut prod, mut cons) = new_pair(64);
        assert_eq!(prod.capacity(), 1024);
        let big: Vec<f32> = vec![0.5; 2000];
        let n = prod.push_batch(&big);
        assert!(n <= 1024);
        assert!(n > 0);
        assert!(n < 2000);
        let mut out = Vec::new();
        cons.pop_all(&mut out);
        assert_eq!(out.len(), n);
    }

    #[test]
    fn ring_empty_pop() {
        let (_prod, mut cons) = new_pair(1024);
        let mut out = Vec::new();
        assert_eq!(cons.pop_batch(&mut out, 10), 0);
        assert!(out.is_empty());
    }
}
