#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdapterStatus {
    Unavailable,
    Available,
}

pub fn detect_adapter() -> AdapterStatus {
    if is_webgpu_available() {
        AdapterStatus::Available
    } else {
        AdapterStatus::Unavailable
    }
}

pub fn is_webgpu_available() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_ci_reports_webgpu_unavailable() {
        assert_eq!(detect_adapter(), AdapterStatus::Unavailable);
    }
}
