fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn main() {
    let unused_var = 5;
    let result: i32 = add(1, 2);
    println!("{}", result);
}
