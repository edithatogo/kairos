#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdapterStatus {
    BrowserBindingsNotConfigured,
    BrowserApiUnavailable,
    BrowserApiDetected,
    Available,
}

#[cfg(not(test))]
pub fn detect_adapter() -> AdapterStatus {
    AdapterStatus::BrowserBindingsNotConfigured
}

#[cfg(test)]
thread_local! {
    pub(crate) static MOCK_ADAPTER_STATUS: std::cell::RefCell<Option<AdapterStatus>> = std::cell::RefCell::new(None);
}

#[cfg(test)]
pub fn detect_adapter() -> AdapterStatus {
    MOCK_ADAPTER_STATUS.with(|status| status.borrow().unwrap_or(AdapterStatus::BrowserBindingsNotConfigured))
}

pub fn is_webgpu_available() -> bool {
    detect_adapter() == AdapterStatus::Available
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

    #[test]
    fn test_is_webgpu_available_true() {
        MOCK_ADAPTER_STATUS.with(|status| *status.borrow_mut() = Some(AdapterStatus::Available));
        assert!(is_webgpu_available());
        MOCK_ADAPTER_STATUS.with(|status| *status.borrow_mut() = None);
    }

    #[test]
    fn test_is_webgpu_available_false_when_unavailable() {
        MOCK_ADAPTER_STATUS.with(|status| *status.borrow_mut() = Some(AdapterStatus::BrowserApiUnavailable));
        assert!(!is_webgpu_available());
        MOCK_ADAPTER_STATUS.with(|status| *status.borrow_mut() = None);
    }

    #[test]
    fn test_is_webgpu_available_false_when_detected_but_not_available() {
        MOCK_ADAPTER_STATUS.with(|status| *status.borrow_mut() = Some(AdapterStatus::BrowserApiDetected));
        assert!(!is_webgpu_available());
        MOCK_ADAPTER_STATUS.with(|status| *status.borrow_mut() = None);
    }
}
