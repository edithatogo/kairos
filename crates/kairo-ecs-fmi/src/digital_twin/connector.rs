use crate::{
    error::{validation_error, FmiResult},
    FmiError,
};

#[derive(Debug, Clone, PartialEq)]
pub struct DigitalTwinConnector {
    sample_rate_hz: f64,
    epsilon: f64,
    last_values: std::collections::HashMap<u32, f64>,
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
            last_values: std::collections::HashMap::new(),
        }
    }

    pub fn try_new(sample_rate_hz: f64, epsilon: f64) -> FmiResult<Self> {
        let connector = Self::new(sample_rate_hz, epsilon);
        connector.validate_contract()?;
        Ok(connector)
    }

    pub fn sample_rate_hz(&self) -> f64 {
        self.sample_rate_hz
    }

    pub fn validate_contract(&self) -> FmiResult<()> {
        require_positive_finite("sample_rate_hz", self.sample_rate_hz)?;
        if !self.epsilon.is_finite() || self.epsilon < 0.0 {
            return Err(validation_error(
                "digital twin connector",
                "epsilon must be a finite non-negative value",
            ));
        }
        Ok(())
    }

    pub fn publish_changes(
        &mut self,
        topic_prefix: &str,
        values: impl IntoIterator<Item = (u32, f64)>,
    ) -> Vec<TwinPublication> {
        self.try_publish_changes(topic_prefix, values)
            .expect("digital twin publication contract")
    }

    pub fn try_publish_changes(
        &mut self,
        topic_prefix: &str,
        values: impl IntoIterator<Item = (u32, f64)>,
    ) -> FmiResult<Vec<TwinPublication>> {
        self.validate_contract()?;
        validate_topic_prefix(topic_prefix)?;
        let mut publications = Vec::new();
        for (value_reference, value) in values {
            if !value.is_finite() {
                return Err(validation_error(
                    "digital twin publication",
                    format!(
                        "valueReference {} has non-finite value {}",
                        value_reference, value
                    ),
                ));
            }
            use std::collections::hash_map::Entry;
            match self.last_values.entry(value_reference) {
                Entry::Occupied(mut entry) => {
                    let previous_value = entry.get_mut();
                    if (*previous_value - value).abs() > self.epsilon {
                        *previous_value = value;
                        publications.push(publication(topic_prefix, value_reference, value));
                    }
                }
                Entry::Vacant(entry) => {
                    entry.insert(value);
                    publications.push(publication(topic_prefix, value_reference, value));
                }
            }
        }
        Ok(publications)
    }
}

fn require_positive_finite(field: &'static str, value: f64) -> Result<(), FmiError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(validation_error(
            "digital twin connector",
            format!("{field} must be a finite positive value"),
        ))
    }
}

fn validate_topic_prefix(topic_prefix: &str) -> FmiResult<()> {
    if topic_prefix.trim().is_empty() {
        return Err(validation_error(
            "digital twin publication",
            "topic prefix must not be empty",
        ));
    }
    if topic_prefix.starts_with('/') || topic_prefix.ends_with('/') {
        return Err(validation_error(
            "digital twin publication",
            "topic prefix must not start or end with '/'",
        ));
    }
    Ok(())
}

fn publication(topic_prefix: &str, value_reference: u32, value: f64) -> TwinPublication {
    TwinPublication {
        topic: format!("{topic_prefix}/fmi/{value_reference}"),
        value_reference,
        value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publishes_initial_values_and_delta_changes_only() {
        let mut connector = DigitalTwinConnector::try_new(10.0, 0.1).expect("connector");

        let initial = connector
            .try_publish_changes("asset-a", [(1, 20.0), (2, 40.0)])
            .expect("initial publications");
        assert_eq!(initial.len(), 2);
        assert_eq!(initial[0].topic, "asset-a/fmi/1");

        let unchanged = connector
            .try_publish_changes("asset-a", [(1, 20.05), (2, 40.0)])
            .expect("unchanged publications");
        assert!(unchanged.is_empty());

        let changed = connector
            .try_publish_changes("asset-a", [(1, 20.2)])
            .expect("changed publications");
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].value, 20.2);
    }

    #[test]
    fn rejects_invalid_publication_contracts() {
        assert!(DigitalTwinConnector::try_new(0.0, 0.1).is_err());
        assert!(DigitalTwinConnector::try_new(10.0, -0.1).is_err());

        let mut connector = DigitalTwinConnector::try_new(10.0, 0.1).expect("connector");
        assert!(connector.try_publish_changes("", [(1, 1.0)]).is_err());
        assert!(connector
            .try_publish_changes("asset-a", [(1, f64::NAN)])
            .is_err());
    }
}
