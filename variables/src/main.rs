fn main() {
    assign();
    shadow();
    operations();
    tuple();
    array();
}

fn assign() {
    let x = 5;
    println!("The value of x is: {x}");
    // x = 6;
    // println!("The value of x is: {x}");
}

fn shadow() {
    let x = 5;

    let x = x + 1;

    {
        let x = x * 2;
        println!("The value of x in the inner scope is: {x}");
    }

    println!("The value of x is: {x}");
}

fn operations() {
    // addition
    let sum = 5 + 10;
    println!("The value of x is: {sum}");

    // subtraction
    let difference = 95.5 - 4.3;
    println!("The value of difference is: {difference}");

    // multiplication
    let product = 4 * 30;
    println!("The value of product is: {product}");

    // division
    let quotient = 56.7 / 32.2;
    println!("The value of quotient is: {quotient}");
    let truncated = -5 / 3; // Results in -1
    println!("The value of truncated is: {truncated}");

    // remainder
    let remainder = 43 % 5;
    println!("The value of remainder is: {remainder}");
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
