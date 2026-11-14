//! In-memory QNN SDK double. Models what a backend *reports*; it proves
//! nothing about real hardware and must never be cited as HTP/GPU
//! conformance.

use std::collections::BTreeMap;

use crate::{
    engine::QnnEngine,
    sdk::{BackendProbe, ProbeFailure, QnnSdk, QnnVersion, SUPPORTED_CORE_API_MAJOR},
};

#[derive(Clone, Debug, Default)]
pub struct MockSdk {
    outcomes: BTreeMap<QnnEngine, Result<BackendProbe, ProbeFailure>>,
}

impl MockSdk {
    /// Every engine fails with [`ProbeFailure::LibraryNotFound`].
    pub fn absent() -> Self {
        Self::default()
    }

    /// Makes `engine` discoverable with a compatible, plausible report.
    pub fn with_backend(mut self, engine: QnnEngine) -> Self {
        let probe = BackendProbe {
            engine,
            provider_name: format!("mock-{}", engine.as_str()),
            backend_id: 1,
            core_api: QnnVersion {
                major: SUPPORTED_CORE_API_MAJOR,
                minor: 0,
                patch: 0,
            },
            backend_api: QnnVersion {
                major: 5,
                minor: 0,
                patch: 0,
            },
        };
        self.outcomes.insert(engine, Ok(probe));
        self
    }

    pub fn with_failure(mut self, engine: QnnEngine, failure: ProbeFailure) -> Self {
        self.outcomes.insert(engine, Err(failure));
        self
    }
}

impl QnnSdk for MockSdk {
    fn probe(&self, engine: QnnEngine) -> Result<BackendProbe, ProbeFailure> {
        self.outcomes
            .get(&engine)
            .cloned()
            .unwrap_or(Err(ProbeFailure::LibraryNotFound))
    }
}
