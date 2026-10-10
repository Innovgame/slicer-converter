use std::collections::HashMap;

use crate::{
    common::ConvertError,
    formats::{common::binary::BinaryReader, LayerDecodeContext},
};

#[derive(Debug, Clone, Copy)]
struct PwszPoint {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Copy)]
struct PwszSegment {
    start: PwszPoint,
    end: PwszPoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PwszPointKey {
    x: i32,
    y: i32,
}

fn pwsz_decode_error(offset: usize, detail: impl Into<String>) -> ConvertError {
    ConvertError::RleDecodeError {
        offset,
        detail: detail.into(),
    }
}

fn require_pwsz_marker(
    reader: &mut BinaryReader<'_>,
    marker: &[u8],
    label: &str,
) -> Result<(), ConvertError> {
    let offset = reader.position();
    let value = reader
        .read_range(marker.len())
        .map_err(|_| pwsz_decode_error(offset, format!("truncated pwszImg {label} marker")))?;
    if value != marker {
        return Err(pwsz_decode_error(
            offset,
            format!("invalid pwszImg {label} marker"),
        ));
    }
    Ok(())
}

fn read_pwsz_u32(reader: &mut BinaryReader<'_>, label: &str) -> Result<u32, ConvertError> {
    let offset = reader.position();
    reader
        .read_le_u32()
        .map_err(|_| pwsz_decode_error(offset, format!("truncated pwszImg {label}")))
}

fn read_pwsz_f32(reader: &mut BinaryReader<'_>, label: &str) -> Result<f32, ConvertError> {
    let offset = reader.position();
    reader
        .read_le_f32()
        .map_err(|_| pwsz_decode_error(offset, format!("truncated pwszImg {label}")))
}

fn read_pwsz_u8(reader: &mut BinaryReader<'_>, label: &str) -> Result<u8, ConvertError> {
    let offset = reader.position();
    reader
        .read_u8()
        .map_err(|_| pwsz_decode_error(offset, format!("truncated pwszImg {label}")))
}

fn pwsz_display_to_pixel(x_mm: f32, y_mm: f32, ctx: &LayerDecodeContext) -> PwszPoint {
    PwszPoint {
        x: x_mm * ctx.width as f32 / ctx.display_width_mm,
        y: y_mm * ctx.height as f32 / ctx.display_height_mm,
    }
}

fn pwsz_points_close(left: PwszPoint, right: PwszPoint) -> bool {
    (left.x - right.x).abs() < 0.01 && (left.y - right.y).abs() < 0.01
}

fn pwsz_point_key(point: PwszPoint) -> PwszPointKey {
    PwszPointKey {
        x: (point.x * 100.0).round() as i32,
        y: (point.y * 100.0).round() as i32,
    }
}

fn flush_pwsz_polygon(current: &mut Vec<PwszPoint>, polygons: &mut Vec<Vec<PwszPoint>>) {
    if current.len() >= 4
        && pwsz_points_close(
            *current.first().expect("current polygon has first point"),
            *current.last().expect("current polygon has last point"),
        )
    {
        current.pop();
        polygons.push(std::mem::take(current));
    } else {
        current.clear();
    }
}

fn push_pwsz_edge(
    edges: &mut HashMap<PwszPointKey, Vec<(PwszPoint, PwszPoint)>>,
    start: PwszPoint,
    end: PwszPoint,
) {
    edges
        .entry(pwsz_point_key(start))
        .or_default()
        .push((start, end));
}

fn remove_pwsz_edge(
    edges: &mut HashMap<PwszPointKey, Vec<(PwszPoint, PwszPoint)>>,
    start: PwszPoint,
    end: PwszPoint,
) {
    let key = pwsz_point_key(start);
    let should_remove = if let Some(bucket) = edges.get_mut(&key) {
        if let Some(index) = bucket.iter().position(|&(edge_start, edge_end)| {
            pwsz_points_close(edge_start, start) && pwsz_points_close(edge_end, end)
        }) {
            bucket.swap_remove(index);
        }
        bucket.is_empty()
    } else {
        false
    };
    if should_remove {
        edges.remove(&key);
    }
}

fn pop_pwsz_edge_from(
    edges: &mut HashMap<PwszPointKey, Vec<(PwszPoint, PwszPoint)>>,
    start: PwszPoint,
) -> Option<(PwszPoint, PwszPoint)> {
    let key = pwsz_point_key(start);
    let (edge_start, edge_end, should_remove) = {
        let bucket = edges.get_mut(&key)?;
        let (edge_start, edge_end) = bucket.pop()?;
        (edge_start, edge_end, bucket.is_empty())
    };
    if should_remove {
        edges.remove(&key);
    }
    remove_pwsz_edge(edges, edge_end, edge_start);
    Some((edge_start, edge_end))
}

fn pop_any_pwsz_edge(
    edges: &mut HashMap<PwszPointKey, Vec<(PwszPoint, PwszPoint)>>,
) -> Option<(PwszPoint, PwszPoint)> {
    let key = edges
        .iter()
        .find_map(|(key, bucket)| (!bucket.is_empty()).then_some(*key))?;
    let (edge_start, edge_end, should_remove) = {
        let bucket = edges.get_mut(&key)?;
        let (edge_start, edge_end) = bucket.pop()?;
        (edge_start, edge_end, bucket.is_empty())
    };
    if should_remove {
        edges.remove(&key);
    }
    remove_pwsz_edge(edges, edge_end, edge_start);
    Some((edge_start, edge_end))
}

fn pwsz_segments_to_polygons(segments: &[PwszSegment]) -> Vec<Vec<PwszPoint>> {
    let mut polygons = Vec::new();
    let mut edges = HashMap::with_capacity(segments.len().saturating_mul(2));
    for segment in segments {
        push_pwsz_edge(&mut edges, segment.start, segment.end);
        push_pwsz_edge(&mut edges, segment.end, segment.start);
    }

    while let Some((start, end)) = pop_any_pwsz_edge(&mut edges) {
        let mut current = vec![start, end];

        while !pwsz_points_close(
            *current.first().expect("current polygon has first point"),
            *current.last().expect("current polygon has last point"),
        ) {
            let Some((_, next)) = pop_pwsz_edge_from(
                &mut edges,
                *current.last().expect("current polygon has last point"),
            ) else {
                break;
            };
            current.push(next);
        }

        flush_pwsz_polygon(&mut current, &mut polygons);
    }
    polygons
}

fn pwsz_point_in_polygon(point: PwszPoint, polygon: &[PwszPoint]) -> bool {
    if polygon.len() < 3 {
        return false;
    }

    let mut inside = false;
    let mut previous = polygon.len() - 1;
    for current in 0..polygon.len() {
        let current_point = polygon[current];
        let previous_point = polygon[previous];
        if (current_point.y > point.y) != (previous_point.y > point.y) {
            let x_intersection = (previous_point.x - current_point.x) * (point.y - current_point.y)
                / (previous_point.y - current_point.y)
                + current_point.x;
            if point.x < x_intersection {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
}

fn pwsz_polygon_contains_polygon(container: &[PwszPoint], polygon: &[PwszPoint]) -> bool {
    polygon
        .iter()
        .all(|&point| pwsz_point_in_polygon(point, container))
}

fn pwsz_polygon_depth(index: usize, polygons: &[Vec<PwszPoint>]) -> usize {
    polygons
        .iter()
        .enumerate()
        .filter(|(candidate_index, candidate)| {
            *candidate_index != index && pwsz_polygon_contains_polygon(candidate, &polygons[index])
        })
        .count()
}

fn paint_pwsz_polygon(
    output: &mut [u8],
    width: u32,
    height: u32,
    polygon: &[PwszPoint],
    value: u8,
) {
    if polygon.len() < 3 {
        return;
    }

    let mut intersections = Vec::new();
    let min_y = polygon
        .iter()
        .map(|point| point.y)
        .fold(f32::INFINITY, f32::min)
        .floor()
        .max(0.0) as u32;
    let max_y = polygon
        .iter()
        .map(|point| point.y)
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .min(height as f32) as u32;

    for y in min_y..max_y {
        let sample_y = y as f32 + 0.5;
        intersections.clear();

        let mut previous = polygon.len() - 1;
        for current in 0..polygon.len() {
            let current_point = polygon[current];
            let previous_point = polygon[previous];
            if (current_point.y > sample_y) != (previous_point.y > sample_y) {
                let x_intersection = (previous_point.x - current_point.x)
                    * (sample_y - current_point.y)
                    / (previous_point.y - current_point.y)
                    + current_point.x;
                intersections.push(x_intersection);
            }
            previous = current;
        }

        intersections
            .sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));

        for pair in intersections.chunks_exact(2) {
            let start_x = (pair[0] - 0.5).ceil().max(0.0) as u32;
            let end_x = (pair[1] - 0.5).ceil().min(width as f32) as u32;
            for x in start_x..end_x {
                output[(y as usize * width as usize) + x as usize] = value;
            }
        }
    }
}

fn fill_pwsz_polygons_by_contour_tree(
    output: &mut [u8],
    width: u32,
    height: u32,
    polygons: &[Vec<PwszPoint>],
) {
    let mut indexed_depths = polygons
        .iter()
        .enumerate()
        .filter(|(_, polygon)| polygon.len() >= 3)
        .map(|(index, _)| (index, pwsz_polygon_depth(index, polygons)))
        .collect::<Vec<_>>();

    indexed_depths.sort_by_key(|&(_, depth)| depth);
    for (index, depth) in indexed_depths {
        let value = if depth % 2 == 0 { 255 } else { 0 };
        paint_pwsz_polygon(output, width, height, &polygons[index], value);
    }
}

fn fill_pwsz_single_points(output: &mut [u8], width: u32, height: u32, points: &[PwszPoint]) {
    for point in points {
        if point.x >= 0.0 && point.x < width as f32 && point.y >= 0.0 && point.y < height as f32 {
            let x = point.x as usize;
            let y = point.y as usize;
            output[(y * width as usize) + x] = 255;
        }
    }
}

pub(super) fn decode_pwsz_contours_into(
    data: &[u8],
    ctx: &LayerDecodeContext,
    output: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    if ctx.width == 0 || ctx.height == 0 {
        return Err(pwsz_decode_error(
            0,
            "pwszImg decode requires non-zero pixel resolution",
        ));
    }
    if ctx.display_width_mm <= 0.0 || ctx.display_height_mm <= 0.0 {
        return Err(pwsz_decode_error(
            0,
            "pwszImg decode requires display size in millimeters",
        ));
    }

    let mut reader = BinaryReader::new(data);
    require_pwsz_marker(&mut reader, b"{==\0", "file start")?;
    let _area = read_pwsz_f32(&mut reader, "area")?;
    let _x_min = read_pwsz_f32(&mut reader, "x min")?;
    let _y_min = read_pwsz_f32(&mut reader, "y min")?;
    let _x_max = read_pwsz_f32(&mut reader, "x max")?;
    let _y_max = read_pwsz_f32(&mut reader, "y max")?;
    let _padding = read_pwsz_u32(&mut reader, "padding")?;
    let _object_count = read_pwsz_u32(&mut reader, "object count")?;
    require_pwsz_marker(&mut reader, b"[--\0", "coordinates start")?;
    let _padding = read_pwsz_u32(&mut reader, "coordinates padding")?;
    let line_count = read_pwsz_u32(&mut reader, "line count")?;
    let _unknown = read_pwsz_u32(&mut reader, "coordinates unknown")?;

    let record_bytes = (line_count as usize).checked_mul(17).ok_or_else(|| {
        pwsz_decode_error(reader.position(), "pwszImg line count overflows usize")
    })?;
    if data.len().saturating_sub(reader.position()) < record_bytes + 8 {
        return Err(pwsz_decode_error(
            reader.position(),
            "truncated pwszImg line records",
        ));
    }

    let half_display_x = ctx.display_width_mm / 2.0;
    let half_display_y = ctx.display_height_mm / 2.0;
    let mut segments = Vec::with_capacity(line_count as usize);
    let mut single_points = Vec::new();
    for _ in 0..line_count {
        let start_x = read_pwsz_f32(&mut reader, "line start x")? + half_display_x;
        let start_y = read_pwsz_f32(&mut reader, "line start y")? + half_display_y;
        let end_x = read_pwsz_f32(&mut reader, "line end x")? + half_display_x;
        let end_y = read_pwsz_f32(&mut reader, "line end y")? + half_display_y;
        let _cw = read_pwsz_u8(&mut reader, "line winding")?;
        let segment = PwszSegment {
            start: pwsz_display_to_pixel(start_x, start_y, ctx),
            end: pwsz_display_to_pixel(end_x, end_y, ctx),
        };
        if pwsz_points_close(segment.start, segment.end) {
            single_points.push(segment.start);
        } else {
            segments.push(segment);
        }
    }

    require_pwsz_marker(&mut reader, b"--]\0", "coordinates end")?;
    require_pwsz_marker(&mut reader, b"==}\0", "file end")?;

    let expected = (ctx.width as usize).saturating_mul(ctx.height as usize);
    output.clear();
    output.resize(expected, 0);
    let polygons = pwsz_segments_to_polygons(&segments);
    fill_pwsz_polygons_by_contour_tree(output, ctx.width, ctx.height, &polygons);
    fill_pwsz_single_points(output, ctx.width, ctx.height, &single_points);

    Ok(())
}
