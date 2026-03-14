trait Summable {
    fn sum(self) Int
}

struct Pair {
    a Int,
    b Int,
}

impl Summable for Pair {
    fn sum(self) Int {
        self.a + self.b
    }
}

fn main() Int {
    let p = Pair { a: 5, b: 8 }
    p.sum()
}
