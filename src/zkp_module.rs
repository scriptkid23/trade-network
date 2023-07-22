use std::io::BufWriter;

use bellman::{
    groth16::{
        create_random_proof, generate_random_parameters, prepare_verifying_key, verify_proof,
        Proof, VerifyingKey,
    },
    Circuit, ConstraintSystem, SynthesisError, VerificationError,
};
use bls12_381::{Bls12, Scalar};
use rand::thread_rng;

#[derive(Clone, Copy)]
struct MyCircuit {
    x: Option<Scalar>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Output {
    pub verify_key: String,
    pub proof: String,
}

pub fn generate_proof(x: u64) -> Output {
    let rng = &mut thread_rng();

    let params = {
        let c = MyCircuit { x: None };
        generate_random_parameters::<Bls12, _, _>(c, rng).unwrap()
    };

    let vk_vec: Vec<u8> = vec![];

    let mut vk_writer = BufWriter::new(vk_vec);

    params
        .vk
        .write(&mut vk_writer)
        .expect("Failed to write vk to file");

    let circuit = MyCircuit {
        x: Some(Scalar::from(x)),
    };

    let proof_vec: Vec<u8> = vec![];
    let mut proof_writer = BufWriter::new(proof_vec);
    let proof = create_random_proof(circuit, &params, rng).unwrap();

    proof
        .write(&mut proof_writer)
        .expect("Failed to write proof");

    return Output {
        verify_key: serde_json::to_string(vk_writer.buffer()).expect("Failed to string"),
        proof: serde_json::to_string(proof_writer.buffer()).expect("Failed to string"),
    };
}

pub fn verify(verify_key: &[u8], proof: &[u8]) -> Result<(), VerificationError> {
    let vk_receiver = VerifyingKey::read(verify_key).expect("Failed to read verify key");
    let proof_recevier: Proof<Bls12> = Proof::read(proof).expect("Failed to read buffer");

    let pvk = prepare_verifying_key(&vk_receiver);

    return verify_proof(&pvk, &proof_recevier, &[]);
}

impl Circuit<bls12_381::Scalar> for MyCircuit {
    fn synthesize<CS: ConstraintSystem<Scalar>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
        // Định nghĩa các ràng buộc và quan hệ trong mạch
        let x = cs.alloc(|| "x", || self.x.ok_or(SynthesisError::AssignmentMissing))?;

        let x_squared = cs.alloc(
            || "x_squared",
            || {
                let x_val = self.x.ok_or(SynthesisError::AssignmentMissing)?;
                Ok(x_val * x_val)
            },
        )?;

        let x_three = cs.alloc(
            || "3x",
            || {
                let x_val = self.x.ok_or(SynthesisError::AssignmentMissing)?;
                Ok(Scalar::from(3) * x_val)
            },
        )?;

        let three = cs.alloc(|| "four", || Ok(Scalar::from(3)))?;

        cs.enforce(
            || "x_squared constraint",
            |lc| lc + x,
            |lc| lc + x,
            |lc| lc + x_squared,
        );

        cs.enforce(
            || "three_x constraint",
            |lc| lc + three,
            |lc| lc + x,
            |lc| lc + x_three,
        );

        let result = cs.alloc(
            || "result",
            || {
                let x_val = self.x.ok_or(SynthesisError::AssignmentMissing)?;
                Ok(x_val * x_val + Scalar::from(3) * x_val - Scalar::from(4))
            },
        )?;

        cs.enforce(
            || "result contraint",
            |lc| lc + result,
            |lc| lc + CS::one(),
            |lc| lc,
        );

        //

        Ok(())
    }
}
