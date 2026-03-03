use std::sync::Arc;
use vec::append_vec::AppendVec;

fn main() {
    // basic append and get
    let mut v: AppendVec<i32> = AppendVec::with_capacity(8);
    v.append(10);
    v.append(20);
    v.append(30);
    println!("after append: len={} cap={}", v.len(), v.cap());
    println!(
        "get(0)={:?} get(1)={:?} get(2)={:?}",
        v.get(0),
        v.get(1),
        v.get(2)
    );

    // out of bounds returns None
    println!("get(99)={:?}", v.get(99));

    // zst - cap() always 0, append is unbounded
    let mut z: AppendVec<()> = AppendVec::with_capacity(4);
    z.append(());
    z.append(());
    z.append(());
    println!("zst: len={} cap={} get(0)={:?}", z.len(), z.cap(), z.get(0));

    // concurrent reads - write first, then share via Arc across threads
    let mut log: AppendVec<i32> = AppendVec::with_capacity(100);
    for i in 0..100 {
        log.append(i);
    }

    let log = Arc::new(log);
    let mut handles = vec![];

    for t in 0..4 {
        let log = Arc::clone(&log);
        handles.push(std::thread::spawn(move || {
            let len = log.len();
            println!(
                "thread {t}: len={len} first={:?} last={:?}",
                log.get(0),
                log.get(len - 1)
            );
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}
