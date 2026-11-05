//! Convertit une ou plusieurs planches SVG en un PDF multipage.
//! Usage : cargo run --example svg_to_pdf -- sortie.pdf planche1.svg [planche2.svg …]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        eprintln!("usage : svg_to_pdf sortie.pdf planche.svg [...]");
        std::process::exit(2);
    }
    let sheets: Vec<String> = args[1..]
        .iter()
        .map(|p| std::fs::read_to_string(p).expect("lecture SVG"))
        .collect();
    let pdf = atelier_vsm_lib::pdf::sheets_to_pdf(&sheets, "VSM").expect("conversion PDF");
    std::fs::write(&args[0], pdf).expect("écriture PDF");
    println!("{} page(s) → {}", sheets.len(), args[0]);
}
