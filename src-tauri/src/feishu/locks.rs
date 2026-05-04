//! 按 `open_id` 串行化处理飞书消息
//!
//! 为什么需要：同一用户连续发多条消息时，必须按到达顺序顺次跑流水线，否则会出现：
//! - 第二条消息读历史时漏掉第一条（数据库写入顺序不确定）；
//! - 工具调用 / 状态返回交错，前端体验混乱。
//!
//! 不同用户之间互不影响，所以用 `HashMap<open_id, Arc<Mutex>>`。每次入站消息处理：
//! 1. `lock_for(open_id).await` 拿到该用户的独占锁；
//! 2. 跑完一整轮流水线 + 出站发送；
//! 3. drop guard 释放锁，下一条排队消息醒来。
//!
//! 注意：`HashMap` 用同步 `std::sync::Mutex` 保护，因为只是 entry().or_insert() 这种
//! 极短路径，避免 await 期间持锁；返回的用户级 `tokio::sync::Mutex` 才是异步阻塞的。

use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

pub struct LockMap {
    inner: StdMutex<HashMap<String, Arc<AsyncMutex<()>>>>,
}

impl LockMap {
    pub fn new() -> Self {
        Self {
            inner: StdMutex::new(HashMap::new()),
        }
    }

    /// 取（或新建）某 `open_id` 的串行锁，并 await 拿到独占。
    ///
    /// 返回的 `OwnedMutexGuard` 跨 await 持有也安全。
    pub async fn lock_for(&self, open_id: &str) -> OwnedMutexGuard<()> {
        let mtx = {
            let mut guard = self.inner.lock().expect("lock_map mutex poisoned");
            guard
                .entry(open_id.to_string())
                .or_insert_with(|| Arc::new(AsyncMutex::new(())))
                .clone()
        };
        mtx.lock_owned().await
    }
}

impl Default for LockMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[tokio::test]
    async fn same_open_id_serializes() {
        let map = Arc::new(LockMap::new());
        let counter = Arc::new(AtomicUsize::new(0));
        let observed = Arc::new(StdMutex::new(Vec::<usize>::new()));

        let mut tasks = Vec::new();
        for _ in 0..5 {
            let map = map.clone();
            let counter = counter.clone();
            let observed = observed.clone();
            tasks.push(tokio::spawn(async move {
                let _g = map.lock_for("user-x").await;
                let n = counter.fetch_add(1, Ordering::SeqCst);
                // 在持锁期间故意 yield，验证别人不会插队
                tokio::time::sleep(Duration::from_millis(5)).await;
                observed.lock().unwrap().push(n);
            }));
        }
        for t in tasks {
            t.await.unwrap();
        }
        let mut seq = observed.lock().unwrap().clone();
        // 由于持锁期间 sleep，串行后看到的顺序必须严格递增（即 0,1,2,3,4）
        assert_eq!(seq.len(), 5);
        seq.sort();
        assert_eq!(seq, vec![0, 1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn different_open_ids_do_not_block_each_other() {
        let map = Arc::new(LockMap::new());
        let g1 = map.lock_for("alice").await;
        // 另一个用户应能立即拿锁，不被 alice 阻塞
        let g2 = tokio::time::timeout(
            Duration::from_millis(100),
            map.lock_for("bob"),
        )
        .await
        .expect("bob 应当不被 alice 阻塞");
        drop(g1);
        drop(g2);
    }
}
