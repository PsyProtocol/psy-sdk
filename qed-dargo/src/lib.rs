mod package;
mod toml;

use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;

pub const FILE_EXTENSION: &str = "qed";
