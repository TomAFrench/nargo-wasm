use acvm::acir::brillig_vm::Value;

use crate::js_transforms::field_element_to_js_string;

pub(super) fn encode_foreign_call_inputs(foreign_call_inputs: &[Vec<Value>]) -> js_sys::Array {
    let inputs = js_sys::Array::default();
    for input in foreign_call_inputs {
        let input_array = js_sys::Array::default();
        for value in input {
            let hex_js_string = field_element_to_js_string(&value.to_field());
            input_array.push(&hex_js_string);
        }
        inputs.push(&input_array);
    }

    inputs
}
