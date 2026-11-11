// Shadowing keeps the old value alive until scope end; assignment drops the old value now.
struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Wrap {
    d: D,
}

fn main() {
    println!("-- shadowing");
    {
        let x = D("x1");
        println!("x is {}", x.0);
        let x = D("x2");
        println!("x is {}", x.0);
        let x = D("x3");
        println!("x is {}", x.0);
        println!("end of shadow scope");
    }
    println!("-- assignment");
    {
        let mut y = D("y1");
        println!("y is {}", y.0);
        y = D("y2");
        println!("y is {}", y.0);
        y = D("y3");
        println!("y is {} end of scope", y.0);
    }
    println!("-- assignment through &mut");
    {
        let mut z = D("z1");
        let r = &mut z;
        *r = D("z2");
        println!("z is {}", z.0);
    }
    println!("-- field assignment");
    {
        let mut w = Wrap { d: D("field1") };
        w.d = D("field2");
        println!("w.d is {}", w.d.0);
    }
    println!("-- Option reassignment");
    {
        let mut o: Option<D> = None;
        println!("starts none {}", o.is_none());
        println!("assign Some(o1)");
        o = Some(D("o1"));
        println!("is_some {}", o.is_some());
        println!("assign Some(o2)");
        o = Some(D("o2"));
        println!("holds {}", o.as_ref().unwrap().0);
        println!("assign None");
        o = None;
        println!("o is_none {}", o.is_none());
        o = Some(D("o3"));
        println!("end with {}", o.as_ref().unwrap().0);
    }
    println!("-- late initialisation");
    {
        let late: D;
        println!("declared");
        late = D("late");
        println!("initialised {}", late.0);
    }
    println!("-- array element assignment");
    {
        let mut arr = [D("a0"), D("a1")];
        arr[1] = D("a1-new");
        println!("arr {} {}", arr[0].0, arr[1].0);
    }
    println!("end of main");
}
