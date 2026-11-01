use fhec_backend::FheBackend;
use fhec_backend_tfhe::TfheBackend;

#[test]
fn actual_tfhe_mux_selects_encrypted_branches_for_all_boolean_inputs() {
    let backend = TfheBackend::new();

    for bits in 0u8..8 {
        let condition = (bits & 1) != 0;
        let if_true = (bits & 2) != 0;
        let if_false = (bits & 4) != 0;
        let encrypted_condition = backend.encrypt_bool(condition).unwrap();
        let encrypted_true = backend.encrypt_bool(if_true).unwrap();
        let encrypted_false = backend.encrypt_bool(if_false).unwrap();
        let encrypted_result = backend
            .mux(&encrypted_condition, &encrypted_true, &encrypted_false)
            .unwrap();
        let decrypted = backend.decrypt_bool(&encrypted_result).unwrap();
        assert_eq!(decrypted, if condition { if_true } else { if_false });
    }
}
