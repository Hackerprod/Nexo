#[allow(dead_code)]
mod r#gen;

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    let seed: u64 = arguments[1].parse().expect("seed");
    let count: u64 = arguments[2].parse().expect("count");
    let rd = r#gen::rd_sources();
    for index in 0..count {
        let (_, source) = r#gen::gen_program(seed, index, &rd);
        println!("{}", source.replace('\n', " "));
    }
}
