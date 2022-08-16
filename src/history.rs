use std::collections::{HashMap, VecDeque};
use tokio::sync::RwLock;

pub struct MessageHistory {
    map: RwLock<HashMap<String, VecDeque<String>>>,
    maxlen: usize,
}

impl MessageHistory {
    pub fn new(maxlen: usize) -> MessageHistory {
        MessageHistory {
            map: RwLock::new(HashMap::new()),
            maxlen,
        }
    }

    pub async fn last_msg(&self, user: &str) -> Option<String> {
        let map = self.map.read().await;
        map.get(user)
            .and_then(|d| d.get(0))
            .map(ToString::to_string)
    }

    pub async fn last_msgs(&self, user: &str, count: usize) -> Option<Vec<String>> {
        let map = self.map.read().await;
        if let Some(deque) = map.get(user) {
            let count = if deque.len() < count {
                deque.len()
            } else {
                count
            };
            Some(
                deque
                    .range(..count)
                    .rev()
                    .map(ToString::to_string)
                    .collect(),
            )
        } else {
            None
        }
    }

    pub async fn edit_message(&self, user: &str, pos: usize, edited: String) -> bool {
        let mut map = self.map.write().await;
        if let Some(deque) = map.get_mut(user) {
            if let Some(old) = deque.get_mut(pos) {
                *old = edited;
                return true;
            }
        }
        false
    }

    pub async fn add_message(&self, user: &str, message: String) {
        let mut map = self.map.write().await;
        if let Some(deque) = map.get_mut(user) {
            if deque.len() == self.maxlen {
                deque.remove(deque.len() - 1);
            }
            deque.push_front(message);
        } else {
            let mut deque = VecDeque::with_capacity(self.maxlen);
            deque.push_front(message);
            map.insert(user.to_string(), deque);
        }
    }
}
