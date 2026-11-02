use hy_check::{check_file, Verdict};
use hy_syntax::parse_file;

fn check(src: &str) -> hy_check::CheckResult {
    let path = "inline.hy";
    let ast = parse_file(path, src).unwrap_or_else(|diags| panic!("must parse: {:?}", diags));
    check_file(&ast)
}

#[test]
fn hy_region_composite_move_isolation() {
    // .hy types are region-allocated and isolated.
    // Moving them to a task transfers the whole region page.
    let src = r#"
struct data .hy {
    val int
}

fn worker(d *data, done chan int) {
    d.val = 42
    done <- 1
}

fn main() {
    d := data{val: 0}
    done := make(chan int, 1)
    // This move is now a zero-copy page transfer, not a handle copy.
    task worker(move d, done)
    <-done
}
"#;
    let res = check(src);
    assert_eq!(res.diagnostics.len(), 0);
    assert!(matches!(res.verdict, Verdict::Accept));
}

#[test]
fn hy_region_reject_ambient_gc_alias() {
    // Attempting to alias a .hy type into a non-hy (ambient GC) container
    // should be rejected because they live in different memory regions.
    let src = r#"
struct data .hy {
    val int
}

struct container {
    ptr *data
}

fn main() {
    d := data{val: 10}
    c := container{ptr: &d} // Error: .hy type cannot be aliased into ambient GC heap
}
"#;
    let res = check(src);
    assert!(matches!(res.verdict, Verdict::Reject));
    // HYB-E4001: Region isolation violation
    assert!(res.diagnostics.iter().any(|d| d.code == "HYB-E4001"));
}

#[test]
fn hy_region_nested_move_preserves_isolation() {
    let src = r#"
struct inner .hy {
    x int
}

struct outer .hy {
    i *inner
}

fn worker(o outer, done chan int) {
    o.i.x = 100
    done <- 1
}

fn main() {
    i := inner{x: 0}
    o := outer{i: &i}
    done := make(chan int, 1)
    task worker(move o, done)
    <-done
}
"#;
    let res = check(src);
    assert_eq!(res.diagnostics.len(), 0);
    assert!(matches!(res.verdict, Verdict::Accept));
}
