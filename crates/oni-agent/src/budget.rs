use std::time::Instant;

pub struct BudgetTracker {
    total_prompt_tokens: u64,
    total_eval_tokens: u64,
    total_duration_ns: u64,
    turn_count: u32,
    started_at: Instant,
}

impl BudgetTracker {
    pub fn new() -> Self {
        Self {
            total_prompt_tokens: 0,
            total_eval_tokens: 0,
            total_duration_ns: 0,
            turn_count: 0,
            started_at: Instant::now(),
        }
    }

    pub fn record_turn(
        &mut self,
        prompt_tokens: u64,
        eval_tokens: u64,
        duration_ns: u64,
    ) {
        self.total_prompt_tokens += prompt_tokens;
        self.total_eval_tokens += eval_tokens;
        self.total_duration_ns += duration_ns;
        self.turn_count += 1;
    }

    pub fn total_tokens(&self) -> u64 {
        self.total_prompt_tokens + self.total_eval_tokens
    }

    pub fn eval_tokens(&self) -> u64 {
        self.total_eval_tokens
    }

    pub fn turn_count(&self) -> u32 {
        self.turn_count
    }

    pub fn tokens_per_second(&self) -> f64 {
        if self.total_duration_ns == 0 {
            return 0.0;
        }
        self.total_eval_tokens as f64 / (self.total_duration_ns as f64 / 1_000_000_000.0)
    }

    pub fn session_duration_secs(&self) -> f64 {
        self.started_at.elapsed().as_secs_f64()
    }
}

impl Default for BudgetTracker {
    fn default() -> Self {
        Self::new()
    }
}
