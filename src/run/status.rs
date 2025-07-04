use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum JobStatus {
    TOSUBMIT,
    SUBMITTED,
    DONE,
    FAILED,
    RETRIED
}
