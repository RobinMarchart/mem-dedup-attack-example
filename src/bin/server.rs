use std::{
    io::Read,
    os,
};

use test_program::{alloc_single_page, fill_page, PAGESIZE};

pub fn main() {
    let mut page = alloc_single_page();
    let path=std::env::args_os().nth(1).expect("Expected 1 argument");
    std::fs::remove_file(&path);
    let listener = os::unix::net::UnixListener::bind(&path).expect("unable to bind socket");
    let mut random = std::fs::File::open("/dev/urandom").expect("unable to open urandom device");
    println!("Waiting for connection");
    let (mut socket, _) = listener
        .accept()
        .expect("Unable to accept socket connection");
    println!("Connected");
    fill_page(&mut page);
    let mut rcv_buf = [0_u8];
    loop {
        socket
            .read_exact(&mut rcv_buf)
            .expect("Unable to properly read from socket");
        random
            .read_exact(&mut rcv_buf)
            .expect("Unable to read random byte");
        println!("{}", rcv_buf[0]);
        page.copy_within(1..(*PAGESIZE), 0);
        page[(*PAGESIZE) - 1] = rcv_buf[0];
    }
}
