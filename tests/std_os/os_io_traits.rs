// Read/Write/BufRead/Seek on Cursor, slices, Vec, BufReader/BufWriter/LineWriter, adapters, io::copy
use std::io::{self, BufRead, BufReader, BufWriter, Cursor, LineWriter, Read, Seek, SeekFrom, Write};

struct Trickle<'a> {
    data: &'a [u8],
    reads: usize,
}
impl<'a> Read for Trickle<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reads += 1;
        let n = buf.len().min(3).min(self.data.len());
        buf[..n].copy_from_slice(&self.data[..n]);
        self.data = &self.data[n..];
        Ok(n)
    }
}

struct CountingWriter {
    writes: usize,
    data: Vec<u8>,
}
impl Write for CountingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writes += 1;
        self.data.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn generic_sum<R: BufRead>(r: R) -> i64 {
    r.lines().map(|l| l.unwrap().trim().parse::<i64>().unwrap_or(0)).sum()
}

fn main() {
    let mut c = Cursor::new(Vec::new());
    write!(c, "hello {}", 42).unwrap();
    c.write_all(b" world").unwrap();
    println!("cursor pos {} inner {:?}", c.position(), String::from_utf8_lossy(c.get_ref()));
    c.seek(SeekFrom::Start(6)).unwrap();
    c.write_all(b"XY").unwrap();
    println!("overwrite {:?}", String::from_utf8_lossy(c.get_ref()));
    c.seek(SeekFrom::End(2)).unwrap();
    c.write_all(b"!").unwrap();
    println!("past end {:?}", c.get_ref());
    println!("seek negative err {}", c.seek(SeekFrom::Current(-100)).is_err());
    c.set_position(0);
    let mut s = String::new();
    c.read_to_string(&mut s).unwrap();
    println!("read back {:?}", s);
    println!("stream_position {}", c.stream_position().unwrap());
    c.rewind().unwrap();
    let mut two = [0u8; 2];
    c.read_exact(&mut two).unwrap();
    println!("after rewind {:?}", two);

    let mut sl: &[u8] = b"line one\nline two\r\nthree";
    let mut first = String::new();
    sl.read_line(&mut first).unwrap();
    println!("slice read_line {:?} rest {}", first, sl.len());
    let rest: Vec<String> = sl.lines().map(|l| l.unwrap()).collect();
    println!("slice lines {:?}", rest);

    let tr = Trickle { data: b"alpha\nbeta\n\ngamma", reads: 0 };
    let mut br = BufReader::with_capacity(4, tr);
    let mut v = Vec::new();
    let n = br.read_until(b'\n', &mut v).unwrap();
    println!("read_until {} {:?} buffered {:?} cap {}", n, String::from_utf8_lossy(&v), br.buffer(), br.capacity());
    let lines: Vec<String> = br.by_ref().lines().map(|l| l.unwrap()).collect();
    println!("rest lines {:?} reads {}", lines, br.get_ref().reads);

    let parts: Vec<Vec<u8>> = Cursor::new(&b"a,b,,c,"[..]).split(b',').map(|p| p.unwrap()).collect();
    println!("split {:?}", parts);
    println!("generic sum {}", generic_sum(Cursor::new("1\n2\n 3 \nx\n10")));

    let mut bytes = Vec::new();
    for b in (&b"xyz"[..]).bytes() {
        bytes.push(b.unwrap());
    }
    println!("bytes {:?}", bytes);
    let mut chained = String::new();
    (&b"left-"[..]).chain(&b"right"[..]).read_to_string(&mut chained).unwrap();
    println!("chain {:?}", chained);
    let mut taken = String::new();
    (&b"0123456789"[..]).take(4).read_to_string(&mut taken).unwrap();
    println!("take {:?}", taken);

    let mut src: &[u8] = &[7u8; 20000];
    let mut dst = Vec::new();
    let copied = io::copy(&mut src, &mut dst).unwrap();
    println!("copy {} {}", copied, dst.len());
    let n = io::copy(&mut io::empty(), &mut io::sink()).unwrap();
    println!("empty->sink {}", n);
    let mut rep = [0u8; 4];
    io::repeat(b'z').read_exact(&mut rep).unwrap();
    println!("repeat {:?}", rep);

    let mut bw = BufWriter::with_capacity(10, CountingWriter { writes: 0, data: Vec::new() });
    bw.write_all(b"abc").unwrap();
    bw.write_all(b"defg").unwrap();
    println!("bufwriter buffered {} writes {}", bw.buffer().len(), bw.get_ref().writes);
    bw.write_all(b"hijkl").unwrap();
    println!("after overflow buffered {} writes {}", bw.buffer().len(), bw.get_ref().writes);
    bw.write_all(b"a long chunk bigger than cap").unwrap();
    println!("big chunk writes {}", bw.get_ref().writes);
    let inner = bw.into_inner().unwrap_or_else(|_| panic!("into_inner"));
    println!("inner {:?} writes {}", String::from_utf8_lossy(&inner.data), inner.writes);

    let mut lw = LineWriter::new(CountingWriter { writes: 0, data: Vec::new() });
    lw.write_all(b"partial").unwrap();
    println!("linewriter partial writes {}", lw.get_ref().writes);
    lw.write_all(b" line\nnext").unwrap();
    println!("linewriter after newline writes {} data {:?}", lw.get_ref().writes, String::from_utf8_lossy(&lw.get_ref().data));
    lw.flush().unwrap();
    println!("after flush {:?}", String::from_utf8_lossy(&lw.get_ref().data));

    let mut out = Vec::new();
    writeln!(out, "{:>5}|{:<4}|{:^7}|", 12, "ab", 'c').unwrap();
    write!(&mut out, "{:?}", (1, "two")).unwrap();
    println!("vec writer {:?}", String::from_utf8(out).unwrap());

    let mut arr = [0u8; 6];
    {
        let mut cur = Cursor::new(&mut arr[..]);
        let n = cur.write(b"12345678").unwrap();
        println!("cursor slice wrote {} pos {}", n, cur.position());
    }
    println!("arr {:?}", arr);
    let mut s2 = String::new();
    let n = io::read_to_string(Cursor::new("whole")).map(|s| { s2 = s; s2.len() }).unwrap();
    println!("io::read_to_string {} {:?}", n, s2);
    let mut v2 = Vec::new();
    Cursor::new(vec![1u8, 2, 3]).read_to_end(&mut v2).unwrap();
    println!("read_to_end {:?}", v2);
}
