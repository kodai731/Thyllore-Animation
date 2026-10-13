/// A FIFO of one event type: the UI command queue and the dialog requests the platform drains.
#[derive(Clone, Debug)]
pub struct EventQueue<E> {
    events: Vec<E>,
}

impl<E> Default for EventQueue<E> {
    fn default() -> Self {
        Self { events: Vec::new() }
    }
}

impl<E> EventQueue<E> {
    pub fn send(&mut self, event: E) {
        self.events.push(event);
    }

    pub fn drain(&mut self) -> impl Iterator<Item = E> + '_ {
        self.events.drain(..)
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_empty() {
        let queue: EventQueue<u32> = EventQueue::default();
        assert!(queue.is_empty());
    }

    #[test]
    fn test_send_and_drain() {
        let mut queue: EventQueue<u32> = EventQueue::default();
        queue.send(1);
        queue.send(2);
        queue.send(3);
        assert!(!queue.is_empty());

        let drained: Vec<u32> = queue.drain().collect();
        assert_eq!(drained, vec![1, 2, 3]);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_drain_on_empty() {
        let mut queue: EventQueue<u32> = EventQueue::default();
        let drained: Vec<u32> = queue.drain().collect();
        assert!(drained.is_empty());
    }

    #[test]
    fn test_multiple_drains() {
        let mut queue: EventQueue<String> = EventQueue::default();
        queue.send("first".to_string());
        let first_drain: Vec<String> = queue.drain().collect();
        assert_eq!(first_drain, vec!["first"]);

        queue.send("second".to_string());
        let second_drain: Vec<String> = queue.drain().collect();
        assert_eq!(second_drain, vec!["second"]);
    }
}
