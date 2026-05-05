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

    pub fn total_gpu_to_host_bytes(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.direction == TransferDirection::GpuToHost)
            .map(|step| step.len_bytes)
            .sum()
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
    }
}
