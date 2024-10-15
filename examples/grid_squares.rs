use serde_json::json;
use tinygrib::message::DataRepresentationSectionHeader;
use tinygrib::templates::{
    read_data_7_200, DataRepresentationTemplate5_200, GridDefinitionTemplate3_0,
    ProductDefinitionTemplate4_0, ProductDefinitionTemplate4_8,
};
use tinygrib::{Error, MessageReader, Result};

use std::{
    fs,
    io::{BufRead, BufReader},
};

enum ProductDefinitionTemplates {
    Template0(ProductDefinitionTemplate4_0),
    Template8(ProductDefinitionTemplate4_8),
}

#[derive(Default)]
struct GridSquareMessageReader {
    gds_template: Option<GridDefinitionTemplate3_0>,
    drs: Option<DataRepresentationSectionHeader>,
    drs_template: Option<DataRepresentationTemplate5_200>,
    pds_template: Option<ProductDefinitionTemplates>,
}

impl<R: BufRead> MessageReader<R> for GridSquareMessageReader {
    fn handle_grid_definition(
        &mut self,
        gds: tinygrib::message::GridDefinitionSectionHeader,
        reader: &mut std::io::Take<&mut R>,
    ) -> Result<()> {
        assert_eq!(gds.template_number, 0);
        self.gds_template = Some(GridDefinitionTemplate3_0::read(reader)?);
        Ok(())
    }

    fn handle_data_representation(
        &mut self,
        drs: tinygrib::message::DataRepresentationSectionHeader,
        reader: &mut std::io::Take<&mut R>,
    ) -> Result<()> {
        assert_eq!(drs.template_number, 200);
        self.drs = Some(drs);
        self.drs_template = Some(DataRepresentationTemplate5_200::read(reader)?);
        Ok(())
    }

    fn handle_product_definition(
        &mut self,
        pds: tinygrib::message::ProductDefinitionSectionHeader,
        reader: &mut std::io::Take<&mut R>,
    ) -> Result<()> {
        match pds.template_number {
            0 => {
                self.pds_template = Some(ProductDefinitionTemplates::Template0(
                    ProductDefinitionTemplate4_0::read(reader)?,
                ));
            }
            8 => {
                self.pds_template = Some(ProductDefinitionTemplates::Template8(
                    ProductDefinitionTemplate4_8::read(reader)?,
                ));
            }
            50011 => {
                println!(
                    "Production definition template {} is not implemented yet.",
                    pds.template_number
                );
            }
            _ => unreachable!("{:?}", pds.template_number),
        }
        Ok(())
    }

    fn handle_data(
        &mut self,
        data: tinygrib::message::DataSectionHeader,
        reader: &mut std::io::Take<&mut R>,
    ) -> Result<()> {
        let product_kind = match &self.pds_template {
            Some(ProductDefinitionTemplates::Template0(template)) => {
                match (template.parameter_category, template.parameter_number) {
                    (0, 0) => "Temperature",
                    _ => todo!(),
                }
            }
            Some(ProductDefinitionTemplates::Template8(template)) => {
                match (
                    template.parameter_category,
                    template.parameter_number,
                    template.time_ranges[0].statistical_process,
                ) {
                    (191, 192, 196) => "Weather",
                    (1, 204, 1) => "Total precipitation",
                    (1, 233, 1) => "Snow depth",
                    (0, 0, 0) => "Temperature",
                    (0, 0, 3) => "Min temperature",
                    (0, 0, 2) => "Max temperature",
                    _ => unreachable!("{:?}", template),
                }
            }
            // None => unreachable!(),
            None => "todo",
        };

        if product_kind == "Temperature" {
            let values = read_data_7_200(
                reader,
                data.body_len() as usize,
                self.drs.as_ref().unwrap(),
                self.drs_template.as_ref().unwrap(),
            )?;
            let gds_template = self.gds_template.as_ref().unwrap();
            println!("{:#?}", gds_template);
            assert_eq!(
                values.len(),
                gds_template.ni as usize * gds_template.nj as usize
            );

            panic!();

            let template = gds_template;
            for j in 0..template.nj as usize {
                for i in 0..template.ni as usize {
                    let idx = j * template.ni as usize + i;
                    let v = values[idx];
                    let lng1 = (template.lo1 + i as u32 * template.di - template.di / 2) as f64
                        / 1_000_000.0;
                    let lat1 = (template.la1 - j as u32 * template.dj + template.dj / 2) as f64
                        / 1_000_000.0;
                    let lng2 = (template.lo1 + i as u32 * template.di + template.di / 2) as f64
                        / 1_000_000.0;
                    let lat2 = (template.la1 - j as u32 * template.dj - template.dj / 2) as f64
                        / 1_000_000.0;
                    if v.is_finite() {
                        let a = json!({
                            "type": "Feature",
                            "geometry": {
                                "type": "Polygon",
                                "coordinates": [[
                                    [lng1, lat1],
                                    [lng2, lat1],
                                    [lng2, lat2],
                                    [lng1, lat2],
                                    [lng1, lat1],
                                ]],
                            },
                            "properties": {
                                "temperature": v
                            }
                        });
                        println!("{},", serde_json::to_string(&a).unwrap());
                    }
                }
            }
        }
        Ok(())
    }
}

impl GridSquareMessageReader {
    fn new() -> Self {
        Self::default()
    }
}

fn main() -> Result<()> {
    let filename = std::env::args()
        .nth(1)
        .ok_or(Error::InvalidData("filename is required".to_string()))?;

    let file = fs::File::open(filename)?;

    let mut reader = BufReader::new(file);

    let mut num_messages = 0;
    let mut msg_reader = GridSquareMessageReader::new();
    while let Some(()) = msg_reader.read_next_message(&mut reader)? {
        num_messages += 1;
    }
    println!("num_messages: {}", num_messages);

    Ok(())
}
