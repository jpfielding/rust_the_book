#![allow(unused)]
type CheckNumber = u32;
type CardNumber = String;

enum CardType {
    Visa,
    MasterCard,
    AmericanExpress,
    Discover,
}

#[derive(Debug)]
struct CreditInfo(CheckNumber, CardNumber);

#[derive(Debug)]
enum PaymentMethod {
    Cash,
    Check(CheckNumber),
    Card(CreditInfo),
}

type PaymentAmount = f32;
#[derive(Debug)]
enum Currency {
    Eur,
    Usd,
}

#[derive(Debug)]
struct Payment {
    amount: PaymentAmount,
    currency: Currency, 
    method: PaymentMethod,
}

trait PrintDetails {
    fn payment_info(&self) -> String;
}

impl PrintDetails for Payment {
    fn payment_info(&self) -> String {
        let method = match &self.method {
            PaymentMethod::Cash => String::from("cash"),
            PaymentMethod::Check(c) => format!("a check with a number {:?}",c),
            PaymentMethod::Card(c) => format!("a check {} with a credit card {}", c.0, c.1),
        };
        format!(
            "An Amount of {}, was paid in {:?} using {}", &self.amount, &self.currency, method
        )
    }
}


fn main() {
    let cc_payment = Payment {
        amount: 100.33,
        currency: Currency::Usd,
        method: PaymentMethod::Card(CreditInfo(122, String::from("452389798712097"))),
    };

    let check_payment = Payment {
        amount: 100.33,
        currency: Currency::Usd,
        method: PaymentMethod::Check(42),
    };

    println!("Payment details: {}\n", check_payment.payment_info());
    println!("Payment details: {}\n", cc_payment.payment_info());
}
