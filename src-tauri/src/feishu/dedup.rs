//! 飞书入站消息去重缓存
//!
//! 飞书 IM 事件存在重投递可能（push 失败重试 / 客户端确认失序），同一个 `message_id`
//! 可能短时间内到达多次。本模块以 `message_id` 为 key 做"出现即吃掉"的过滤：
//! - 容量上限 2048（设计文档 §8.1）；
//! - 单条 TTL 24h（足以覆盖飞书最长重投递窗口）；
//! - 写入时同时驱逐过期项 + 超容时驱逐最旧项。
//!
//! 使用方式：在 [`mark_seen_or_dup`] 返回 `true` 时直接丢弃事件。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// `message_id` 去重缓存。
///
/// 内部用单个 `Mutex<HashMap>`，因为热路径 QPS 很低（最多用户打字速度），
/// 锁竞争不是瓶颈；优先实现简单可读。
pub struct Dedup {
    inner: Mutex<DedupInner>,
    capacity: usize,
    ttl: Duration,
}

struct DedupInner {
    map: HashMap<String, Instant>,
}

impl Dedup {
    /// `capacity`：缓存最多保留多少条；`ttl`：单条最长有效期。
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            inner: Mutex::new(DedupInner {
                map: HashMap::with_capacity(capacity.min(4096)),
            }),
            capacity,
            ttl,
        }
    }

    /// 标记一个 `message_id`：
    /// - 若已存在且未过期 → 返回 `true`（重复，调用方应跳过该消息）；
    /// - 否则插入并返回 `false`。
    ///
    /// 副作用：每次调用顺手淘汰过期项；超容时驱逐最早的一条。
    pub fn mark_seen_or_dup(&self, message_id: &str) -> bool {
        let now = Instant::now();
        let mut inner = self.inner.lock().expect("dedup mutex poisoned");

        // 命中 → 重复
        if let Some(seen_at) = inner.map.get(message_id) {
            if now.duration_since(*seen_at) <= self.ttl {
                return true;
            }
            // 过期，下面统一 prune 并重新写
        }

        // 淘汰过期项
        let ttl = self.ttl;
        inner.map.retain(|_, t| now.duration_since(*t) <= ttl);

        // 超容 → 找出最早的一条 evict
        if inner.map.len() >= self.capacity {
            if let Some((oldest_key, _)) = inner
                .map
                .iter()
                .min_by_key(|(_, t)| **t)
                .map(|(k, t)| (k.clone(), *t))
            {
                inner.map.remove(&oldest_key);
            }
        }

        inner.map.insert(message_id.to_string(), now);
        false
    }

    /// 当前缓存条数（测试用）。
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.inner.lock().expect("dedup mutex poisoned").map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_seen_returns_false_repeats_return_true() {
        let d = Dedup::new(16, Duration::from_secs(60));
        assert!(!d.mark_seen_or_dup("m1"));
        assert!(d.mark_seen_or_dup("m1"));
        assert!(d.mark_seen_or_dup("m1"));
    }

    #[test]
    fn distinct_ids_do_not_collide() {
        let d = Dedup::new(16, Duration::from_secs(60));
        assert!(!d.mark_seen_or_dup("a"));
        assert!(!d.mark_seen_or_dup("b"));
        assert!(!d.mark_seen_or_dup("c"));
        assert_eq!(d.len(), 3);
    }

    #[test]
    fn over_capacity_evicts_oldest() {
        let d = Dedup::new(3, Duration::from_secs(60));
        assert!(!d.mark_seen_or_dup("first"));
        // 让 "first" 时间戳最早
        std::thread::sleep(Duration::from_millis(5));
        assert!(!d.mark_seen_or_dup("second"));
        std::thread::sleep(Duration::from_millis(5));
        assert!(!d.mark_seen_or_dup("third"));
        std::thread::sleep(Duration::from_millis(5));
        // 第 4 个应当驱逐 "first"
        assert!(!d.mark_seen_or_dup("fourth"));
        assert_eq!(d.len(), 3);
        // "first" 已被驱逐 → 再来一次仍是首次
        assert!(!d.mark_seen_or_dup("first"));
    }

    #[test]
    fn ttl_expiry_treats_as_first_seen() {
        let d = Dedup::new(16, Duration::from_millis(20));
        assert!(!d.mark_seen_or_dup("ephemeral"));
        std::thread::sleep(Duration::from_millis(40));
        // 已过期 → 视为首见
        assert!(!d.mark_seen_or_dup("ephemeral"));
    }
}
