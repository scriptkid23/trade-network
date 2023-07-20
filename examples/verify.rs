use std::{fs::File, io::BufReader};

use bellman::groth16::{prepare_verifying_key, verify_proof, Proof, VerifyingKey};
use bls12_381::Bls12;

fn main() {
    let vk_file = File::open("vk.bin").expect("vk_file");
    let mut vk_reader = BufReader::new(vk_file);
    let vk_receiver: VerifyingKey<Bls12> = VerifyingKey::read(&mut vk_reader).expect("123");

    let pvk = prepare_verifying_key(&vk_receiver);

    let file = File::open("proof.bin").expect("Failed to open file");

    let mut reader = std::io::BufReader::new(file);
    let proof_receiver: Proof<Bls12> = Proof::read(&mut reader).expect("Failed to read proof");

    let is_valid = verify_proof(&pvk, &proof_receiver, &[]);

    println!("Proof is valid: {:?}", is_valid);
}
