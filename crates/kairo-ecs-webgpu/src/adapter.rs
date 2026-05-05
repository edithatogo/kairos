#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdapterStatus {
    BrowserBindingsNotConfigured,
    BrowserApiUnavailable,
    BrowserApiDetected,
}

pub fn detect_adapter() -> AdapterStatus {
    AdapterStatus::BrowserBindingsNotConfigured
}

pub fn is_webgpu_available() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_ci_reports_webgpu_unavailable() {
        assert_eq!(
            detect_adapter(),
            AdapterStatus::BrowserBindingsNotConfigured
        );
        assert!(!is_webgpu_available());
    }
}
