use vec::MyVec;

fn main() {
    let mut v: MyVec<i32> = MyVec::new();

    v.push(10);
    v.push(20);
    v.push(30);
    v.push(40);
    v.push(50);

    println!("len={} cap={}", v.len(), v.cap());

    println!("v[0] = {}", v[0]);
    println!("v[1] = {}", v[1]);
    println!("v[2] = {}", v[2]);

    println!("v.get(0) = {:?}", v.get(0));
    println!("v.get(9) = {:?}", v.get(9));

    println!("pop -> {:?}", v.pop());
    println!("pop -> {:?}", v.pop());
    println!("pop -> {:?}", v.pop());
    println!("pop -> {:?}", v.pop());
    println!("pop -> {:?}", v.pop());
    println!("pop -> {:?}", v.pop());
}
