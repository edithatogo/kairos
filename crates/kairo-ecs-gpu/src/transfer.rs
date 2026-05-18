#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferDirection {
    HostToGpu,
    GpuToHost,
    ZeroCopyBorrow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferStep {
    pub label: String,
    pub direction: TransferDirection,
    pub len_bytes: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TransferPlan {
    steps: Vec<TransferStep>,
}

impl TransferPlan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn push(&mut self, step: TransferStep) {
        self.steps.push(step);
    }

    pub fn steps(&self) -> &[TransferStep] {
        &self.steps
    }

    pub fn total_host_to_gpu_bytes(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.direction == TransferDirection::HostToGpu)
            .map(|step| step.len_bytes)
            .sum()
    }

    pub fn checked_total_host_to_gpu_bytes(&self) -> Option<usize> {
        self.steps
            .iter()
            .filter(|step| step.direction == TransferDirection::HostToGpu)
            .try_fold(0usize, |total, step| total.checked_add(step.len_bytes))
    }

    pub fn total_gpu_to_host_bytes(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.direction == TransferDirection::GpuToHost)
            .map(|step| step.len_bytes)
            .sum()
    }

    pub fn checked_total_gpu_to_host_bytes(&self) -> Option<usize> {
        self.steps
            .iter()
            .filter(|step| step.direction == TransferDirection::GpuToHost)
            .try_fold(0usize, |total, step| total.checked_add(step.len_bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_plan_separates_upload_and_download_totals() {
        let mut plan = TransferPlan::new();
        plan.push(TransferStep {
            label: "particles".into(),
            direction: TransferDirection::HostToGpu,
            len_bytes: 128,
        });
        plan.push(TransferStep {
            label: "results".into(),
            direction: TransferDirection::GpuToHost,
            len_bytes: 64,
        });

        assert_eq!(plan.total_host_to_gpu_bytes(), 128);
        assert_eq!(plan.total_gpu_to_host_bytes(), 64);
        assert_eq!(plan.checked_total_host_to_gpu_bytes(), Some(128));
        assert_eq!(plan.checked_total_gpu_to_host_bytes(), Some(64));
    }

    #[test]
    fn checked_transfer_totals_report_overflow() {
        let mut plan = TransferPlan::new();
        plan.push(TransferStep {
            label: "a".into(),
            direction: TransferDirection::HostToGpu,
            len_bytes: usize::MAX,
        });
        plan.push(TransferStep {
            label: "b".into(),
            direction: TransferDirection::HostToGpu,
            len_bytes: 1,
        });

        assert_eq!(plan.checked_total_host_to_gpu_bytes(), None);
    }
}
