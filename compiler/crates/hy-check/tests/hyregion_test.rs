//! Tests for HyRegion (Region-Isolated Linear Slabs) semantics.

use hy_check::{check_file, Verdict};
use hy_syntax::parse_file;

fn check(src: &str) -> hy_check::CheckResult {
    let ast = parse_file("test.hy", src).expect("snippet parses");
    check_file(&ast)
}

#[test]
fn test_hyregion_allocation_and_move() {
    // Test that allocating in a region and moving it to a task is accepted.
    // This bypasses the standard alias reconstruction because the slab is isolated.
    let r = check(
        "struct Data {
           val int
         }
         
         fn worker(d *Data) {
           d.val = 100
         }

         fn main() {
           // 'region' keyword allocates in an off-heap linear slab
           d := region Data{val: 42}
           
           // Moving a region-allocated object is a zero-copy page transfer
           task worker(move d)
           
           // d is now moved, any access here would be a compile error
           // but the important part is the lack of handle-table overhead.
         }",
    );
    assert_eq!(r.verdict, Verdict::Accept);
}

#[test]
fn test_hyregion_isolation_violation() {
    // Test that we cannot put Go-heap pointers into a HyRegion.
    // This is critical for the safety of the zero-sync transfer.
    let r = check(
        "struct Container {
           m map[string]int // Maps are always on the Go heap
         }

         fn main() {
           m1 := map[string]int{\"a\": 1}
           // This should fail because Container contains a Go-heap pointer (map)
           // and thus cannot be isolated in a linear slab.
           c := region Container{m: m1}
         }",
    );
    // The verdict should be Reject because of isolation violation.
    assert_eq!(r.verdict, Verdict::Reject);
}
