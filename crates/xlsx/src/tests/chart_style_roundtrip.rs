//! Synthetic DrawingML only: chart style ID default, legacy 17..=48 preservation, and round trips.

use crate::{chart::read_chart, read_xlsx, write_xlsx, xml::parse};
use std::io::Read;

/// Builds a minimal chart part. `<c:style>` is a child of `<c:chartSpace>` and must precede
/// `<c:chart>` (CT_ChartSpace order: roundedCorners, style, clrMapOvr, pivotSource, protection,
/// chart), which is where [`crate::chart::write_chart`] emits it.
fn chart(style: &str) -> gridcraft_model::Chart {
    let xml = format!(
        "<c:chartSpace xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\">\
        <c:roundedCorners val=\"0\"/>\
        {style}\
        <c:chart>\
        <c:plotArea><c:barChart><c:barDir val=\"col\"/><c:grouping val=\"clustered\"/><c:ser>\
        <c:idx val=\"0\"/><c:order val=\"0\"/><c:val><c:numLit><c:formatCode>General</c:formatCode>\
        <c:ptCount val=\"1\"/><c:pt idx=\"0\"><c:v>12</c:v></c:pt></c:numLit></c:val></c:ser>\
        <c:axId val=\"1\"/><c:axId val=\"2\"/></c:barChart><c:catAx/><c:valAx/></c:plotArea></c:chart>\
        </c:chartSpace>"
    );
    read_chart(&parse(xml.as_bytes()).unwrap()).unwrap_or_else(|| panic!("failed to read {style:?}"))
}

#[test]
fn missing_style_default_is_zero() {
    // No `<c:style>` at all, and the empty element, both mean the polished default.
    assert_eq!(chart("").style, gridcraft_model::STYLE_DEFAULT);
    assert_eq!(chart("<c:style/>").style, gridcraft_model::STYLE_DEFAULT);
}

#[test]
fn zero_and_one_preserved() {
    assert_eq!(chart("<c:style val=\"0\"/>").style, 0);
    assert_eq!(chart("<c:style val=\"1\"/>").style, gridcraft_model::STYLE_POLISHED);
    // The style element must be read from `<c:chartSpace>`, not from inside `<c:chart>`:
    // nesting it there is a different document and must not be picked up.
    let nested = "<c:chartSpace xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\">\
        <c:chart><c:style val=\"1\"/>\
        <c:plotArea><c:barChart><c:barDir val=\"col\"/><c:grouping val=\"clustered\"/></c:barChart>\
        <c:catAx/><c:valAx/></c:plotArea></c:chart></c:chartSpace>";
    assert_eq!(
        read_chart(&parse(nested.as_bytes()).unwrap()).unwrap().style,
        gridcraft_model::STYLE_DEFAULT,
        "a <c:style> inside <c:chart> must not be read as the chart style"
    );
}

#[test]
fn legacy_17_to_48_preserved_round_trip() {
    // Legacy XLSX chart style IDs 17..=48 must survive write/reopen unchanged.
    for id in [17, 20, 42, 48] {
        let c = chart(&format!("<c:style val=\"{id}\"/>"));
        assert_eq!(c.style, id, "import parse of {id} failed");
        let mut wb = gridcraft_model::Workbook::new();
        wb.sheet_mut(0).unwrap().charts.push(c);
        let bytes = write_xlsx(&wb).unwrap();
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes.clone())).unwrap();
        let mut chart_xml = String::new();
        z.by_name("xl/charts/chart1.xml").unwrap().read_to_string(&mut chart_xml).unwrap();
        let want = format!("<c:style val=\"{id}\"/>");
        assert!(chart_xml.contains(&want), "export missing for style {id}; chart_xml:\n{chart_xml}");
        let (back, _) = read_xlsx(&bytes).unwrap();
        assert_eq!(back.sheets[0].charts[0].style, id, "style {id} did not round-trip");
    }
}

#[test]
fn export_writes_zero_for_default() {
    // Chart.style == 0 must serialize too, so the polished default round-trips losslessly.
    let mut wb = gridcraft_model::Workbook::new();
    let mut c = chart("");
    c.style = gridcraft_model::STYLE_DEFAULT;
    c.id = 1;
    wb.sheet_mut(0).unwrap().charts.push(c);
    let bytes = write_xlsx(&wb).unwrap();
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut chart_xml = String::new();
    z.by_name("xl/charts/chart1.xml").unwrap().read_to_string(&mut chart_xml).unwrap();
    assert!(chart_xml.contains("<c:style val=\"0\"/>"), "style=0 should serialize; chart_xml:\n{chart_xml}");
}

#[test]
fn style_element_precedes_chart_element() {
    // Excel repairs a chart part whose `<c:style>` comes after `<c:chart>`.
    let mut wb = gridcraft_model::Workbook::new();
    let mut c = chart("<c:style val=\"1\"/>");
    c.id = 1;
    wb.sheet_mut(0).unwrap().charts.push(c);
    let bytes = write_xlsx(&wb).unwrap();
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut chart_xml = String::new();
    z.by_name("xl/charts/chart1.xml").unwrap().read_to_string(&mut chart_xml).unwrap();
    let style_at = chart_xml.find("<c:style").expect("no <c:style>");
    let chart_at = chart_xml.find("<c:chart>").expect("no <c:chart>");
    assert!(style_at < chart_at, "<c:style> must precede <c:chart>; chart_xml:\n{chart_xml}");
}
