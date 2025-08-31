// rust the book: 4.1 Ownership
// https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html

fn main() {
    println!("Hello, world!");
    scope_assignment();
    ownership_transfer();
    drop_example();
    clone_example();
    stack_only_data();
    ownership_and_functions();
    return_values_scope();
    move_len_calc();
}


fn scope_assignment() {
    let x = 5; // x comes into scope
    let y = x; // x is copied into y (i32 implements the Copy trait)
    println!("x: {}, y: {}", x, y); // both x and y can be used
}

fn ownership_transfer() {
    let s1 = String::from("hello"); // s1 comes into scope
    let s2 = s1; // ownership of the String is moved to s2
    // println!("{}", s1); // this would cause a compile-time error because s1 is no longer valid
    println!("{}", s2); // s2 is valid and can be used
}

fn drop_example() {
    let mut s = String::from("hello");
    s = String::from("ahoy");

    println!("{s}, world!");
}

fn clone_example() {
    let s1 = String::from("hello");
    let s2 = s1.clone(); // deep copy of the data on the heap

    println!("s1: {}, s2: {}", s1, s2);
}

fn stack_only_data() {
    let x = 5; // x comes into scope
    let y = x; // x is copied into y (i32 implements the Copy trait)
    println!("x: {}, y: {}", x, y); // both x and y can be used
}

fn ownership_and_functions() {
    let s = String::from("hello"); // s comes into scope
    takes_ownership(s.clone()); // s's value moves into the function and is no longer valid here
    println!("{}", s); // this would cause a compile-time error

    let t = String::from("world"); // s comes into scope
    takes_ownership(t); // s's value moves into the function and is no longer valid here
    // println!("{}", t); // this would cause a compile-time error

    let x = 5; // x comes into scope
    makes_copy(x); // x is copied into the function, so it's still valid here
    println!("{}", x); // this works fine
}

fn takes_ownership(some_string: String) {
    println!("{}", some_string);
} // some_string goes out of scope and is dropped here

fn makes_copy(some_integer: i32) {
    println!("{}", some_integer);
} // some_integer goes out of scope here, but nothing special happens

fn return_values_scope() {
    let _s1 = gives_ownership(); // gives_ownership moves its return value into s1
    let s2 = String::from("hello"); // s2 comes into scope
    let _s3 = takes_and_gives_back(s2); // s2 is moved into takes_and_gives_back, which also moves its return value into s3
} // s3 goes out of scope and is dropped. s2 was moved, so nothing happens. s1 goes out of scope and is dropped.

fn gives_ownership() -> String {
    let some_string = String::from("hello"); // some_string comes into scope
    some_string // some_string is returned and moves out to the calling function
}

fn takes_and_gives_back(some_string: String) -> String {
    println!("{}", some_string);
    String::from("goodbye")
}

fn move_len_calc() {
    let s1 = String::from("hello");

    let (s2, len) = calculate_length(s1);

    println!("The length of '{s2}' is {len}.");
}

fn calculate_length(s: String) -> (String, usize) {
    let length = s.len(); // len() returns the length of a String

    (s, length)
}