import std.io;

struct Point { x: f64, y: f64 }

impl Point {
    pub fn dist(self, o: Point) -> f64 {
        let dx = self.x - o.x;
        let dy = self.y - o.y;
        (dx * dx + dy * dy).sqrt()
    }
}

enum Shape { Circle(f64), Rect(f64, f64) }

fn area(s: Shape) -> f64 {
    match s {
        Shape::Circle(r) => 3.14159 * r * r,
        Shape::Rect(w, h) => w * h,
    }
}

fn parse(s: str) -> Result<i64, str> {
    let n = s.to_int()?;
    if n < 0 { Err("negativo") } else { Ok(n) }
}

fn main() {
    let mut total = 0.0;
    for i in 0..10 { total += i as f64; }
    io.println("total = {total}");
}
