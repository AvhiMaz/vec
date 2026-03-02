use vec::MyVec;

fn main() {
    let mut v: MyVec<i32> = MyVec::new();

    // push
    v.push(10);
    v.push(20);
    v.push(30);
    v.push(40);
    v.push(50);
    println!("after push: len={} cap={}", v.len(), v.cap());

    // index
    println!("v[0]={} v[1]={} v[2]={}", v[0], v[1], v[2]);

    // get
    println!("get(2)={:?} get(99)={:?}", v.get(2), v.get(99));

    // insert
    v.insert(1, 99);
    println!("after insert(1, 99): len={}", v.len());
    println!("v[0]={} v[1]={} v[2]={}", v[0], v[1], v[2]);

    // remove
    let removed = v.remove(1);
    println!("remove(1)={} len={}", removed, v.len());

    // pop
    println!("pop={:?}", v.pop());
    println!("pop={:?}", v.pop());
    println!("pop={:?}", v.pop());
    println!("pop={:?}", v.pop());
    println!("pop={:?}", v.pop());
    println!("pop on empty={:?}", v.pop());

    // zst
    let mut z: MyVec<()> = MyVec::new();
    z.push(());
    z.push(());
    z.push(());
    println!("zst len={} pop={:?}", z.len(), z.pop());
}
