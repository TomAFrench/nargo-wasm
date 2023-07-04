use acvm::{
    acir::{circuit::Circuit, BlackBoxFunc},
    pwg::{OpcodeResolutionError, PartialWitnessGeneratorStatus, ACVM},
    BlackBoxFunctionSolver, FieldElement,
};

use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

use crate::{
    barretenberg::{pedersen::Pedersen, scalar_mul::ScalarMul, schnorr::SchnorrSig, Barretenberg},
    foreign_calls::ForeignCallHandler,
    JsWitnessMap,
};

struct SimulatedBackend {
    blackbox_vendor: Barretenberg,
}

impl SimulatedBackend {
    async fn initialize() -> SimulatedBackend {
        let blackbox_vendor = Barretenberg::new().await;
        SimulatedBackend { blackbox_vendor }
    }
}

impl BlackBoxFunctionSolver for SimulatedBackend {
    fn schnorr_verify(
        &self,
        public_key_x: &FieldElement,
        public_key_y: &FieldElement,
        signature_s: &FieldElement,
        signature_e: &FieldElement,
        message: &[u8],
    ) -> Result<bool, OpcodeResolutionError> {
        let pub_key_bytes: Vec<u8> =
            public_key_x.to_be_bytes().iter().copied().chain(public_key_y.to_be_bytes()).collect();

        let pub_key: [u8; 64] = pub_key_bytes.try_into().unwrap();
        let sig_s: [u8; 32] = signature_s.to_be_bytes().try_into().unwrap();
        let sig_e: [u8; 32] = signature_e.to_be_bytes().try_into().unwrap();

        self.blackbox_vendor.verify_signature(pub_key, sig_s, sig_e, message).map_err(|err| {
            OpcodeResolutionError::BlackBoxFunctionFailed(
                BlackBoxFunc::SchnorrVerify,
                err.to_string(),
            )
        })
    }

    fn pedersen(
        &self,
        inputs: &[FieldElement],
        domain_separator: u32,
    ) -> Result<(FieldElement, FieldElement), OpcodeResolutionError> {
        self.blackbox_vendor.encrypt(inputs.to_vec(), domain_separator).map_err(|err| {
            OpcodeResolutionError::BlackBoxFunctionFailed(BlackBoxFunc::Pedersen, err.to_string())
        })
    }

    fn fixed_base_scalar_mul(
        &self,
        input: &FieldElement,
    ) -> Result<(FieldElement, FieldElement), OpcodeResolutionError> {
        self.blackbox_vendor.fixed_base(input).map_err(|err| {
            OpcodeResolutionError::BlackBoxFunctionFailed(
                BlackBoxFunc::FixedBaseScalarMul,
                err.to_string(),
            )
        })
    }
}

/// Executes an ACIR circuit to generate the solved witness from the initial witness.
///
/// @param {Uint8Array} circuit - A serialized representation of an ACIR circuit
/// @param {WitnessMap} initial_witness - The initial witness map defining all of the inputs to `circuit`..
/// @param {ForeignCallHandler} foreign_call_handler - A callback to process any foreign calls from the circuit.
/// @returns {WitnessMap} The solved witness calculated by executing the circuit on the provided inputs.
#[wasm_bindgen(js_name = executeCircuit, skip_jsdoc)]
pub async fn execute_circuit(
    circuit: Vec<u8>,
    initial_witness: JsWitnessMap,
    _foreign_call_handler: ForeignCallHandler,
) -> Result<JsWitnessMap, JsValue> {
    console_error_panic_hook::set_once();
    let circuit: Circuit = Circuit::read(&*circuit).expect("Failed to deserialize circuit");

    let backend = SimulatedBackend::initialize().await;
    let mut acvm = ACVM::new(backend, circuit.opcodes, initial_witness.into());

    loop {
        let solver_status = acvm.solve().map_err(|err| err.to_string())?;

        match solver_status {
            PartialWitnessGeneratorStatus::Solved => break,
            PartialWitnessGeneratorStatus::RequiresForeignCall { .. } => {
                // TODO: add handling for `Brillig` opcodes.
            }
        }
    }

    let witness_map = acvm.finalize();
    Ok(witness_map.into())
}
