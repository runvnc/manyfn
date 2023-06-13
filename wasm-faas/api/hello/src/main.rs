
fn main() {
    let name = std::env::var("name").unwrap();
    println!("__ Hello {}!", name);
}
