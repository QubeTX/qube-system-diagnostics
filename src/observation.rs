use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationStatus {
    Available,
    Unavailable,
    Unsupported,
    PermissionDenied,
    Error,
    Contradictory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Observation {
    pub status: ObservationStatus,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl Observation {
    pub fn available(source: impl Into<String>) -> Self {
        Self {
            status: ObservationStatus::Available,
            source: source.into(),
            detail: None,
        }
    }

    pub fn unavailable(source: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            status: ObservationStatus::Unavailable,
            source: source.into(),
            detail: Some(detail.into()),
        }
    }

    pub fn unsupported(source: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            status: ObservationStatus::Unsupported,
            source: source.into(),
            detail: Some(detail.into()),
        }
    }

    pub fn permission_denied(source: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            status: ObservationStatus::PermissionDenied,
            source: source.into(),
            detail: Some(detail.into()),
        }
    }

    pub fn error(source: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            status: ObservationStatus::Error,
            source: source.into(),
            detail: Some(detail.into()),
        }
    }

    pub fn contradictory(source: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            status: ObservationStatus::Contradictory,
            source: source.into(),
            detail: Some(detail.into()),
        }
    }

    pub fn is_available(&self) -> bool {
        self.status == ObservationStatus::Available
    }
}

impl Default for Observation {
    fn default() -> Self {
        Self::unavailable("not_collected", "The collector has not run yet")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_is_not_healthy_or_available() {
        let observation = Observation::unavailable("fixture", "provider returned no rows");
        assert!(!observation.is_available());
        assert_eq!(observation.status, ObservationStatus::Unavailable);
    }
}
