// Operator precedence and associativity. Check the dump by eye; parens are
// not kept in the AST, so nesting shows the resolved precedence.
fn prec() {
    a + b * c;                  // (+ a (* b c))
    a * b + c;                  // (+ (* a b) c)
    a - b - c;                  // (- (- a b) c)          left assoc
    a = b = c;                  // (= a (= b c))          right assoc
    a += b * 2;                 // (+= a (* b 2))
    a || b && c;                // (|| a (&& b c))
    a && b || c && d;           // (|| (&& a b) (&& c d))
    a == b && c != d;           // (&& (== a b) (!= c d))
    a | b ^ c & d;              // (| a (^ b (& c d)))
    a << b + c;                 // (<< a (+ b c))
    a & b == c;                 // (== (& a b) c)       & binds tighter than ==
    -a * b;                     // (* (neg a) b)
    !a.b();                     // (! (method a b))
    -x as u32;                  // (as (neg x) u32)
    a + b as u64 * c;           // (+ a (* (as b u64) c))
    *p.q;                       // (deref (field p q))
    &a[i];                      // (& (index a i))
    a..b + 1;                   // (.. a (+ b 1))
    x = a..=b;                  // (= x (..= a b))
    ..;                         // (.. _ _)
    a.. ;                       // (.. a _)
    ..b;                        // (.. _ b)
    f(x)?.g()?;                 // (? (method (? (call f x)) g))
    a < b == (c > d);           // (== (< a b) (> c d))
    &&x;                        // (& (& x))
    &mut *p;                    // (&mut (deref p))
    a % b / c * d;              // (* (/ (% a b) c) d)
    x as i64 as u8;             // (as (as x i64) u8)
    a | b || c;                 // (|| (| a b) c)
    1 + 2 * 3 - 4 / 2 % 3;      // (- (+ 1 (* 2 3)) (% (/ 4 2) 3))
    a >> 1 >= b << 2;           // (>= (>> a 1) (<< b 2))
    x.0.1 + t.2;                // (+ (field (field x 0) 1) (field t 2))
    return a + b;
}
