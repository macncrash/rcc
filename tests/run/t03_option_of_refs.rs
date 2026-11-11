struct Entry {
    key: &'static str,
    val: i32,
}

fn find<'a>(table: &'a [Entry], key: &str) -> Option<&'a Entry> {
    for i in 0..table.len() {
        if table[i].key == key {
            return Some(&table[i]);
        }
    }
    None
}

fn find_mut<'a>(table: &'a mut [Entry], key: &str) -> Option<&'a mut Entry> {
    for e in table.iter_mut() {
        if e.key == key {
            return Some(e);
        }
    }
    None
}

fn get_val(e: &Entry) -> i32 {
    e.val
}

fn lookup_sum(table: &[Entry], a: &str, b: &str) -> Option<i32> {
    let x = find(table, a)?.val;
    let y = find(table, b)?.val;
    Some(x + y)
}

fn main() {
    let mut table = [
        Entry { key: "add", val: 1 },
        Entry { key: "sub", val: 2 },
        Entry { key: "mul", val: 3 },
        Entry { key: "div", val: 4 },
    ];
    let keys = ["mul", "add", "mod", "div"];
    for i in 0..keys.len() {
        match find(&table, keys[i]) {
            Some(e) => println!("{} -> {}", e.key, e.val),
            None => println!("{} missing", keys[i]),
        }
        println!("  mapped {}", find(&table, keys[i]).map(get_val).unwrap_or(-1));
    }
    if let Some(e) = find_mut(&mut table, "sub") {
        e.val *= 50;
    }
    match find_mut(&mut table, "nope") {
        Some(_) => println!("?"),
        None => println!("nope missing"),
    }
    println!("sub now {}", find(&table, "sub").unwrap().val);
    println!("{}", lookup_sum(&table, "add", "sub").unwrap_or(0));
    println!("{}", lookup_sum(&table, "add", "xyz").is_none());
    let o: Option<&Entry> = table.get(2);
    println!("{}", o.is_some() && o.unwrap().key == "mul");
    let none: Option<&Entry> = table.get(9);
    println!("{}", none.is_none());
}
