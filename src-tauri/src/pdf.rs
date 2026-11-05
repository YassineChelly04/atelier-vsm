//! Export PDF vectoriel : chaque planche est un SVG dimensionné en millimètres
//! (format papier choisi dans l'interface). Les planches sont converties par
//! svg2pdf puis assemblées dans un seul PDF multipage, polices embarquées.

use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, TextStr};
use std::collections::HashMap;
use svg2pdf::usvg;

/// Polices de l'application, embarquées dans le binaire pour un rendu
/// identique sur toutes les machines (hors ligne).
const FONTS: &[&[u8]] = &[
    include_bytes!("../fonts/ibm-plex-sans-latin-400-normal.ttf"),
    include_bytes!("../fonts/ibm-plex-sans-latin-500-normal.ttf"),
    include_bytes!("../fonts/ibm-plex-sans-latin-600-normal.ttf"),
    include_bytes!("../fonts/ibm-plex-mono-latin-400-normal.ttf"),
    include_bytes!("../fonts/ibm-plex-mono-latin-500-normal.ttf"),
    include_bytes!("../fonts/barlow-condensed-latin-500-normal.ttf"),
    include_bytes!("../fonts/barlow-condensed-latin-600-normal.ttf"),
    include_bytes!("../fonts/barlow-condensed-latin-700-normal.ttf"),
];

fn options() -> usvg::Options<'static> {
    let mut opt = usvg::Options::default();
    {
        let db = opt.fontdb_mut();
        for f in FONTS {
            db.load_font_data(f.to_vec());
        }
        // Secours pour les caractères absents des sous-ensembles latins.
        db.load_system_fonts();
        db.set_sans_serif_family("IBM Plex Sans");
        db.set_monospace_family("IBM Plex Mono");
    }
    opt.font_family = "IBM Plex Sans".into();
    opt
}

/// Convertit une liste de planches SVG en un PDF multipage.
/// La taille de chaque page est celle du SVG (largeur/hauteur en mm ou pt).
pub fn sheets_to_pdf(sheets: &[String], title: &str) -> Result<Vec<u8>, String> {
    if sheets.is_empty() {
        return Err("aucune planche à exporter".into());
    }
    let opt = options();
    let mut alloc = Ref::new(1);
    let catalog_id = alloc.bump();
    let tree_id = alloc.bump();
    let info_id = alloc.bump();

    let mut pdf = Pdf::new();
    let mut page_ids = Vec::with_capacity(sheets.len());
    let mut chunks = Vec::with_capacity(sheets.len());

    for (i, svg) in sheets.iter().enumerate() {
        let tree = usvg::Tree::from_str(svg, &opt)
            .map_err(|e| format!("planche {} : SVG invalide ({e})", i + 1))?;
        // Taille SVG en px CSS (96 dpi) → points PDF (72 dpi).
        let w = tree.size().width() * 72.0 / 96.0;
        let h = tree.size().height() * 72.0 / 96.0;
        let (chunk, svg_ref) = svg2pdf::to_chunk(&tree, svg2pdf::ConversionOptions::default())
            .map_err(|e| format!("planche {} : {e}", i + 1))?;
        let mut map = HashMap::new();
        let chunk = chunk.renumber(|old| *map.entry(old).or_insert_with(|| alloc.bump()));
        let svg_id = *map.get(&svg_ref).ok_or("référence SVG introuvable")?;

        let page_id = alloc.bump();
        let content_id = alloc.bump();
        let name = format!("S{i}");

        let mut page = pdf.page(page_id);
        page.media_box(Rect::new(0.0, 0.0, w, h));
        page.parent(tree_id);
        page.contents(content_id);
        page.resources().x_objects().pair(Name(name.as_bytes()), svg_id);
        page.finish();

        let mut content = Content::new();
        content.transform([w, 0.0, 0.0, h, 0.0, 0.0]).x_object(Name(name.as_bytes()));
        pdf.stream(content_id, &content.finish());

        page_ids.push(page_id);
        chunks.push(chunk);
    }

    pdf.catalog(catalog_id).pages(tree_id);
    pdf.pages(tree_id).kids(page_ids.iter().copied()).count(page_ids.len() as i32);
    for c in &chunks {
        pdf.extend(c);
    }
    let mut info = pdf.document_info(info_id);
    info.title(TextStr(title));
    info.producer(TextStr("Atelier VSM"));
    info.creator(TextStr("Atelier VSM"));
    info.finish();
    Ok(pdf.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet(w_mm: f32, h_mm: f32) -> String {
        format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w_mm}mm" height="{h_mm}mm" viewBox="0 0 {w_mm} {h_mm}">
<style>.t{{font-family:'IBM Plex Sans';font-weight:600;font-size:6px;fill:#17202B}}</style>
<rect x="10" y="10" width="{}" height="{}" fill="none" stroke="#17202B" stroke-width="0.5"/>
<text class="t" x="20" y="30">Délai 16,3 j — Takt 60 s</text></svg>"##,
            w_mm - 20.0,
            h_mm - 20.0
        )
    }

    #[test]
    fn multipage_pdf_with_paper_sizes() {
        // A4 paysage + A3 paysage
        let pdf = sheets_to_pdf(&[sheet(297.0, 210.0), sheet(420.0, 297.0)], "Test").unwrap();
        let s = String::from_utf8_lossy(&pdf);
        assert!(pdf.starts_with(b"%PDF-"));
        assert!(s.contains("/Count 2"));
        // 297 mm = 841,89 pt ; 420 mm = 1190,55 pt
        assert!(s.contains("841.8") || s.contains("841.9"), "largeur A4 attendue");
        assert!(s.contains("1190.5") || s.contains("1190.6"), "largeur A3 attendue");
        // Police embarquée
        assert!(s.contains("IBMPlexSans") || s.contains("IBM"), "police non embarquée");
    }

    #[test]
    fn empty_is_error() {
        assert!(sheets_to_pdf(&[], "x").is_err());
    }
}
