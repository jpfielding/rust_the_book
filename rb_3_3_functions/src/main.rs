fn main() {
    let y = {
        let x = 3;
        x + 1
    };
    println!("The value of y is: {y}");
    another_function(5);
    print_labeled_measurement(5, 'h');
    sum_str_vec(vec!["1", "2", "3"]);
}
fn print_labeled_measurement(value: i32, unit_label: char) {
    println!("The measurement is: {value}{unit_label}");
}
fn another_function(x: i32) {
    println!("The value of x is: {x}");
}
fn sum_str_vec(strs: Vec<&str>) {
    let mut acc1 = 0;
    let mut acc2 = 0;
    let mut acc3 = 0;
    for s in &strs {
        // Using match to handle potential parsing errors
        acc1 += match s.parse::<i32>() {
            Ok(n) => n,
            Err(_) => 0,
        };
        // Using unwrap_or to provide a default value in case of parsing errors
        acc2 += s.parse::<i32>().unwrap_or(0);
        // Using if let to handle parsing results
        if let Ok(n) = s.parse::<i32>() {
            acc3 += n;
        }
    }
    println!("The sum of the vector loop is: {}", acc1);
    println!("The sum of the vector unwrap_or is: {}", acc2);
    println!("The sum of the vector if let is: {}", acc3);
    // fluent style using iterators and combinators
    let acc4 = strs
        .iter()
        .map(|s| s.parse::<i32>().unwrap_or(0))
        .sum::<i32>()
        .to_string();
    println!("The sum of the vector fluent style is: {}", acc4);
}
