#[derive(Debug, Clone)]
pub struct MemoryConfig {
    pub max_facts_per_session: i32,
    pub provisional_promote_hits: i64,
    pub analysis_snapshot_char_limit: usize,
    pub quick_note_snapshot_char_limit: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_facts_per_session: 10,
            provisional_promote_hits: 3,
            analysis_snapshot_char_limit: 1800,
            quick_note_snapshot_char_limit: 800,
        }
    }
}
