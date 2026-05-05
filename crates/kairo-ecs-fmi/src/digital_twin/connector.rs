#[derive(Debug, Clone, PartialEq)]
pub struct DigitalTwinConnector {
    sample_rate_hz: f64,
    epsilon: f64,
    last_values: Vec<(u32, f64)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TwinPublication {
    pub topic: String,
    pub value_reference: u32,
    pub value: f64,
}

impl DigitalTwinConnector {
    pub fn new(sample_rate_hz: f64, epsilon: f64) -> Self {
        Self {
            sample_rate_hz,
            epsilon,
            last_values: Vec::new(),
        }
    }

    pub fn sample_rate_hz(&self) -> f64 {
        self.sample_rate_hz
    }

    pub fn publish_changes(
        &mut self,
        topic_prefix: &str,
        values: impl IntoIterator<Item = (u32, f64)>,
    ) -> Vec<TwinPublication> {
        let mut publications = Vec::new();
        for (value_reference, value) in values {
            let previous = self
                .last_values
                .iter_mut()
                .find(|(candidate, _)| *candidate == value_reference);

            match previous {
                Some((_, previous_value)) if (*previous_value - value).abs() <= self.epsilon => {}
                Some((_, previous_value)) => {
                    *previous_value = value;
                    publications.push(publication(topic_prefix, value_reference, value));
                }
                None => {
                    self.last_values.push((value_reference, value));
                    publications.push(publication(topic_prefix, value_reference, value));
                }
            }
        }
        publications
    }
}

fn publication(topic_prefix: &str, value_reference: u32, value: f64) -> TwinPublication {
    TwinPublication {
        topic: format!("{topic_prefix}/fmi/{value_reference}"),
        value_reference,
        value,
    }
}
