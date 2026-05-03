#[derive(Debug, Clone)]
pub struct MemoryConfig {
    pub max_facts_per_session: i32,
    pub provisional_promote_hits: i64,
    pub analysis_snapshot_char_limit: usize,
    pub quick_note_snapshot_char_limit: usize,
    pub auto_decay_days: i64,
    pub auto_decay_confidence_threshold: f32,
    pub snapshot_role_budget_ratio: f32,
    pub snapshot_min_fact_chars: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_facts_per_session: 10,
            provisional_promote_hits: 3,
            analysis_snapshot_char_limit: 1800,
            quick_note_snapshot_char_limit: 800,
            auto_decay_days: 90,
            auto_decay_confidence_threshold: 0.5,
            snapshot_role_budget_ratio: 0.25,
            snapshot_min_fact_chars: 80,
        }
    }
}
