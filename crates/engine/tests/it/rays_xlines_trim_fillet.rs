//! TRIM, FILLET and CHAMFER on rays and xlines (#1235): what is left of a ray with a finite end
//! is a line, what is left of an xline is a ray, as in AutoCAD. EXTEND moves a ray's base (#1236).

use cadcraft_engine::Session;
use cadcraft_engine::doc::EntityKind;

fn r(v: f64) -> f64 {
    (v * 1e6).round() / 1e6 + 0.0
}

/// Every object as text, sorted: lines with their ends in a fixed order, rays and xlines with
/// base and direction.
fn objects(s: &Session) -> Vec<String> {
    let mut v: Vec<String> = s
        .doc()
        .unwrap()
        .model
        .iter()
        .map(|e| match &e.kind {
            EntityKind::Line(l) => {
                let (a, b) = ((r(l.a.x), r(l.a.y)), (r(l.b.x), r(l.b.y)));
                let (a, b) = if a <= b { (a, b) } else { (b, a) };
                format!("LINE {a:?} {b:?}")
            }
            EntityKind::Ray(k) => format!("RAY {:?} {:?}", (r(k.base.x), r(k.base.y)), (r(k.dir.x), r(k.dir.y))),
            EntityKind::XLine(k) => format!("XLINE {:?} {:?}", (r(k.base.x), r(k.base.y)), (r(k.dir.x), r(k.dir.y))),
            EntityKind::Arc(_) => "ARC".to_string(),
            k => format!("{k:?}"),
        })
        .collect();
    v.sort();
    v
}

/// A vertical line at x = 10 and, along y = 0, the ray or xline drawn by `cmd`.
fn session(cmd: &str) -> Session {
    let mut s = Session::new();
    s.cmdline("LINE 10,-2 10,12").unwrap();
    s.cmdline("").unwrap();
    s.script(&format!("{cmd}\n\n")).unwrap();
    assert!(s.running.is_none(), "{}", s.prompt_text());
    s
}

fn run(cmd: &str, then: &str) -> Vec<String> {
    let mut s = session(cmd);
    s.script(then).unwrap();
    assert!(s.running.is_none(), "{cmd} / {then}: {}", s.prompt_text());
    objects(&s)
}

const EDGE: &str = "LINE (10.0, -2.0) (10.0, 12.0)";

/// The expected objects, sorted as `objects` sorts them.
fn want(v: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = v.iter().map(|s| s.to_string()).collect();
    v.sort();
    v
}

#[test]
fn trim_a_ray_leaves_a_line_or_a_ray() {
    // The far side goes: the ray keeps its base and becomes a line.
    assert_eq!(run("RAY 0,0 1,0", "TRIM 15,0\n\n"), want(&[EDGE, "LINE (0.0, 0.0) (10.0, 0.0)"]));
    // The side with the base goes: what is left is still infinite, a ray from the edge.
    assert_eq!(run("RAY 0,0 1,0", "TRIM 5,0\n\n"), want(&[EDGE, "RAY (10.0, 0.0) (1.0, 0.0)"]));
}

#[test]
fn trim_an_xline_leaves_a_ray() {
    assert_eq!(run("XLINE H 0,0", "TRIM 15,0\n\n"), want(&[EDGE, "RAY (10.0, 0.0) (-1.0, 0.0)"]));
    assert_eq!(run("XLINE H 0,0", "TRIM 5,0\n\n"), want(&[EDGE, "RAY (10.0, 0.0) (1.0, 0.0)"]));
}

#[test]
fn trim_between_two_edges_splits_a_ray_and_an_xline() {
    let mut s = session("RAY -5,0 1,0");
    s.cmdline("LINE 0,-2 0,12").unwrap();
    s.cmdline("").unwrap();
    s.script("TRIM 5,0\n\n").unwrap();
    let edges = ["LINE (0.0, -2.0) (0.0, 12.0)", EDGE];
    assert_eq!(objects(&s), want(&[&edges[..], &["LINE (-5.0, 0.0) (0.0, 0.0)", "RAY (10.0, 0.0) (1.0, 0.0)"]].concat()));
    let mut s = session("XLINE H 0,0");
    s.cmdline("LINE 0,-2 0,12").unwrap();
    s.cmdline("").unwrap();
    s.script("TRIM 5,0\n\n").unwrap();
    assert_eq!(objects(&s), want(&[&edges[..], &["RAY (0.0, 0.0) (-1.0, 0.0)", "RAY (10.0, 0.0) (1.0, 0.0)"]].concat()));
}

#[test]
fn fillet_a_ray_or_an_xline_with_a_line() {
    let corner = "LINE (10.0, 0.0) (10.0, 12.0)";
    // The ray's base side kept: it becomes a line to the corner.
    assert_eq!(run("RAY 0,0 1,0", "FILLET R 0 5,0 10,5\n"), want(&["LINE (0.0, 0.0) (10.0, 0.0)", corner]));
    // Its infinite side kept: a ray from the corner.
    assert_eq!(run("RAY 0,0 1,0", "FILLET R 0 15,0 10,5\n"), want(&["RAY (10.0, 0.0) (1.0, 0.0)", corner]));
    // An xline becomes a ray running to the picked side, from the arc's tangent point.
    assert_eq!(run("XLINE H 0,0", "FILLET R 1 5,0 10,5\n"), want(&["ARC", "LINE (10.0, 1.0) (10.0, 12.0)", "RAY (9.0, 0.0) (-1.0, 0.0)"]));
    // Two xlines: two rays.
    let mut s = Session::new();
    s.script("XLINE H 0,0\n\nXLINE V 10,0\n\nFILLET R 0 5,0 10,5\n").unwrap();
    assert!(s.running.is_none(), "{}", s.prompt_text());
    assert_eq!(objects(&s), want(&["RAY (10.0, 0.0) (-1.0, 0.0)", "RAY (10.0, 0.0) (0.0, 1.0)"]));
}

#[test]
fn chamfer_a_ray_and_an_xline() {
    assert_eq!(
        run("RAY 0,0 1,0", "CHAMFER D 2 2 15,0 10,5\n"),
        want(&["LINE (10.0, 2.0) (10.0, 12.0)", "LINE (10.0, 2.0) (12.0, 0.0)", "RAY (12.0, 0.0) (1.0, 0.0)"])
    );
    assert_eq!(
        run("XLINE H 0,0", "CHAMFER D 2 2 5,0 10,5\n"),
        want(&["LINE (10.0, 2.0) (10.0, 12.0)", "LINE (8.0, 0.0) (10.0, 2.0)", "RAY (8.0, 0.0) (-1.0, 0.0)"])
    );
}

#[test]
fn extend_a_ray_moves_its_base_back_to_the_edge() {
    assert_eq!(run("RAY 15,0 16,0", "EXTEND 16,0\n\n"), want(&[EDGE, "RAY (10.0, 0.0) (1.0, 0.0)"]));
    // The edge lies ahead of the base, not behind it: the ray is left alone.
    let mut s = session("RAY 5,0 6,0");
    s.script("EXTEND 6,0\n\n").unwrap();
    assert_eq!(objects(&s), want(&[EDGE, "RAY (5.0, 0.0) (1.0, 0.0)"]));
}

/// An xline at 30 degrees through the origin and vertical lines at x = 123.456 and 200: values
/// that are not binary fractions, so a cut point computed from far away shows its error.
fn slanted(more: &str) -> Session {
    let mut s = Session::new();
    s.script(&format!("XLINE A 30 0,0\n\nLINE 123.456,-10 123.456,300\n\n{more}")).unwrap();
    assert!(s.running.is_none(), "{}", s.prompt_text());
    s
}

/// The point of the 30 degree xline at `x`, as command-line text.
fn on_slant(x: f64) -> String {
    format!("{x},{}", x * 30f64.to_radians().tan())
}

fn ray_base_x(s: &Session) -> f64 {
    let rays: Vec<f64> = s.doc().unwrap().model.iter().filter_map(|e| if let EntityKind::Ray(k) = &e.kind { Some(k.base.x) } else { None }).collect();
    assert_eq!(rays.len(), 1, "{:?}", objects(s));
    rays[0]
}

#[test]
fn an_xline_cut_keeps_the_cut_point_exact() {
    let mut s = slanted("");
    s.script(&format!("TRIM {}\n\n", on_slant(100.0))).unwrap();
    assert!((ray_base_x(&s) - 123.456).abs() < 1e-10, "{}", ray_base_x(&s) - 123.456);
    let mut s = slanted("");
    s.script(&format!("FILLET R 0 {} 123.456,0\n", on_slant(200.0))).unwrap();
    assert!(s.running.is_none(), "{}", s.prompt_text());
    assert!((ray_base_x(&s) - 123.456).abs() < 1e-10, "{}", ray_base_x(&s) - 123.456);
}

#[test]
fn a_ray_left_by_trim_does_not_see_its_own_edge_as_a_cut() {
    // The ray left by the first TRIM starts on the edge: the second TRIM must cut only at x = 200,
    // not also leave a sliver between the ray's base and the edge it was trimmed to.
    let mut s = slanted("LINE 200,-10 200,300\n\n");
    s.script(&format!("TRIM {}\n\nTRIM {}\n\n", on_slant(100.0), on_slant(160.0))).unwrap();
    assert!(s.running.is_none(), "{}", s.prompt_text());
    assert_eq!(objects(&s).len(), 3, "{:?}", objects(&s));
}

#[test]
fn preselected_xline_is_the_cutting_edge_in_both_modes() {
    use cadcraft_engine::Input;
    use serde_json::json;
    for mode in [0, 1] {
        let mut s = Session::new();
        let edge = s.execute("xline", &json!({"base":[10,0], "through":[10,1]})).unwrap()["handle"].as_str().unwrap().to_owned();
        let target = s.execute("xline", &json!({"base":[0,0], "through":[1,0]})).unwrap()["handle"].as_str().unwrap().to_owned();
        // This unselected edge must not cut the target.
        s.execute("xline", &json!({"base":[20,0], "through":[20,1]})).unwrap();
        let edge = cadcraft_engine::doc::Handle::parse_hex(&edge).unwrap();
        let target = cadcraft_engine::doc::Handle::parse_hex(&target).unwrap();
        s.doc_mut().unwrap().header.set_i64("TRIMEXTENDMODE", mode);
        s.set_selection(vec![edge]);
        s.start("trim").unwrap();
        assert!(s.prompt_text().contains("Select object to trim"), "{}", s.prompt_text());
        s.input(Input::Point(cadcraft_engine::geom::Vec2::new(15.0, 0.0))).unwrap();
        let d = s.doc().unwrap();
        assert!(matches!(d.entity(edge).unwrap().kind, EntityKind::XLine(_)));
        let EntityKind::Ray(ray) = &d.entity(target).unwrap().kind else { panic!("trim must preserve the target handle and leave a ray") };
        assert!((ray.base.x - 10.0).abs() < 1e-9 && ray.dir.x < 0.0);
        s.input(Input::Keyword("Undo".into())).unwrap();
        assert!(matches!(s.doc().unwrap().entity(target).unwrap().kind, EntityKind::XLine(_)));
        s.input(Input::Enter).unwrap();
        assert!(s.running.is_none());
    }
}

#[test]
fn ray_cutting_edge_does_not_cut_behind_its_base() {
    use serde_json::json;
    let mut s = Session::new();
    let edge = s.execute("ray", &json!({"base":[10,5],"through":[10,6]})).unwrap()["handle"].clone();
    let target = s.execute("line", &json!({"points":[[0,0],[20,0]]})).unwrap()["handles"][0].clone();
    let error = s.execute("trim", &json!({"handle":target,"pick":[15,0],"edges":[edge]})).unwrap_err();
    assert!(error.to_string().contains("No usable cutting-edge intersection"), "{error}");
    let target = s.execute("line", &json!({"points":[[0,10],[20,10]]})).unwrap()["handles"][0].clone();
    s.execute("trim", &json!({"handle":target,"pick":[15,10],"edges":[edge]})).unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 3);
}

#[test]
fn picking_the_cut_leaves_the_xline_unchanged() {
    use serde_json::json;
    let mut s = Session::new();
    s.execute("line", &json!({"points":[[10,-10],[10,10]]})).unwrap();
    let target = s.execute("xline", &json!({"base":[0,0],"through":[1,0]})).unwrap()["handle"].clone();
    assert!(s.execute("trim", &json!({"handle":target,"pick":[10,0],"mode":"quick"})).is_err());
    let h = cadcraft_engine::doc::Handle::parse_hex(target.as_str().unwrap()).unwrap();
    assert!(matches!(s.doc().unwrap().entity(h).unwrap().kind, EntityKind::XLine(_)));
}

#[test]
fn pickfirst_off_asks_for_edges_in_standard_mode() {
    let mut s = session("XLINE H 0,0");
    let h = s.doc().unwrap().model.iter().next().unwrap().handle;
    s.settings.pickfirst = false;
    s.doc_mut().unwrap().header.set_i64("TRIMEXTENDMODE", 0);
    s.set_selection(vec![h]);
    s.start("trim").unwrap();
    assert!(s.prompt_text().contains("Select objects"));
    assert!(s.selection().is_empty());
}

#[test]
fn fillet_and_chamfer_no_trim_preserve_unbounded_sources() {
    use serde_json::json;
    for command in ["fillet", "chamfer"] {
        let mut s = Session::new();
        let h1 = s.execute("xline", &json!({"base":[0,0],"through":[1,0]})).unwrap()["handle"].clone();
        let h2 = s.execute("ray", &json!({"base":[10,0],"through":[10,1]})).unwrap()["handle"].clone();
        let before = objects(&s);
        s.execute(command, &json!({"h1":h1,"h2":h2,"p1":[5,0],"p2":[10,5],"radius":1,"d1":1,"d2":1,"trim":false})).unwrap();
        let after = objects(&s);
        assert!(before.iter().all(|object| after.contains(object)));
        assert_eq!(after.len(), before.len() + 1);
    }
}
