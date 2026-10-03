//! Output must be byte-identical across processes. Each process seeds std's
//! `HashMap`/`HashSet` hashers at random, so running the binary several
//! times catches any hash iteration order that reaches the JSON (#22).

use std::process::Command;

const TEXT: &str = "Foo v. Bar, 1 U.S. 1, 4 (1999); see also Lissner v. Saad, \
    5 F.3d 17, 19 (2d Cir. 2003); 2 Mass. 3; 1 Johns. 2; 22 U.S.C. § 1983. \
    Id. at 5. Bar, 1 U.S., at 3. Lissner, supra, at 20. \
    In re Smith, 100 N.Y.S.2d 4, 7 (Sup. Ct. 1950); 3 Ohio St. 3d 4; \
    1 Tex. 2; Lissner, 5 F.3d at 18.";

const RUNS: usize = 6;

fn run(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_citare"))
        .args(args)
        .arg("--text")
        .arg(TEXT)
        .output()
        .expect("failed to run citare");
    assert!(
        out.status.success(),
        "citare {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("stdout is not UTF-8")
}

fn assert_stable(args: &[&str]) {
    let first = run(args);
    assert!(first.len() > 2, "no output from citare {args:?}");
    for i in 1..RUNS {
        let next = run(args);
        assert_eq!(first, next, "citare {args:?} output differed on run {i}");
    }
}

#[test]
fn find_output_is_identical_across_processes() {
    assert_stable(&["find"]);
}

#[test]
fn resolve_output_is_identical_across_processes() {
    assert_stable(&["resolve"]);
}

#[test]
fn annotate_output_is_identical_across_processes() {
    assert_stable(&["annotate"]);
}
