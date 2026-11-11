use std::cell::{Ref, RefCell, RefMut};

#[derive(Debug)]
struct State {
    names: Vec<String>,
    counter: u32,
}

fn first_name(s: &RefCell<State>) -> Ref<String> {
    Ref::map(s.borrow(), |st| &st.names[0])
}

fn counter_mut(s: &RefCell<State>) -> RefMut<u32> {
    RefMut::map(s.borrow_mut(), |st| &mut st.counter)
}

fn main() {
    let s = RefCell::new(State { names: vec!["alpha".into(), "beta".into()], counter: 0 });
    {
        let f = first_name(&s);
        println!("first={} len={}", *f, f.len());
        println!("borrow_mut while Ref alive err={}", s.try_borrow_mut().is_err());
        let g = Ref::clone(&f);
        println!("cloned ref={}", g);
    }
    println!("borrow_mut after drop err={}", s.try_borrow_mut().is_err());
    for _ in 0..3 {
        *counter_mut(&s) += 5;
    }
    println!("counter={}", s.borrow().counter);
    {
        let mut c = counter_mut(&s);
        *c *= 2;
        println!("shared borrow while RefMut err={}", s.try_borrow().is_err());
    }
    let st = s.borrow();
    let (a, b) = Ref::map_split(st, |st| (&st.names[0], &st.names[1]));
    println!("split {} {}", a, b);
    drop(a);
    println!("still borrowed via b err={}", s.try_borrow_mut().is_err());
    drop(b);
    s.borrow_mut().names.push("gamma".into());
    println!("{:?}", s.borrow());
    let total: usize = s.borrow().names.iter().map(|n| n.len()).sum();
    println!("total len={}", total);
    let taken = s.take_names();
    println!("taken={:?} after={:?}", taken, s.borrow().names);
}

trait TakeNames {
    fn take_names(&self) -> Vec<String>;
}
impl TakeNames for RefCell<State> {
    fn take_names(&self) -> Vec<String> {
        std::mem::take(&mut self.borrow_mut().names)
    }
}
