use crate::common::{FileMetadata, FileMetadataBuilder};

fn parse_f32(value: &str) -> Option<f32> {
    value.parse::<f32>().ok().filter(|value| value.is_finite())
}

fn parse_u32(value: &str) -> Option<u32> {
    value.parse::<u32>().ok()
}

fn parse_u16(value: &str) -> Option<u16> {
    value.parse::<u16>().ok()
}

fn parse_print_time(value: &str) -> Option<u32> {
    parse_f32(value).and_then(|value| {
        (value >= 0.0)
            .then(|| value.round())
            .filter(|value| *value <= u32::MAX as f32)
            .map(|value| value as u32)
    })
}

fn default_gcode_metadata() -> FileMetadata {
    FileMetadataBuilder::new()
        .machine_name("Default")
        .layer_height_mm(0.05)
        .bottom_layer_count(4)
        .anti_alias_level(1)
        .exposure(7.0, 35.0)
        .lift(5.0, 60.0)
        .bottom_lift(5.0, 60.0)
        .retract(150.0)
        .light_pwm(255, 255)
        .build_unchecked()
}

pub(super) fn parse_gcode_metadata(gcode: &str) -> FileMetadata {
    let mut metadata = default_gcode_metadata();

    for line in gcode.lines().map(str::trim) {
        let Some(comment) = line.strip_prefix(';') else {
            continue;
        };
        let Some((key, value)) = comment.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();

        match key {
            "machineType" => metadata.printer.machine_name = value.to_string(),
            "estimatedPrintTime" => {
                if let Some(value) = parse_print_time(value) {
                    metadata.stats.print_time_sec = value;
                }
            }
            "volume" => {
                if let Some(value) = parse_f32(value) {
                    metadata.stats.volume_ml = value;
                }
            }
            "weight" => {
                if let Some(value) = parse_f32(value) {
                    metadata.stats.weight_g = value;
                }
            }
            "price" => {
                if let Some(value) = parse_f32(value) {
                    metadata.stats.cost = value;
                }
            }
            "layerHeight" => {
                if let Some(value) = parse_f32(value) {
                    metadata.slicing.layer_height_mm = value;
                }
            }
            "resolutionX" => {
                if let Some(value) = parse_u32(value) {
                    metadata.printer.resolution_x = value;
                }
            }
            "resolutionY" => {
                if let Some(value) = parse_u32(value) {
                    metadata.printer.resolution_y = value;
                }
            }
            "machineX" => {
                if let Some(value) = parse_f32(value) {
                    metadata.printer.display_width_mm = value;
                }
            }
            "machineY" => {
                if let Some(value) = parse_f32(value) {
                    metadata.printer.display_height_mm = value;
                }
            }
            "machineZ" => {
                if let Some(value) = parse_f32(value) {
                    metadata.printer.machine_z_mm = value;
                }
            }
            "projectType" => {
                metadata.printer.mirror_x = value.eq_ignore_ascii_case("LCD_mirror");
            }
            "normalExposureTime" => {
                if let Some(value) = parse_f32(value) {
                    metadata.exposure.exposure_time = value;
                }
            }
            "bottomLayExposureTime" | "bottomLayerExposureTime" => {
                if let Some(value) = parse_f32(value) {
                    metadata.exposure.bottom_exposure_time = value;
                }
            }
            "normalDropSpeed" => {
                if let Some(value) = parse_f32(value) {
                    metadata.motion.retract_speed = value;
                }
            }
            "normalLayerLiftSpeed" => {
                if let Some(value) = parse_f32(value) {
                    metadata.motion.lift_speed = value;
                }
            }
            "normalLayerLiftHeight" => {
                if let Some(value) = parse_f32(value) {
                    metadata.motion.lift_height = value;
                }
            }
            "bottomLayCount" | "bottomLayerCount" => {
                if let Some(value) = parse_u32(value) {
                    metadata.slicing.bottom_layer_count = value;
                }
            }
            "mirror" => {
                if let Some(value) = parse_u32(value) {
                    metadata.printer.mirror_x = value != 0;
                }
            }
            "totalLayer" => {
                if let Some(value) = parse_u32(value) {
                    metadata.slicing.layer_count = value;
                }
            }
            "bottomLayerLiftHeight" => {
                if let Some(value) = parse_f32(value) {
                    metadata.motion.bottom_lift_height = value;
                }
            }
            "bottomLayerLiftSpeed" => {
                if let Some(value) = parse_f32(value) {
                    metadata.motion.bottom_lift_speed = value;
                }
            }
            "bottomLightOffTime" => {
                if let Some(value) = parse_f32(value) {
                    metadata.exposure.bottom_light_off_delay = value;
                }
            }
            "lightOffTime" => {
                if let Some(value) = parse_f32(value) {
                    metadata.exposure.light_off_delay = value;
                }
            }
            "bottomPWMLight" => {
                if let Some(value) = parse_u16(value) {
                    metadata.exposure.bottom_light_pwm = value;
                }
            }
            "PWMLight" => {
                if let Some(value) = parse_u16(value) {
                    metadata.exposure.light_pwm = value;
                }
            }
            "antiAliasLevel" => {
                if let Some(value) = parse_u32(value) {
                    metadata.slicing.anti_alias_level = value;
                }
            }
            _ => {}
        }
    }

    metadata
}

pub(super) fn build_gcode_metadata(metadata: &FileMetadata, total_layers: u32) -> String {
    format!(
        ";(****Build and Slicing Parameters****)\n\
         ;fileName: generated.zip\n\
         ;machineType: {}\n\
         ;estimatedPrintTime: {}\n\
         ;volume: {}\n\
         ;weight: {}\n\
         ;price: {}\n\
         ;layerHeight: {}\n\
         ;resolutionX: {}\n\
         ;resolutionY: {}\n\
         ;machineX: {}\n\
         ;machineY: {}\n\
         ;machineZ: {}\n\
         ;normalExposureTime: {}\n\
         ;bottomLayerExposureTime: {}\n\
         ;normalDropSpeed: {}\n\
         ;normalLayerLiftSpeed: {}\n\
         ;normalLayerLiftHeight: {}\n\
         ;bottomLayerCount: {}\n\
         ;totalLayer: {}\n\
         ;bottomLayerLiftHeight: {}\n\
         ;bottomLayerLiftSpeed: {}\n\
         ;bottomLightOffTime: {}\n\
         ;lightOffTime: {}\n\
         ;bottomPWMLight: {}\n\
         ;PWMLight: {}\n\
         ;antiAliasLevel: {}\n\
         ;mirror: {}\n",
        effective_machine_name(metadata),
        metadata.stats.print_time_sec,
        metadata.stats.volume_ml,
        metadata.stats.weight_g,
        metadata.stats.cost,
        metadata.slicing.layer_height_mm,
        metadata.printer.resolution_x,
        metadata.printer.resolution_y,
        metadata.printer.display_width_mm,
        metadata.printer.display_height_mm,
        metadata.printer.machine_z_mm,
        metadata.exposure.exposure_time,
        metadata.exposure.bottom_exposure_time,
        metadata.motion.retract_speed,
        metadata.motion.lift_speed,
        metadata.motion.lift_height,
        metadata.slicing.bottom_layer_count,
        total_layers,
        metadata.motion.bottom_lift_height,
        metadata.motion.bottom_lift_speed,
        metadata.exposure.bottom_light_off_delay,
        metadata.exposure.light_off_delay,
        metadata.exposure.bottom_light_pwm,
        metadata.exposure.light_pwm,
        metadata.slicing.anti_alias_level.max(1),
        u8::from(metadata.printer.mirror_x),
    )
}

fn effective_machine_name(metadata: &FileMetadata) -> &str {
    let name = metadata.printer.machine_name.trim();
    if name.is_empty() {
        "Default"
    } else {
        name
    }
}
