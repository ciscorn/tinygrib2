use std::collections::hash_map::Entry;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

use i_overlay::{
    core::{
        fill_rule::FillRule,
        overlay::{ContourDirection, Overlay},
        overlay_rule::OverlayRule,
    },
    i_float::int::point::IntPoint,
    i_shape::int::shape::IntContour,
};
use itertools::Itertools;
use japanmesh::gridsquare::{LngLat, LngLatBox};
use prost::Message;
use scoa::{
    delta::{delta_decode, delta_encode},
    sfcurve::{hilbert_to_xy, xy_to_hilbert},
    ScoaReader,
};
use tinygrib::{
    message::DataRepresentationSectionHeader,
    templates::{
        read_data_7_200, DataRepresentationTemplate5_200, GridDefinitionTemplate3_0,
        ProductDefinitionTemplate4_0, ProductDefinitionTemplate4_50011,
        ProductDefinitionTemplate4_8,
    },
    MessageReader,
};
use tinymvt::{
    vector_tile::tile::Layer,
    webmercator::{lnglat_to_web_mercator, web_mercator_to_lnglat},
};

type IndexMap<K, V> = indexmap::IndexMap<K, V, foldhash::fast::RandomState>;
type HashMap<K, V> = std::collections::HashMap<K, V, foldhash::fast::RandomState>;

enum ProductDefinitionTemplates {
    Template0(ProductDefinitionTemplate4_0),
    Template8(ProductDefinitionTemplate4_8),
    Template50011(ProductDefinitionTemplate4_50011),
}

#[derive(Default)]
struct GridSquareMessageReader {
    gds_template: Option<GridDefinitionTemplate3_0>,
    drs: Option<DataRepresentationSectionHeader>,
    drs_template: Option<DataRepresentationTemplate5_200>,
    pds_template: Option<ProductDefinitionTemplates>,
    non_missing_points: u32,
    total_grid_points: u32,
    tiles: IndexMap<u64, Vec<TileEntry>>,
    base_z: u8,
}

#[derive(Default, Debug, bincode::Encode, bincode::Decode)]
struct Tile {
    pub min_x: u32,
    pub min_y: u32,
    pub point_ids: Vec<u64>,
    pub values: Vec<i32>,
    /// 0 = 250m, 2 = 1km
    pub scales: Vec<i8>,
}

#[derive(Debug)]
struct TileEntry {
    pub x: u32,
    pub y: u32,
    pub value: u16,
    /// 0 = 250m, 2 = 1km
    pub scale: i8,
}

#[derive(Debug, bincode::Encode, bincode::Decode)]
struct UserData {
    base_z: u8,
    grid: LngLatGrid,
}

#[derive(Debug, bincode::Encode, bincode::Decode)]
struct LngLatGrid {
    lng_0: f64,
    lat_0: f64,
    lng_denom: u32,
    lat_denom: u32,
}

impl<R: Read> MessageReader<R> for GridSquareMessageReader {
    fn handle_grid_definition(
        &mut self,
        gds: tinygrib::message::GridDefinitionSectionHeader,
        reader: &mut io::Take<&mut R>,
    ) -> tinygrib::Result<()> {
        assert_eq!(gds.template_number, 0);
        self.gds_template = Some(GridDefinitionTemplate3_0::read(reader)?);
        Ok(())
    }

    fn handle_data_representation(
        &mut self,
        drs: tinygrib::message::DataRepresentationSectionHeader,
        reader: &mut io::Take<&mut R>,
    ) -> tinygrib::Result<()> {
        assert_eq!(drs.template_number, 200);
        self.drs = Some(drs);
        self.drs_template = Some(DataRepresentationTemplate5_200::read(reader)?);
        Ok(())
    }

    fn handle_product_definition(
        &mut self,
        pds: tinygrib::message::ProductDefinitionSectionHeader,
        reader: &mut io::Take<&mut R>,
    ) -> tinygrib::Result<()> {
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
                self.pds_template = Some(ProductDefinitionTemplates::Template50011(
                    ProductDefinitionTemplate4_50011::read(reader)?,
                ));
            }
            _ => unreachable!("{:?}", pds.template_number),
        }
        Ok(())
    }

    fn handle_data(
        &mut self,
        data: tinygrib::message::DataSectionHeader,
        reader: &mut io::Take<&mut R>,
    ) -> tinygrib::Result<()> {
        let product_kind = match &self.pds_template.as_ref().unwrap() {
            ProductDefinitionTemplates::Template0(tmpl) => {
                match (tmpl.parameter_category, tmpl.parameter_number) {
                    (0, 0) => "Temperature",
                    _ => todo!(),
                }
            }
            ProductDefinitionTemplates::Template8(tmpl) => {
                match (
                    tmpl.parameter_category,
                    tmpl.parameter_number,
                    tmpl.time_ranges[0].statistical_process,
                ) {
                    (191, 192, 196) => "Weather",
                    (1, 204, 1) => "Total precipitation",
                    (1, 233, 1) => "Snow depth",
                    (0, 0, 0) => "Temperature",
                    (0, 0, 3) => "Min temperature",
                    (0, 0, 2) => "Max temperature",
                    _ => unimplemented!("{:?}", tmpl),
                }
            }
            ProductDefinitionTemplates::Template50011(ext_template) => {
                let tmpl = &ext_template.template_8;
                // if tmpl.type_of_generating_process == 0 {
                //     println!("{}", tmpl.forecast_time);
                // }
                match (
                    tmpl.parameter_category,
                    tmpl.parameter_number,
                    tmpl.type_of_generating_process,
                    tmpl.background_generating_process_identifier,
                ) {
                    (1, 8, 0, 151) => "High-Res Nowcast Intensity Analysis (5-min)",
                    (1, 8, 2, 151) => "High-Res Nowcast Intensity Forecast (5-min)",
                    (1, 203, 0, 151) => "High-Res Nowcast Intensity Analysis",
                    (1, 203, 2, 151) => "High-Res Nowcast Intensity Forecast",
                    (1, 214, 0, 151) => "High-Res Nowcast Error",
                    _ => unimplemented!("{:?}", tmpl),
                }
                // println!("product_kind: {} {}", kind, tmpl.forecast_time);
            }
        };

        // let scale = 10.0_f64.powi(-self.drs_template.as_ref().unwrap().decimal_scale_factor as i32);

        match product_kind {
            //"Temperature"
            "High-Res Nowcast Intensity Analysis"
            | "High-Res Nowcast Intensity Analysis (5-min)" => {
                let values = read_data_7_200(
                    reader,
                    data.body_len() as usize,
                    self.drs.as_ref().unwrap(),
                    self.drs_template.as_ref().unwrap(),
                )?;
                let tmpl = self.gds_template.as_ref().unwrap();
                assert_eq!(values.len(), tmpl.n_i as usize * tmpl.n_j as usize);

                let (x_first, y_last, scale) = if tmpl.d_i == 12500 {
                    let x_first = ((tmpl.lo1 - 1_000_000 * 100) / 12500) * 4;
                    let y_last = (((tmpl.la1 + 1) as f64 / 8333.33333333) as i32) * 4;
                    (x_first as u32, y_last as u32, 2)
                } else if tmpl.d_i == 3125 {
                    let x_first = (tmpl.lo1 - 1_000_000 * 100) / 3125;
                    let y_last = ((tmpl.la1 + 1) as f64 / 2083.33333333) as i32;
                    (x_first as u32, y_last as u32, 0)
                } else {
                    unimplemented!("Unsupported d_i: {}", tmpl.d_i);
                };
                let width = 1 << scale;

                let mut non_missing = 0;

                for j in 0..tmpl.n_j {
                    for i in 0..tmpl.n_i {
                        let idx = j as usize * tmpl.n_i as usize + i as usize;
                        let value = match values[idx] {
                            Some(0) => continue,
                            Some(v) => v,
                            None => continue,
                        };

                        let (gx, gy, x1, x2, y1, y2) = {
                            let gx = x_first + i * width;
                            let gy = y_last - j * width;
                            let lng1 = 100.0 + (gx as f64 / 320.);
                            let lng2 = 100.0 + ((gx + width) as f64 / 320.);
                            let lat1 = (gy + width) as f64 / 480.;
                            let lat2 = gy as f64 / 480.;
                            let (mx1, my1) = lnglat_to_web_mercator(lng1, lat1);
                            let (mx2, my2) = lnglat_to_web_mercator(lng2, lat2);

                            fn web_mercator_to_zxy(base_z: u8, mx: f64, my: f64) -> (u8, u32, u32) {
                                let z = base_z;
                                let x = (mx * (1 << z) as f64) as u32;
                                let y = (my * (1 << z) as f64) as u32;
                                (z, x, y)
                            }

                            let buffer = 64. / 4096. / (1 << self.base_z) as f64;
                            let (_, x1, y1) =
                                web_mercator_to_zxy(self.base_z, mx1 - buffer, my1 - buffer);
                            let (_, x2, y2) =
                                web_mercator_to_zxy(self.base_z, mx2 + buffer, my2 + buffer);
                            (gx, gy, x1, x2, y1, y2)
                        };
                        assert!(x1 <= x2 && y1 <= y2);
                        for y in y1..=y2 {
                            for x in x1..=x2 {
                                let tile_id = xy_to_hilbert(self.base_z, x, y);
                                let tile = self.tiles.entry(tile_id).or_default();
                                tile.push(TileEntry {
                                    x: gx,
                                    y: gy,
                                    value,
                                    scale,
                                });
                            }
                        }

                        non_missing += 1;
                    }
                }
                self.non_missing_points += non_missing;
                self.total_grid_points += tmpl.n_i * tmpl.n_j;
            }
            _ => {}
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let filename = std::env::args().nth(1).expect("filename is required");

    let file = fs::File::open(filename)?;
    let mut reader = BufReader::new(file);

    let base_z = 9;

    // let mut num_messages = 0;
    let mut msg_reader = GridSquareMessageReader {
        base_z,
        ..Default::default()
    };
    while let Some(()) = msg_reader.read_next_message(&mut reader)? {
        // num_messages += 1;
    }
    eprintln!(
        "points filled: {:.1}% ({} of {})",
        msg_reader.non_missing_points as f64 / msg_reader.total_grid_points as f64 * 100.0,
        msg_reader.non_missing_points,
        msg_reader.total_grid_points
    );
    // println!("num_messages: {}", num_messages);

    let mut chunk_ids: Vec<u32> = vec![];
    let mut end_positions: Vec<u32> = vec![];
    let mut body: Vec<u8> = vec![];

    msg_reader.tiles.sort_unstable_keys();

    for (tile_id, pre_tile) in msg_reader.tiles {
        let (min_x, min_y) = pre_tile
            .iter()
            .map(|tile| (tile.x, tile.y))
            .fold((u32::MAX, u32::MAX), |(min_x, min_y), (gx, gy)| {
                (min_x.min(gx), min_y.min(gy))
            });

        let mut pre_tile = pre_tile
            .into_iter()
            .map(|point| {
                let point_id = xy_to_hilbert(32, point.x - min_x, point.y - min_y);
                (point_id, point)
            })
            .collect::<Vec<_>>();
        pre_tile.sort_unstable_by_key(|&(point_id, _)| point_id);

        let tile = Tile {
            min_x,
            min_y,
            point_ids: delta_encode(pre_tile.iter().map(|&(id, _)| id), 1).collect(),
            values: delta_encode(pre_tile.iter().map(|(_, entry)| entry.value as i32), 0).collect(),
            scales: delta_encode(pre_tile.iter().map(|(_, entry)| entry.scale), 0).collect(),
        };
        let compressed_data =
            compress_gzip(&bincode::encode_to_vec(tile, bincode::config::standard()).unwrap())?;

        body.extend(compressed_data);
        chunk_ids.push(tile_id as u32);
        end_positions.push(body.len() as u32);
    }

    {
        let mut writer = BufWriter::new(fs::File::create("demo.scoa")?);
        let user_data = bincode::encode_to_vec(
            UserData {
                base_z,
                grid: LngLatGrid {
                    lng_0: 100.,
                    lat_0: 0.,
                    lng_denom: 320,
                    lat_denom: 480,
                },
            },
            bincode::config::standard(),
        )
        .unwrap();
        scoa::write_header(
            &mut writer,
            chunk_ids.len() as u32,
            chunk_ids.iter().map(|&v| v as u64),
            end_positions.iter().map(|&v| v as u64),
            user_data,
        )?;
        writer.write_all(&body)?;
    }

    let output_dir = Path::new("output");
    read_and_dump(Path::new("demo.scoa"), output_dir, 0, 10)?;

    Ok(())
}

fn read_and_dump(
    filename: &Path,
    output_dir: &Path,
    min_zoom: u8,
    max_zoom: u8,
) -> Result<(), Box<dyn Error>> {
    let mut reader = BufReader::new(File::open(filename)?);

    let scoa: ScoaReader = {
        let mut buf = vec![0; 64 * 1024];
        let read_len = reader.read(&mut buf)?;
        ScoaReader::from_header_bytes(&buf[..read_len])?
    };
    let (user_data, _) =
        bincode::decode_from_slice::<UserData, _>(scoa.user_data(), bincode::config::standard())?;

    let mut stack: Vec<(u8, u32, u32)> = vec![(0, 0, 0)];
    while let Some(zxy) = stack.pop() {
        let (id_begin, id_end) = zxy_to_begin_end(user_data.base_z, zxy);
        let (z, x, y) = zxy;
        let Some(chunks) = scoa.bisect_range(id_begin, id_end) else {
            continue;
        };

        if z >= min_zoom {
            let tile_path = output_dir.join(format!("{}/{}/{}.pbf", z, x, y));
            fs::create_dir_all(tile_path.parent().unwrap())?;

            let buf = {
                reader.seek(SeekFrom::Start(chunks.body_begin() as u64))?;
                let mut buf = vec![0; chunks.body_size()];
                reader.read_exact(&mut buf)?;
                buf
            };

            if let Some(encoded_tile) = make_tile(zxy, chunks.iter_chunks(&buf), &user_data)? {
                let mut writer = {
                    let file = fs::File::create(tile_path)?;
                    // GzEncoder::new(BufWriter::new(file), Compression::new(4))
                    BufWriter::new(file)
                };
                writer.write_all(&encoded_tile)?;
                eprintln!(
                    "{z}/{x}/{y} -> {} bytes",
                    bytesize::ByteSize::b(encoded_tile.len() as u64)
                );
            } else {
                continue;
            }
        }

        if z < max_zoom {
            stack.push((z + 1, x * 2, y * 2));
            stack.push((z + 1, x * 2 + 1, y * 2));
            stack.push((z + 1, x * 2, y * 2 + 1));
            stack.push((z + 1, x * 2 + 1, y * 2 + 1));
        }
    }

    Ok(())
}

fn make_tile<'a>(
    zxy: (u8, u32, u32),
    chunk_iter: impl Iterator<Item = (u64, &'a [u8])>,
    user_data: &UserData,
) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    let (z, x, y) = zxy;
    let tile_mx1 = x as f64 / (1 << z) as f64;
    let tile_my1 = y as f64 / (1 << z) as f64;
    let tile_mx2 = (x + 1) as f64 / (1 << z) as f64;
    let tile_my2 = (y + 1) as f64 / (1 << z) as f64;
    let tile_box = {
        let buffer = (tile_mx2 - tile_mx1) * 64. / 4096.;
        let (tile_lng1, tile_lat1) = web_mercator_to_lnglat(tile_mx1 - buffer, tile_my2 + buffer);
        let (tile_lng2, tile_lat2) = web_mercator_to_lnglat(tile_mx2 + buffer, tile_my1 - buffer);
        LngLatBox::new(
            LngLat::new(tile_lng1, tile_lat1),
            LngLat::new(tile_lng2, tile_lat2),
        )
    };

    struct Value {
        value: i32,
        width: u16,
    }

    let mut collected_points: HashMap<(u32, u32), Value> = HashMap::default();

    for (_chunk_id, raw_chunk) in chunk_iter {
        let (mut tile, _) = bincode::decode_from_slice::<Tile, _>(
            &decompress_gzip(raw_chunk)?,
            bincode::config::standard(),
        )?;
        tile.point_ids = delta_decode(tile.point_ids.iter().copied(), 1).collect();
        tile.values = delta_decode(tile.values.iter().copied(), 0).collect();
        tile.scales = delta_decode(tile.scales.iter().copied(), 0).collect();

        let maximum_detail_zoom = 9;
        let agg_width = if z < maximum_detail_zoom {
            1u32 << (maximum_detail_zoom - z)
        } else {
            1u32
        };

        for ((x, y, scale), group) in tile
            .point_ids
            .into_iter()
            .zip_eq(tile.values)
            .zip_eq(tile.scales)
            .chunk_by(|&((point_id, _), scale)| {
                let (dx, dy) = hilbert_to_xy(32, point_id);
                let (x, y) = (tile.min_x + dx, tile.min_y + dy);
                (x - x % agg_width, y - y % agg_width, scale)
            })
            .into_iter()
        {
            let width: u32 = ((1 << scale) as u32).max(agg_width);
            let lng1 = user_data.grid.lng_0 + (x as f64 / user_data.grid.lng_denom as f64);
            let lat1 = user_data.grid.lat_0 + (y + width) as f64 / user_data.grid.lat_denom as f64;
            let lng2 =
                user_data.grid.lng_0 + ((x + width) as f64 / user_data.grid.lng_denom as f64);
            let lat2 = user_data.grid.lat_0 + y as f64 / user_data.grid.lat_denom as f64;
            let point_box = LngLatBox::new(LngLat::new(lng1, lat1), LngLat::new(lng2, lat2));
            if !tile_box.intersects_box(&point_box) {
                continue;
            }

            let value = group
                .map(|((_, value), _)| value)
                .fold(0, |acc, v| acc.max(v));

            match collected_points.entry((x, y)) {
                Entry::Occupied(mut entry) => {
                    let point = entry.get_mut();
                    point.value = point.value.max(value);
                    point.width = point.width.max(width as u16);
                }
                Entry::Vacant(entry) => {
                    entry.insert(Value {
                        value,
                        width: width as u16,
                    });
                }
            }
        }
    }

    if collected_points.is_empty() {
        return Ok(None);
    }

    let mut value_mpoly_map: HashMap<i32, Vec<IntContour>> = HashMap::default();

    for ((x, y), point) in collected_points {
        let value = point.value;
        let width = point.width as u32;
        let lng1 = user_data.grid.lng_0 + (x as f64 / user_data.grid.lng_denom as f64);
        let lat1 = user_data.grid.lat_0 + (y + width) as f64 / user_data.grid.lat_denom as f64;
        let lng2 = user_data.grid.lng_0 + ((x + width) as f64 / user_data.grid.lng_denom as f64);
        let lat2 = user_data.grid.lat_0 + y as f64 / user_data.grid.lat_denom as f64;
        let (mx1, my1) = lnglat_to_web_mercator(lng1, lat1);
        let (mx2, my2) = lnglat_to_web_mercator(lng2, lat2);

        let tx1 = (((mx1 - tile_mx1) / (tile_mx2 - tile_mx1) * 4096.0 + 0.5) as i32)
            .clamp(-64, 4096 + 64);
        let ty1 = (((my1 - tile_my1) / (tile_my2 - tile_my1) * 4096.0 + 0.5) as i32)
            .clamp(-64, 4096 + 64);
        let tx2 = (((mx2 - tile_mx1) / (tile_mx2 - tile_mx1) * 4096.0 + 0.5) as i32)
            .clamp(-64, 4096 + 64);
        let ty2 = (((my2 - tile_my1) / (tile_my2 - tile_my1) * 4096.0 + 0.5) as i32)
            .clamp(-64, 4096 + 64);

        if !(tx1 < tx2 && ty1 < ty2) {
            continue;
        }

        value_mpoly_map.entry(value).or_default().push(vec![
            IntPoint { x: tx1, y: ty1 },
            IntPoint { x: tx2, y: ty1 },
            IntPoint { x: tx2, y: ty2 },
            IntPoint { x: tx1, y: ty2 },
        ]);
    }

    let mut features = vec![];
    let mut tags_enc = tinymvt::tag::TagsEncoder::new();

    for (value, contours) in value_mpoly_map {
        let mut geom_enc = tinymvt::geometry::GeometryEncoder::new();

        // Unary-union of polygons having same value
        let mpoly = Overlay::with_contours(&contours, &[]).overlay_custom(
            OverlayRule::Subject,
            FillRule::Positive,
            ContourDirection::Clockwise,
            0,
            Default::default(),
        );

        for poly in mpoly {
            for ring in poly {
                geom_enc.add_ring(ring.iter().map(|p| [p.x as i16, p.y as i16]));
            }
        }
        tags_enc.add("value", value);
        features.push(tinymvt::vector_tile::tile::Feature {
            id: Some(value as u64),
            tags: tags_enc.take_tags(),
            r#type: Some(tinymvt::vector_tile::tile::GeomType::Polygon as i32),
            geometry: geom_enc.into_vec(),
        });
    }

    // Layer
    let (keys, values) = tags_enc.into_keys_and_values();
    let layer = Layer {
        version: 2,
        name: "layer".to_string(),
        features,
        keys,
        values,
        extent: Some(4096),
    };

    // Tile
    let tile = tinymvt::vector_tile::Tile {
        layers: vec![layer],
    };

    // Encode as protobuf
    Ok(Some(tile.encode_to_vec()))
}

fn zxy_to_begin_end(base_z: u8, zxy: (u8, u32, u32)) -> (u64, u64) {
    let (z, x, y) = zxy;
    if z < base_z {
        let dz = 1 << (base_z - z);
        let id_begin = xy_to_hilbert(z, x, y) * dz * dz;
        let id_end = id_begin + dz * dz;
        (id_begin, id_end)
    } else {
        let id_begin = xy_to_hilbert(base_z, x >> (z - base_z), y >> (z - base_z));
        (id_begin, id_begin + 1)
    }
}

fn compress_gzip(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(5));
    encoder.write_all(data)?;
    encoder.finish()
}

fn decompress_gzip(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut decoder = flate2::read::GzDecoder::new(data);
    let mut decompressed_data = Vec::new();
    decoder.read_to_end(&mut decompressed_data)?;
    Ok(decompressed_data)
}
