#![warn(unused_crate_dependencies, unused_extern_crates)]
#![warn(unreachable_pub)]

mod abi;
mod barretenberg;
mod build_info;
mod compression;
mod execute;
mod foreign_calls;
mod js_witness_map;
mod logging;
mod public_witness;

pub use abi::{abi_decode, abi_encode};
pub use build_info::build_info;
pub use compression::{compress_witness, decompress_witness};
pub use execute::execute_circuit;
pub use js_witness_map::JsWitnessMap;
pub use logging::{init_log_level, LogLevel};
pub use public_witness::{get_public_parameters_witness, get_public_witness, get_return_witness};
