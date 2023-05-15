#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::{sync::Arc};

#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Role {
    Admin,
    Standard,
    #[default]
    Guest,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct DB {}

#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct User {
    id: u32, 
    name: String, 
    role: Role,
    #[cfg_attr(feature = "serde", serde(skip))]
    db: Arc<DB>,
}

// Top 5 traits to implement in lbis
// https://youtu.be/Nzclc6MswaI
// cargo run --features serde
fn main() {
    let user = User {
        id: 123, 
        name: "bogdan".to_owned(),
        role:  Role::Admin,
        db: Arc::new(DB {}),
    };
    
    println!("{:?}", user);

    let user2 = user.clone();
    println!("{:?}", user);

    let guest = User::default(); // 0, "", Guest
    let guest2 = User::default();

    assert_eq!(guest, guest2);

    let user_str = "{ \"id\": 123, \"name\": \"bogdan\", \"role\": \"Admin\" }";

    #[cfg(feature = "serde")]
    let user: User = serde_json::from_str(&user_str).unwrap();

    #[cfg(feature = "serde")]
    println!("{:?}", user);

}

// rust for rustaceans
fn is_normal<T: Sized + Send + Sync + Unpin>() {}

#[test]
fn normal_types() {
    is_normal::<User>();
}