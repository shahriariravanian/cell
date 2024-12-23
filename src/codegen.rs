use std::fs::File;
use std::io::Write;
use memmap2::MmapOptions;

#[no_mangle]
#[inline(never)]
fn add(x: i32, y: i32) -> i32 {
    x.wrapping_add(y)
}

#[no_mangle]
#[inline(never)]
fn mul(x: f64, y: f64) -> f64 {
    x * y
}

#[no_mangle]
#[inline(never)]
fn fact(mut n: i32, y: i32) -> i32 {
    let mut f: i32 = 1;
    while n > 0 {
        f = f.wrapping_mul(n);
        n = n.wrapping_sub(1);
    };
    f.wrapping_add(y)
}

fn write_code() {
    let q = fact as *mut u8;
    let mut v: Vec<u8> = Vec::new();
    let q: Vec<u8> = vec![0x55, 0x53, 0x5b, 0x5d, 0xc3];
    
    for x in q {
        //v.push(unsafe { *q.offset(i) });
        v.push(x);
    }    
    
    let mut fs = File::create("code.bin").unwrap();
    fs.write(&v).unwrap();
    //println!("{:?}", &v);
}

#[test]
pub fn test_codegen() {
    write_code();
    let fs = File::open("code.bin").unwrap();
    let mmap = unsafe { MmapOptions::new().map_exec(&fs).unwrap() };
    let p = mmap.as_ptr() as *const fn(x: i32, y: i32) -> i32;
    let f: fn(x: i32, y: i32) -> i32 = unsafe { std::mem::transmute(p) };
    let x = f(6, 3);
    println!("{}",  x);
}

