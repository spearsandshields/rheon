//! Developer helper: `cargo run -p flowstone-feel-wasm --example call -- '<request json>' ...`
fn main() {
  for arg in std::env::args().skip(1) {
    println!("{}", feel_wasm::call(&arg));
  }
}
