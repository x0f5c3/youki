use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use dbus::arg::RefArg;
use oci_spec::runtime::LinuxMemory;

use crate::common::ControllerOpt;

use super::controller::Controller;

pub struct Memory {}

impl Controller for Memory {
    fn apply(
        options: &ControllerOpt,
        _: u32,
        properties: &mut HashMap<&str, Box<dyn RefArg>>,
    ) -> Result<()> {
        if let Some(memory) = options.resources.memory() {
            log::debug!("Applying memory resource restrictions");
            return Self::apply(memory, properties)
                .context("could not apply memory resource restrictions");
        }

        Ok(())
    }
}

impl Memory {
    fn apply(memory: &LinuxMemory, properties: &mut HashMap<&str, Box<dyn RefArg>>) -> Result<()> {
        if let Some(reservation) = memory.reservation() {
            match reservation {
                1..=i64::MAX => {
                    properties.insert("MemoryLow", Box::new(reservation as u64));
                }
                _ => bail!("invalid memory reservation value: {}", reservation),
            }
        }

        if let Some(limit) = memory.limit() {
            match limit {
                1..=i64::MAX => {
                    properties.insert("MemoryMax", Box::new(limit as u64));
                }
                _ => bail!("invalid memory limit value: {}", limit),
            }
        }

        Self::apply_swap(memory.swap(), memory.limit(), properties).context("could not apply swap")?;
        Ok(())
    }

    // Swap needs to be converted as the runtime spec defines swap as the total of memory + swap,
    // which corresponds to memory.memsw.limit_in_bytes in cgroup v1. In v2 however swap is a
    // separate value (memory.swap.max). Therefore swap needs to be calculated from memory limit
    // and swap. Specified values could be None (no value specified), -1 (unlimited), zero or a
    // positive value. Swap needs to be bigger than the memory limit (due to swap being memory + swap)
    fn apply_swap(
        swap: Option<i64>,
        limit: Option<i64>,
        properties: &mut HashMap<&str, Box<dyn RefArg>>,
    ) -> Result<()> {
        let value: Box<dyn RefArg> = match (limit, swap) {
            // memory is unlimited and swap not specified -> assume swap unlimited
            (Some(-1), None) => Box::new(u64::MAX),
            // if swap is unlimited it can be set to unlimited regardless of memory limit value
            (_, Some(-1)) => Box::new(u64::MAX),
            // if swap is zero, then it needs to be rejected regardless of memory limit value
            // as memory limit would be either bigger (invariant violation) or zero which would
            // leave the container with no memory and no swap.
            // if swap is greater than zero and memory limit is unspecified swap cannot be
            // calulated. If memory limit is zero the container would have only swap. If
            // memory is unlimited it would be bigger than swap.
            (_, Some(0)) | (None | Some(0) | Some(-1), Some(1..=i64::MAX)) => bail!(
                "cgroup v2 swap value cannot be calculated from swap of {} and limit of {}",
                swap.unwrap(),
                limit.map_or("none".to_owned(), |v| v.to_string())
            ),
            (Some(l), Some(s)) if l < s => Box::new((s - l) as u64),
            _ => return Ok(()),
        };

        properties.insert("MemorySwapMax", value);
        Ok(())
    }
}
