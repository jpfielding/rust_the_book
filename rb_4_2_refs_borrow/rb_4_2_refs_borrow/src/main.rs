fn main() {
    refs_borrow();
    mut_ref();
}

fn refs_borrow() {
    let s1 = String::from("hello");
    let len = calculate_length(&s1);
    println!("The length of '{s1}' is {len}.");
}

fn calculate_length(s: &String) -> usize {
    s.len()
}

fn mut_ref() {
    let mut s = String::from("hello"); // mutable reference

    change(&mut s); // mutable reference
}

fn change(some_string: &mut String) {
    some_string.push_str(", world"); // mutable reference
}

