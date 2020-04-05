#![allow(dead_code, bare_trait_objects)]

#[macro_use]
extern crate derive_wrapper;
use std::convert::AsRef;
use std::error::Error;

#[derive(AsRef, Default, Display, Debug)]
#[display_from(Debug)]
struct Me(u8);

#[derive(AsRef, Default, LowerHexIter, Display, From, Error, Debug)]
#[display_from(LowerHex)]
struct One {
    a: [u8; 32],
}

#[derive(AsRef, Default, LowerHexIter, Display, From, Index)]
#[display_from(LowerHex)]
#[index_output(u8)]
struct Heap(Box<[u8]>);

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

fn test_from() {
    let a: One = [55u8; 32].into();
    println!("{}", a);
    assert_eq!(
        a.to_string(),
        "3737373737373737373737373737373737373737373737373737373737373737"
    );
}

fn test_error() {
    let a: One = [173; 32].into();
    let a: Box<Error> = Box::new(a);
    println!("{}", a);
    assert_eq!(
        a.to_string(),
        "adadadadadadadadadadadadadadadadadadadadadadadadadadadadadadadad"
    );
}

fn test_index_heap() {
    let a = vec![1, 2, 3, 4, 5, 6, 7].into_boxed_slice();
    let heap = Heap(a);
    assert_eq!(heap[0], 1);
    assert_eq!(&heap[..2], &[1, 2]);
    assert_eq!(&heap[..], &[1, 2, 3, 4, 5, 6, 7]);
}

fn test_lowerhex() {
    let a = You {
        a: (),
        b: [5u8; 16],
    };
    let fmt = format!("0x{:02x}", a);
    println!("{}", fmt);
    assert_eq!(fmt, "0x05050505050505050505050505050505");
    let a = Other { a: (), b: 255 };
    let fmt = format!("0x{:02x}", a);
    assert_eq!(fmt, "0xff");
}

fn test_as_ref() {
    let a = Me::default();
    a.as_ref();
}

fn test_display() {
    let a = Me(175);
    let b = Other { a: (), b: 135 };
    let fmt = format!("me: {}, Other: 0x{}", a, b);
    println!("{}", fmt);
    assert_eq!(fmt, "me: Me(175), Other: 0x87");

    let one = One { a: [173; 32] };
    let fmt = format!("one: 0x{}", one);
    println!("{}", fmt);
    assert_eq!(
        fmt,
        "one: 0xadadadadadadadadadadadadadadadadadadadadadadadadadadadadadadadad"
    )
}

fn test_readme() {
    #[derive(Debug, Default, Index, AsRef, LowerHexIter)]
    struct Array32([u8; 32]);

    #[derive(Debug, Default, LowerHex)]
    struct Flag(i32);

    #[derive(Debug, Index, LowerHexIter, Default)]
    struct Hi {
        #[wrap]
        a: [u8; 32],
        b: Flag,
    }

    #[derive(Debug, Display, From, Error)]
    #[display_from(Debug)]
    struct Printer<T: std::fmt::Debug>(T);

    #[derive(Default, LowerHex, Display, Debug)]
    #[display_from(LowerHex)]
    #[wrap = "two"]
    struct Big {
        one: Array32,
        two: Hi,
    }
}

fn main() {
    test_readme();
    test_lowerhex();
    test_as_ref();
    test_display();
    test_from();
    test_index_heap();
    test_error();
}

#[cfg(not(MSRV))]
#[derive(AsRef, Index)]
#[wrap = 0]
struct Tupling(Vec<u8>, usize);

#[derive(AsRef, Index)]
#[wrap = "0"]
struct Tuplinn(Vec<u8>, usize);
