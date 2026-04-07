use std::time::Duration;

use crate::probe::ProbeResult;

pub struct InputSettings {
    pub input: ProbedInput,
}

pub struct ProbedInput {
    pub probe_result: ProbeResult,
    pub in_point: Duration,
    pub out_point: Duration,
}
