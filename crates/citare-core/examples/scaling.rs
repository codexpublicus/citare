//! Times get_citations, resolve_citations and annotate_citations over a
//! text repeated 1, 2, 4, 8 and 16 times, and fits the scaling exponent.
//!
//! usage: cargo run --release --example scaling -- <text file> [runs] [max multiple] [markup]
//!
//! With `markup`, the file is passed as markup_text (HTML or XML), and
//! annotate maps spans back onto it.
use std::time::{Duration, Instant};

/// Multiple, and median find, resolve and annotate times in ms.
type Row = (usize, f64, f64, f64);

fn median(mut v: Vec<Duration>) -> f64 {
    v.sort();
    v[v.len() / 2].as_secs_f64() * 1000.0
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: scaling <file> [runs] [max] [markup]");
    let runs: usize = args.next().map_or(5, |s| s.parse().unwrap());
    let max: usize = args.next().map_or(16, |s| s.parse().unwrap());
    let markup = args.next().as_deref() == Some("markup");
    let base = std::fs::read_to_string(path).unwrap();
    citare_core::get_citations("warm up 1 U.S. 1", false, "", &[]);

    println!("mult       KB  cites   find_ms resolve_ms annot_ms");
    let mut rows: Vec<Row> = Vec::new();
    let mut k = 1;
    while k <= max {
        let text = base.repeat(k);
        let (mut tf, mut tr, mut ta) = (vec![], vec![], vec![]);
        let mut n = 0;
        for _ in 0..runs {
            let t = Instant::now();
            let (plain, cites) = if markup {
                let mut doc =
                    citare_core::Document::new("", &text, &[citare_core::CleanStep::Html]);
                let c = citare_core::get_citations_from_document(&mut doc, false);
                (doc.plain_text.clone(), c)
            } else {
                (
                    text.clone(),
                    citare_core::get_citations(&text, false, "", &[]),
                )
            };
            tf.push(t.elapsed());
            n = cites.len();
            let t = Instant::now();
            let r = citare_core::resolve_citations(&cites);
            tr.push(t.elapsed());
            std::hint::black_box(r);
            let mut ann: Vec<_> = cites
                .iter()
                .map(|c| (c.span(), "<a>".to_string(), "</a>".to_string()))
                .collect();
            let src = if markup { text.as_str() } else { "" };
            let t = Instant::now();
            let out = citare_core::annotate_citations(
                &plain,
                &mut ann,
                src,
                citare_core::UnbalancedTags::Unchecked,
                None,
                None,
            );
            ta.push(t.elapsed());
            std::hint::black_box(out);
        }
        let row = (k, median(tf), median(tr), median(ta));
        println!(
            "{:>4} {:>8.0} {:>6} {:>9.1} {:>10.1} {:>8.1}",
            k,
            text.len() as f64 / 1024.0,
            n,
            row.1,
            row.2,
            row.3
        );
        rows.push(row);
        k *= 2;
    }
    // Least-squares slope of log(time) against log(multiple).
    let fit = |f: &dyn Fn(&Row) -> f64| {
        let pts: Vec<(f64, f64)> = rows
            .iter()
            .map(|r| ((r.0 as f64).ln(), f(r).max(1e-6).ln()))
            .collect();
        let n = pts.len() as f64;
        let mx = pts.iter().map(|p| p.0).sum::<f64>() / n;
        let my = pts.iter().map(|p| p.1).sum::<f64>() / n;
        let num: f64 = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
        let den: f64 = pts.iter().map(|p| (p.0 - mx).powi(2)).sum();
        num / den
    };
    println!(
        "exponent: find {:.2}, resolve {:.2}, annotate {:.2}",
        fit(&|r| r.1),
        fit(&|r| r.2),
        fit(&|r| r.3)
    );
}
