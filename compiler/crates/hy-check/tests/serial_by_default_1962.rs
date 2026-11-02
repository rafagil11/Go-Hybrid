//! Serial by default: safe execution first (#1962).
//!
//! In an opt-in closed fork-join scope, make task execution safe by default.
//! Rather than reject tasks whenever shared-data independence cannot be proved,
//! the compiler orders possibly conflicting tasks sequentially.

use hy_check::check_file;
use hy_syntax::parse_file;

fn codes(src: &str) -> Vec<String> {
    let ast = parse_file("test.hy", src).expect("parses");
    check_file(&ast)
        .diagnostics
        .into_iter()
        .map(|d| d.code)
        .collect()
}

#[test]
fn serial_scope_allows_unproven_shared_state() {
    // In a 'parallel' block (opt-in closed scope), shared mutable state 
    // that isn't proven safe should not cause a hard error if it's not escaping.
    // The compiler will order these tasks sequentially.
    let src = "
fn main() {
    data := []int{1, 2, 3}
    parallel {
        task { data[0] = 42 }
        task { fmt.println(data[0]) }
    }
}
";
    let cs = codes(src);
    // Should not contain HYB-E3004 (Rejected) or HYB-E3005 (Unproven)
    // because it falls back to sequential execution.
    assert!(!cs.contains(&"HYB-E3004".to_string()));
    assert!(!cs.contains(&"HYB-E3005".to_string()));
}

#[test]
fn serial_scope_still_rejects_escape() {
    // If the mutable state escapes the scope (e.g. to a foreign Go call),
    // it must still be rejected (HYB-E3013).
    let src = "
import \"net/http\"
fn main() {
    hits := 0
    parallel {
        task {
            mux := http.new_serve_mux()
            mux.handle(\"/\", http.handler(fn() { hits++ }))
        }
    }
}
";
    let cs = codes(src);
    assert!(cs.contains(&"HYB-E3013".to_string()));
}
