#[macro_use]
extern crate derive_wrapper;
use std::convert::AsRef;

#[derive(AsRef, Default, Display, Debug)]
#[display_from(Debug)]
struct Me(u8);

#[derive(AsRef, Default, LowerHexIter, Display)]
#[display_from(LowerHex)]
struct One {
    a: [u8; 32],
}

#[derive(AsRef, Index, LowerHexIter)]
#[wrap = "b"]
struct You {
    a: (),
    //    #[wrap]
    b: [u8; 16],
}

#[derive(Debug, AsRef, Default, LowerHex, Display)]
#[display_from(LowerHex)]
struct Other {
    a: (),
    #[wrap]
    b: u8,
}

//#[derive(AsRef)]
//struct Fail1 {
//    a: (),
//    b: u8
//}
//
//#[derive(AsRef)]
//struct Fail2;

fn test_lowerhex() {
    let a = You {
        a: (),
        b: [5u8; 16],
    };
    println!("0x{:02x}", a);
    let a = Other { a: (), b: 255 };
    println!("0x{:02x}", a);
}

fn test_as_ref() {
    let a = Me::default();
    a.as_ref();
}

fn test_display() {
    let a = Me(175);
    let b = Other { a: (), b: 135 };
    println!("me: {}, Other: 0x{}", a, b);

    let one = One { a: [173; 32] };
    println!("one: 0x{}", one);
}

fn main() {
    test_lowerhex();
    test_as_ref();
    test_display();
}
