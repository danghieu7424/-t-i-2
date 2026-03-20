// src/utils/mod.rs
pub mod suid;
// pub mod license;
// pub mod token;
// pub mod email;
pub use self::suid::suid;
pub use self::suid::generate_random_hex;
// pub use self::email::{ send_order_shipping_email, send_order_thank_you_email};