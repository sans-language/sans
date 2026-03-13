enum Shape {
    Circle(Int),
    Rectangle(Int, Int),
}

fn main() Int {
    let s = Shape::Rectangle(3, 4)
    match s {
        Shape::Circle(r) => r,
        Shape::Rectangle(w, h) => w * h,
    }
}
