use std::time::Instant;

/// Mutable context that flows through the entire pipeline.
#[derive(Debug)]
pub struct RequestContext {
    pub request_id: String,
    pub selected_provider: Option<String>,
    pub selected_key_index: Option<usize>,
    pub selected_key_value: Option<String>,
    pub retry_count: u32,
    pub start_time: Instant,
}

impl Default for RequestContext {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestContext {
    pub fn new() -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            selected_provider: None,
            selected_key_index: None,
            selected_key_value: None,
            retry_count: 0,
            start_time: Instant::now(),
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }
}
