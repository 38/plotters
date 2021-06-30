use plotters::prelude::*;

const OUT_FILE_NAME: &'static str = "plotters-doc-data/layout2.png";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const W: u32 = 600;
    const H: u32 = 400;
    let root = BitMapBackend::new(OUT_FILE_NAME, (W, H)).into_drawing_area();
    root.fill(&full_palette::WHITE)?;

    let x_spec = -3.1..3.01f32;
    let y_spec = -1.1..1.1f32;

    let mut chart = ChartLayout::new(&root);
    chart
        .set_chart_title_text("Chart Title")?
        .set_chart_title_style(("serif", 60.).into_font().with_color(&RED))?
        .set_left_label_text("Ratio of Sides")?
        .set_bottom_label_text("Radians")?
        .set_bottom_label_margin(10.)?
        .set_left_label_margin((0., -5., 0., 10.))?
        .build_cartesian_2d(x_spec.clone(), y_spec.clone())?
        .draw()?;

    // If we extract a drawing area corresponding to a chart area, we can
    // use the usual chart API to draw.
    let da_chart = chart.get_chart_drawing_area()?;
    let x_axis = x_spec.clone().step(0.1);
    let mut cc = ChartBuilder::on(&da_chart)
        //.margin(5)
        //.set_all_label_area_size(15)
        .build_cartesian_2d(x_spec.clone(), y_spec.clone())?;

    cc.draw_series(LineSeries::new(x_axis.values().map(|x| (x, x.sin())), &RED))?
        .label("Sine")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    cc.configure_series_labels().border_style(&BLACK).draw()?;
    root.present()?;

    Ok(())
}
#[test]
fn entry_point() {
    main().unwrap()
}
