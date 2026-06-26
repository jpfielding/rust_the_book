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
}
