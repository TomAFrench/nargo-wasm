use acvm::{
    acir::{
        circuit::{
            opcodes::{BlackBoxFuncCall, OracleData},
            Circuit,
        },
        native_types::Witness,
        BlackBoxFunc,
    },
    pwg::{block::Blocks, directives::insert_witness, hash, logic, range, signature},
    FieldElement, OpcodeResolution, OpcodeResolutionError, PartialWitnessGenerator,
    PartialWitnessGeneratorStatus,
};
use std::collections::BTreeMap;

use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

use crate::js_transforms::{
    field_element_to_js_string, js_map_to_witness_map, js_value_to_field_element,
    witness_map_to_js_map,
};

struct SimulatedBackend;

impl PartialWitnessGenerator for SimulatedBackend {
    fn solve_black_box_function_call(
        &self,
        initial_witness: &mut BTreeMap<Witness, FieldElement>,
        func_call: &BlackBoxFuncCall,
    ) -> Result<OpcodeResolution, OpcodeResolutionError> {
        match func_call.name {
            BlackBoxFunc::SHA256 => hash::sha256(initial_witness, func_call),
            BlackBoxFunc::Blake2s => hash::blake2s(initial_witness, func_call),
            BlackBoxFunc::EcdsaSecp256k1 => {
                signature::ecdsa::secp256k1_prehashed(initial_witness, func_call)
            }
            BlackBoxFunc::AND | BlackBoxFunc::XOR => {
                logic::solve_logic_opcode(initial_witness, func_call)
            }
            BlackBoxFunc::RANGE => range::solve_range_opcode(initial_witness, func_call),
            BlackBoxFunc::HashToField128Security => {
                acvm::pwg::hash::hash_to_field_128_security(initial_witness, func_call)
            }
            BlackBoxFunc::Keccak256 => todo!("need to update to a newer version of ACVM"),
            BlackBoxFunc::AES
            | BlackBoxFunc::Pedersen
            | BlackBoxFunc::ComputeMerkleRoot
            | BlackBoxFunc::FixedBaseScalarMul
            | BlackBoxFunc::SchnorrVerify => {
                unimplemented!("opcode does not have a rust implementation")
            }
        }
    }
}

#[wasm_bindgen]
pub async fn execute_circuit(
    circuit: Vec<u8>,
    initial_witness: js_sys::Map,
    oracle_resolver: js_sys::Function,
) -> js_sys::Map {
    console_error_panic_hook::set_once();
    let circuit: Circuit = Circuit::read(&*circuit).expect("Failed to deserialize circuit");
    let mut witness_map = js_map_to_witness_map(initial_witness);

    let mut blocks = Blocks::default();
    let mut opcodes = circuit.opcodes;

    let mut i = 0;
    loop {
        let solver_status = SimulatedBackend
            .solve(&mut witness_map, &mut blocks, opcodes)
            .expect("Threw error while executing circuit");
        i += 1;
        if i > 4 {
            panic!("too many loops");
        }

        match solver_status {
            PartialWitnessGeneratorStatus::Solved => break,
            PartialWitnessGeneratorStatus::RequiresOracleData {
                required_oracle_data,
                unsolved_opcodes,
            } => {
                // Perform all oracle queries
                let oracle_call_futures: Vec<_> = required_oracle_data
                    .into_iter()
                    .map(|oracle_call| resolve_oracle(&oracle_resolver, oracle_call))
                    .collect();

                // Insert results into the witness map
                for oracle_call_future in oracle_call_futures {
                    let resolved_oracle_call: OracleData = oracle_call_future.await.unwrap();
                    for (i, witness_index) in resolved_oracle_call.outputs.iter().enumerate() {
                        insert_witness(
                            *witness_index,
                            resolved_oracle_call.output_values[i],
                            &mut witness_map,
                        )
                        .expect("inserted inconsistent witness value");
                    }
                }

                // Use new opcodes as returned by ACVM.
                opcodes = unsolved_opcodes;
            }
        }
    }

    witness_map_to_js_map(witness_map)
}

async fn resolve_oracle(
    oracle_resolver: &js_sys::Function,
    mut unresolved_oracle_call: OracleData,
) -> Result<OracleData, String> {
    // Prepare to call
    let name = JsValue::from(unresolved_oracle_call.name.clone());
    assert_eq!(unresolved_oracle_call.inputs.len(), unresolved_oracle_call.input_values.len());
    let inputs = js_sys::Array::default();
    for input_value in &unresolved_oracle_call.input_values {
        let hex_js_string = field_element_to_js_string(input_value);
        inputs.push(&hex_js_string);
    }

    // Call and await
    let this = JsValue::null();
    let ret_js_val = oracle_resolver
        .call2(&this, &name, &inputs)
        .map_err(|err| format!("Error calling oracle_resolver: {}", format_js_err(err)))?;
    let ret_js_prom: js_sys::Promise = ret_js_val.into();
    let ret_future: wasm_bindgen_futures::JsFuture = ret_js_prom.into();
    let js_resolution = ret_future
        .await
        .map_err(|err| format!("Error awaiting oracle_resolver: {}", format_js_err(err)))?;

    // Check that result conforms to expected shape.
    if !js_resolution.is_array() {
        return Err("oracle_resolver must return a Promise<string[]>".into());
    }
    let js_arr = js_sys::Array::from(&js_resolution);
    let output_len = js_arr.length() as usize;
    let expected_output_len = unresolved_oracle_call.outputs.len();
    if output_len != expected_output_len {
        return Err(format!(
            "Expected output from oracle '{}' of {} elements, but instead received {}",
            unresolved_oracle_call.name, expected_output_len, output_len
        ));
    }

    // Insert result into oracle data.
    for elem in js_arr.iter() {
        if !elem.is_string() {
            return Err("Non-string element in oracle_resolver return".into());
        }
        unresolved_oracle_call.output_values.push(js_value_to_field_element(elem)?)
    }
    let resolved_oracle_call = unresolved_oracle_call;

    Ok(resolved_oracle_call)
}

fn format_js_err(err: JsValue) -> String {
    match err.as_string() {
        Some(str) => str,
        None => "Unknown".to_owned(),
    }
}