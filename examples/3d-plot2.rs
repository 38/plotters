use plotters::prelude::*;
use std::f64::consts::PI;
fn pdf(x: f64, y: f64) -> f64 {
    // see https://mathworld.wolfram.com/BivariateNormalDistribution.html
    // assumes x_bar = 0 and y_bar = 0
    const SDX: f64 = 0.3;
    const SDY: f64 = 0.3;
    const vx: f64 = SDX * SDX;
    const vy : f64 = SDY * SDY;
    const sdx_sdy: f64 = SDX * SDY;
    const RHO : f64 = 0.9;
    const c1: f64 = 1.0 - RHO * RHO;
    let c2 : f64 = c1.sqrt();
    let A: f64 = 1.0 / (2.0 * PI * sdx_sdy * c2);
    let Z: f64 = x * x / vx - 2.0 * RHO * x * y / sdx_sdy  + y * y / vy; 
    let res = A * (-Z / 2.0 / c1).exp();
    res
}

const OUT_FILE_NAME: &'static str = "plotters-doc-data/3d-plot2.gif";
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::gif(OUT_FILE_NAME, (600, 400), 100)?.into_drawing_area();

    for pitch in 0..157 {
        root.fill(&WHITE)?;

        let mut chart = ChartBuilder::on(&root)
            .caption("2D Guassian PDF", ("sans-serif", 20))
            .build_cartesian_3d(-1.0..1.0, 0.0..3.0, -1.0..1.0)?;
        chart.with_projection(|mut p| {
            p.pitch = 1.57 - (1.57 - pitch as f64 / 50.0).abs();
            p.scale = 0.7;
            p.into_matrix() // build the projection matrix
        });

        chart.configure_axes().draw()?;

        chart.draw_series(
            SurfaceSeries::xoz(
                (-30..=30).map(|x| x as f64 / 30.0),
                (-30..=30).map(|x| x as f64 / 30.0),
                pdf,
            )
            .style_func(&|&v| {
                (&HSLColor(240.0 / 360.0 - 240.0 / 360.0 * v / 5.0, 1.0, 0.7)).into()
            }),
        )?;

        root.present()?;
    }

    // To avoid the IO failure being ignored silently, we manually call the present function
    root.present().expect("Unable to write result to file, please make sure 'plotters-doc-data' dir exists under current dir");
    println!("Result has been saved to {}", OUT_FILE_NAME);

    Ok(())
}
#[test]
fn entry_point() {
    main().unwrap()
}
