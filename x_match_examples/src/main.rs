use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about = "A simple CLI example")]
struct Cli {
    /// number to examine
    #[arg(short, long, default_value_t = 13)]
    number: u8,
}

fn main() {
    let args = Cli::parse();

    let number = args.number;

    println!("Tell me aobut {}", number);
    match number {
        // match a single value
        1 => println!("One!"),
        // match several values
        2 | 3 | 5 | 7 | 11 => println!("this is a prime"),
        // divisible by 2
        n if n % 2 == 0 => println!("an even number"),
        // divisible by 3
        x if x % 3 == 0 => println!("a multiple of three"),

        // TODO ^ try adding 13 to the list of prime values
        // match an inclusive range
        13..=19 => println!("a teen"),
        // handle the rest of cases
        _ => println!("aint special"),
        // todo ^ try commenting out this catch all arm
    }

    let boolean = true;
    // match is an expression too
    let binary = match boolean {
        // the arms of a match must cover all possible values
        false => 0,
        _ => 1, //  not false
    };
    println!("{} -> {}", boolean, binary);

    // guarded match arms
    #[allow(dead_code)]
    enum Temperature {
        Celsius(u8),
        Fahrenheit(u8),
    }
    let temperature = Temperature::Celsius(number);
    match temperature {
        Temperature::Celsius(t) if t > 30 => println!("{}C is above 30 Celsius", t),
        // The `if condition` part ^ is a guard
        Temperature::Celsius(t) => println!("{}C is equal to or below 30 Celsius", t),

        Temperature::Fahrenheit(t) if t > 86 => println!("{}F is above 86 Fahrenheit", t),
        Temperature::Fahrenheit(t) => println!("{}F is equal to or below 86 Fahrenheit", t),
    }

    // Destructuring
    let triple = (0, -2, 3);
    // TODO ^ Try different values for `triple`
    println!("Tell me about {:?}", triple);
    // Match can be used to destructure a tuple
    match triple {
        // Destructure the second and third elements
        (0, y, z) => println!("First is `0`, `y` is {:?}, and `z` is {:?}", y, z),
        (1, ..) => println!("First is `1` and the rest doesn't matter"),
        (.., 2) => println!("last is `2` and the rest doesn't matter"),
        (3, .., 4) => println!("First is `3`, last is `4`, and the rest doesn't matter"),
        // `..` can be used to ignore the rest of the tuple
        _ => println!("It doesn't matter what they are"),
        // `_` means don't bind the value to a variable
    }

    // binding n @ pattern gives us a match but also a variable we can use in the arm
    match Some(number) {
        Some(n @ 1..=12) => println!("a number between 1 and 12: {}", n),
        Some(n @ 42) => println!("The Answer: {}!", n),
        // Match any other number.
        Some(n) => println!("Not interesting... {}", n),
        // Match anything else (`None` variant).
        _ => (),
    }
}
