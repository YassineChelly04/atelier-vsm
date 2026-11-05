//! Moteur de calcul VSM : takt, couvertures, lead time, temps VA, PCE,
//! en-cours, opérateurs théoriques, capacité et charge par processus.
//! Mêmes formules que l'éditeur (voir l'onglet Indicateurs).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct Doc {
    pub pages: Vec<Page>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Page {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub params: Params,
    #[serde(default)]
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Params {
    pub demand: f64,
    pub days: f64,
    pub shifts: f64,
    pub shift_min: f64,
    pub pack: f64,
}

impl Default for Params {
    fn default() -> Self {
        Params { demand: 18400.0, days: 20.0, shifts: 2.0, shift_min: 460.0, pack: 20.0 }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Node {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub w: f64,
    #[serde(default)]
    pub props: HashMap<String, Value>,
}

impl Node {
    fn num(&self, k: &str) -> f64 {
        match self.props.get(k) {
            Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
            Some(Value::String(s)) => s.replace(',', ".").trim().parse().unwrap_or(0.0),
            _ => 0.0,
        }
    }
    fn text(&self, k: &str) -> String {
        match self.props.get(k) {
            Some(Value::String(s)) => s.clone(),
            Some(v) if !v.is_null() => v.to_string(),
            _ => String::new(),
        }
    }
    fn cx(&self) -> f64 {
        self.x + self.w / 2.0
    }
    fn is_process(&self) -> bool {
        self.kind == "process" || self.kind == "shared"
    }
    fn is_flow(&self) -> bool {
        self.is_process() || matches!(self.kind.as_str(), "inventory" | "supermarket" | "fifo")
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProcessLine {
    pub name: String,
    pub ct: f64,
    pub co: f64,
    pub uptime: f64,
    pub operators: f64,
    pub capacity_per_day: f64,
    pub load: f64,
    pub ct_over_takt: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Metrics {
    pub daily_demand: f64,
    pub available_s: f64,
    pub takt_s: f64,
    pub pitch_min: f64,
    pub lead_time_days: f64,
    pub value_added_s: f64,
    pub pce: f64,
    pub wip: f64,
    pub operators: f64,
    pub operators_theoretical: f64,
    pub bottleneck: Option<String>,
    pub processes: Vec<ProcessLine>,
}

pub fn metrics(p: &Page) -> Metrics {
    let pr = &p.params;
    let daily = pr.demand / pr.days.max(1.0);
    let avail = pr.shifts * pr.shift_min * 60.0;
    let takt = if daily > 0.0 { avail / daily } else { 0.0 };

    let mut flow: Vec<&Node> = p.nodes.iter().filter(|n| n.is_flow()).collect();
    flow.sort_by(|a, b| a.cx().partial_cmp(&b.cx()).unwrap_or(std::cmp::Ordering::Equal));

    let mut pt = 0.0;
    let mut ops = 0.0;
    let mut wip = 0.0;
    let mut inv_days = 0.0;
    let mut bottleneck: Option<&Node> = None;
    let mut processes = Vec::new();

    for n in &flow {
        if n.is_process() {
            let ct = n.num("ct");
            let up = n.num("up") / 100.0;
            let scrap = n.num("scrap") / 100.0;
            pt += ct;
            ops += n.num("op");
            if bottleneck.map_or(true, |b| ct > b.num("ct")) {
                bottleneck = Some(n);
            }
            let cap = if ct > 0.0 { avail * up * (1.0 - scrap) / ct } else { 0.0 };
            processes.push(ProcessLine {
                name: n.text("name"),
                ct,
                co: n.num("co"),
                uptime: n.num("up"),
                operators: n.num("op"),
                capacity_per_day: cap,
                load: if cap > 0.0 { daily / cap } else { 0.0 },
                ct_over_takt: if takt > 0.0 { ct / takt } else { 0.0 },
            });
        } else {
            // Quantité saisie, ou couverture saisie en jours × demande journalière.
            let q = if n.num("qty") == 0.0 && n.num("cov") > 0.0 { n.num("cov") * daily } else { n.num("qty") };
            wip += q;
            if daily > 0.0 {
                inv_days += q / daily;
            }
        }
    }

    let lt = inv_days + if avail > 0.0 { pt / avail } else { 0.0 };
    let pce = if lt > 0.0 && avail > 0.0 { pt / (lt * avail) } else { 0.0 };

    Metrics {
        daily_demand: daily,
        available_s: avail,
        takt_s: takt,
        pitch_min: takt * pr.pack / 60.0,
        lead_time_days: lt,
        value_added_s: pt,
        pce,
        wip,
        operators: ops,
        operators_theoretical: if takt > 0.0 { pt / takt } else { 0.0 },
        bottleneck: bottleneck.map(|b| b.text("name")),
        processes,
    }
}

fn csv_field(s: &str) -> String {
    if s.contains([';', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn dec(v: f64, d: usize) -> String {
    format!("{v:.d$}").replace('.', ",")
}

/// Rapport CSV (séparateur « ; », décimales à virgule pour Excel en français).
pub fn report_csv(d: &Doc) -> String {
    let mut out = String::from("\u{feff}");
    out.push_str("Page;Indicateur;Valeur;Unité\n");
    for p in &d.pages {
        let m = metrics(p);
        let page = csv_field(&p.name);
        let rows: [(&str, String, &str); 10] = [
            ("Demande journalière", dec(m.daily_demand, 1), "p/j"),
            ("Takt", dec(m.takt_s, 1), "s"),
            ("Pitch", dec(m.pitch_min, 1), "min"),
            ("Lead time", dec(m.lead_time_days, 2), "j"),
            ("Temps à valeur ajoutée", dec(m.value_added_s, 1), "s"),
            ("PCE", dec(m.pce * 100.0, 4), "%"),
            ("En-cours", dec(m.wip, 0), "p"),
            ("Opérateurs", dec(m.operators, 1), ""),
            ("Opérateurs théoriques", dec(m.operators_theoretical, 2), ""),
            ("Goulot", csv_field(m.bottleneck.as_deref().unwrap_or("—")), ""),
        ];
        for (k, v, u) in rows {
            out.push_str(&format!("{page};{k};{v};{u}\n"));
        }
    }
    out.push_str("\nPage;Processus;C/T (s);C/O (min);Disponibilité (%);Opérateurs;Capacité (p/j);Charge (%);C/T ÷ takt (%)\n");
    for p in &d.pages {
        let m = metrics(p);
        for l in &m.processes {
            out.push_str(&format!(
                "{};{};{};{};{};{};{};{};{}\n",
                csv_field(&p.name),
                csv_field(&l.name),
                dec(l.ct, 1),
                dec(l.co, 1),
                dec(l.uptime, 1),
                dec(l.operators, 1),
                dec(l.capacity_per_day, 0),
                dec(l.load * 100.0, 1),
                dec(l.ct_over_takt * 100.0, 1)
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> Page {
        let v = json!({
            "name": "État actuel",
            "params": {"demand": 18400, "days": 20, "shifts": 2, "shiftMin": 460, "pack": 20},
            "nodes": [
                {"type":"inventory","x":0,"w":50,"props":{"qty":5000}},
                {"type":"process","x":100,"w":130,"props":{"name":"Découpe","ct":39,"up":85,"op":1,"scrap":1}},
                {"type":"inventory","x":250,"w":50,"props":{"qty":4600}},
                {"type":"process","x":350,"w":130,"props":{"name":"Soudure","ct":46,"up":100,"op":1}},
                {"type":"inventory","x":500,"w":50,"props":{"qty":1100}},
                {"type":"process","x":600,"w":130,"props":{"name":"Peinture","ct":62,"up":80,"op":1,"scrap":3}},
                {"type":"inventory","x":750,"w":50,"props":{"qty":1600}},
                {"type":"process","x":850,"w":130,"props":{"name":"Assemblage","ct":"40","up":100,"op":1}},
                {"type":"inventory","x":1000,"w":50,"props":{"qty":2700}},
                {"type":"customer","x":1000,"w":120,"props":{"name":"Client"}}
            ]
        });
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn takt_and_demand() {
        let m = metrics(&sample());
        assert!((m.daily_demand - 920.0).abs() < 1e-9);
        assert!((m.available_s - 55_200.0).abs() < 1e-9);
        assert!((m.takt_s - 60.0).abs() < 1e-9);
        assert!((m.pitch_min - 20.0).abs() < 1e-9);
    }

    #[test]
    fn lead_time_value_added_and_little() {
        let m = metrics(&sample());
        assert!((m.value_added_s - 187.0).abs() < 1e-9);
        assert!((m.wip - 15_000.0).abs() < 1e-9);
        // Loi de Little : WIP / débit = attente en jours.
        let wait = m.wip / m.daily_demand;
        assert!((m.lead_time_days - (wait + 187.0 / 55_200.0)).abs() < 1e-9);
        assert!((m.lead_time_days - 16.308).abs() < 1e-3);
        assert!(m.pce > 0.0 && m.pce < 0.001);
    }

    #[test]
    fn bottleneck_and_operators() {
        let m = metrics(&sample());
        assert_eq!(m.bottleneck.as_deref(), Some("Peinture"));
        assert!((m.operators_theoretical - 187.0 / 60.0).abs() < 1e-9);
        let paint = m.processes.iter().find(|p| p.name == "Peinture").unwrap();
        // 55 200 × 0,80 × 0,97 ÷ 62
        assert!((paint.capacity_per_day - 55_200.0 * 0.8 * 0.97 / 62.0).abs() < 1e-6);
        assert!(paint.ct_over_takt > 1.0);
    }

    #[test]
    fn csv_report_has_rows() {
        let d = Doc { pages: vec![sample()] };
        let csv = report_csv(&d);
        assert!(csv.contains("État actuel;Takt;60,0;s"));
        assert!(csv.contains("Peinture;62,0"));
    }

    #[test]
    fn coverage_in_days_counts_like_quantity() {
        let mut p = sample();
        // Stock amont saisi en jours (5 j) au lieu d'une quantité : 5 × 920 = 4 600 p.
        p.nodes.insert(0, serde_json::from_value(json!({"type":"inventory","x":-60,"w":50,"props":{"qty":"","cov":5}})).unwrap());
        let m = metrics(&p);
        assert!((m.wip - 19_600.0).abs() < 1e-9);
    }

    #[test]
    fn empty_page_is_safe() {
        let p: Page = serde_json::from_value(json!({"name":"vide","params":{"demand":0},"nodes":[]})).unwrap();
        let m = metrics(&p);
        assert_eq!(m.takt_s, 0.0);
        assert_eq!(m.bottleneck, None);
    }
}
