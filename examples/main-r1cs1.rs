use std::{fs::File, io::BufWriter, net::TcpStream};

use bellman::{
    groth16::{
        create_random_proof, generate_random_parameters, prepare_verifying_key, verify_proof, Proof,
    },
    Circuit, ConstraintSystem, SynthesisError,
};
use bls12_381::{Bls12, Scalar};
use rand::thread_rng;

#[derive(Clone, Copy)]
struct MyCircuit {
    x: Option<Scalar>,
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

fn main() {
    let rng = &mut thread_rng();

    let params = {
        let c = MyCircuit { x: None };
        generate_random_parameters::<Bls12, _, _>(c, rng).unwrap()
    };

    let vk_file: File = File::create("vk.bin").expect("Create vk.bin error");
    let mut vk_writer = BufWriter::new(vk_file);

    params.vk.write(&mut vk_writer).expect("Failed to write vk to file");
    let pvk = prepare_verifying_key(&params.vk);

    // witness = 1
    let x_input = Scalar::from(1);

    let circuit = MyCircuit { x: Some(x_input) };

    let file: File = File::create("proof.bin").expect("msg");

    let mut writer = BufWriter::new(file);

    let proof = create_random_proof(circuit, &params, rng).unwrap();

    println!("{:?}", proof);
    proof
        .write(&mut writer)
        .expect("Failed to write proof to file");

    // Assuming you have the `proof` instance already created
    // Open a file for writing
    // let serialized_pvk = serde_json::from_str(&pvk);

    let is_valid = verify_proof(&pvk, &proof, &[]);

    println!("Proof is valid: {:?}", is_valid);
}
