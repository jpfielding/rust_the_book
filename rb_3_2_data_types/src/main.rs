use std::io::{self, Write};

fn main() {
    tuple();
    array();
    array_overrun();
    parameters();
}


fn tuple() {
    let tup = (500, 6.4, 1);

    let (x, y, z) = tup;

    println!("The value of x is: {x}");
    println!("The value of y is: {y}");
    println!("The value of z is: {z}");
}
fn array() {
    let a = [1, 2, 3, 4, 5];
    println!("The value of a is: {:?}", a);

    let months = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    println!("The value of months is: {:?}", months);

    let a2: [i32; 5] = [1, 2, 3, 4, 5];
    let _first = a2[0];
    let _second = a2[1];
}

fn array_overrun() {
    let a = [1, 2, 3, 4, 5];

    loop {
        print!("Please enter a valid array index: ");
        io::stdout().flush().expect("failed to wipe before flush?");
        
        let mut index = String::new();

        io::stdin()
            .read_line(&mut index)
            .expect("Failed to read line");

        let index: usize = match index.trim().parse(){
                Ok(num) => num,
                Err(_) => continue,
            };

        let element = a[index];
    
        println!("The value of the element at index {index} is: {element}");
        break;
    }
}
fn parameters() {
    let print_labeled_measurement = |value: i32, unit_label: char| {
        println!("The measurement is: {value}{unit_label}");
    };
    print_labeled_measurement(5, 'h');
}

