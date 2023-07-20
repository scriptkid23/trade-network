use bellman::{domain::Scalar, ConstraintSystem};
fn main() {
    // Khởi tạo một mạch R1CS
    let m: u64 = 30;
    let x = bls12_381::Scalar::from(m);

    print!("{x:?}");
}
