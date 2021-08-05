///! Contains Basic setup for testing, testable trait and its result type
use anyhow::{Error, Result};

pub enum TestResult {
    /// Test was ok
    Ok,
    /// Test needed to be skipped
    Skip,
    /// Test was error
    Err(Error),
}

impl<T> From<Result<T>> for TestResult {
    fn from(result: Result<T>) -> Self {
        match result {
            Ok(_) => TestResult::Ok,
            Err(err) => TestResult::Err(err),
        }
    }
}

pub trait Testable {
    fn get_name(&self) -> String;
    fn can_run(&self) -> bool {
        true
    }
    fn run(&self) -> TestResult;
}
