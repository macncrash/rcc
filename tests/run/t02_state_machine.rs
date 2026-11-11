enum State {
    Start,
    InNumber { value: u64, digits: u32 },
    InWord { len: u32, upper: u32 },
    Done,
}

fn step(s: State, c: u8, out: &mut [u64; 3]) -> State {
    match (s, c) {
        (State::Done, _) => State::Done,
        (_, b'.') => State::Done,
        (State::Start, b'0'..=b'9') => State::InNumber { value: (c - b'0') as u64, digits: 1 },
        (State::Start, b'a'..=b'z') => State::InWord { len: 1, upper: 0 },
        (State::Start, b'A'..=b'Z') => State::InWord { len: 1, upper: 1 },
        (State::Start, _) => State::Start,
        (State::InNumber { value, digits }, b'0'..=b'9') => {
            State::InNumber { value: value * 10 + (c - b'0') as u64, digits: digits + 1 }
        }
        (State::InNumber { value, digits }, _) => {
            println!("number {} ({} digits)", value, digits);
            out[0] += value;
            State::Start
        }
        (State::InWord { len, upper }, b'a'..=b'z') => State::InWord { len: len + 1, upper },
        (State::InWord { len, upper }, b'A'..=b'Z') => State::InWord { len: len + 1, upper: upper + 1 },
        (State::InWord { len, upper }, _) => {
            println!("word len {} upper {}", len, upper);
            out[1] += 1;
            out[2] += len as u64;
            State::Start
        }
    }
}

fn main() {
    let text = "Hello 42 world 7 and HTTP 1000000 x9 end. ignored 55";
    let b = text.as_bytes();
    let mut s = State::Start;
    let mut out = [0u64; 3];
    for i in 0..b.len() {
        s = step(s, b[i], &mut out);
    }
    match s {
        State::Done => println!("done"),
        _ => println!("not done"),
    }
    println!("sum {} words {} letters {}", out[0], out[1], out[2]);
}
