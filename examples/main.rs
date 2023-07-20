use std::ops::Add;

use bellman::{
    groth16::{
        create_random_proof, generate_random_parameters, prepare_verifying_key, verify_proof,
    },
    Circuit, ConstraintSystem, SynthesisError,
};
use bls12_381::{Bls12, Scalar};
use rand::thread_rng;

#[derive(Clone, Copy)]
struct MyCircuit {
    x: Option<Scalar>,
    y: Option<Scalar>,
}

impl Circuit<bls12_381::Scalar> for MyCircuit {
    fn synthesize<CS: ConstraintSystem<Scalar>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
        // Định nghĩa các ràng buộc và quan hệ trong mạch
        let x = cs.alloc_input(|| "x", || self.x.ok_or(SynthesisError::AssignmentMissing))?;
        let y = cs.alloc(|| "y", || self.y.ok_or(SynthesisError::AssignmentMissing))?;

        //
        cs.enforce(|| "x = y", |lc| lc  + x - y, |lc| lc + CS::one(), |lc| lc);

        Ok(())
    }
}

fn main() {
    let rng = &mut thread_rng();

    let x_input = Scalar::from(2);
    let y_input = Scalar::from(2);

    let circuit = MyCircuit {
        x: Some(x_input),
        y: Some(y_input),
    };

    let params = generate_random_parameters::<Bls12, _, _>(circuit, rng).unwrap();

    let pvk = prepare_verifying_key(&params.vk);

    let proof = create_random_proof(circuit, &params, rng).unwrap();

    // Chuẩn bị khóa xác thực và xác minh chứng minh

    let is_valid = verify_proof(&pvk, &proof, &[x_input]);

    println!("Proof is valid: {:?}", is_valid);
}
