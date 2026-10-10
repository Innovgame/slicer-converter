use crate::common::{FileMetadata, FileMetadataBuilder};

/// Parsed subset of SL1 `config.ini`.
///
/// SL1 archives carry print timing, layer counts, and material statistics in
/// `config.ini`. Unknown keys are intentionally ignored so files from newer
/// PrusaSlicer versions remain readable.
///
/// SL1 ZIP 内 `config.ini` 的已知字段子集。这里保存打印时序、层数与材料统计；
/// 未知键会被忽略，以兼容新版 PrusaSlicer 写出的文件。
pub struct Sl1Config {
    pub exp_time: f32,
    pub exp_time_first: f32,
    pub layer_height: f32,
    pub num_fade: u16,
    pub num_fast: u32,
    pub num_slow: u16,
    pub print_time: f32,
    pub printer_model: String,
    pub used_material: f32,
    pub material_name: String,
    pub job_dir: String,
}

impl Sl1Config {
    /// Parse config.ini content into Sl1Config.
    ///
    /// Missing or malformed values fall back to conservative defaults matching
    /// the importer behavior used by the rest of this crate.
    ///
    /// 解析 `config.ini` 文本内容，将键值对映射到 [`Sl1Config`] 字段。缺失或格式错误的
    /// 值会回退到保守默认值，以保持 importer 可继续读取。
    pub fn parse(ini_text: &str) -> Self {
        let mut config = Self {
            exp_time: 2.0,
            exp_time_first: 30.0,
            layer_height: 0.05,
            num_fade: 0,
            num_fast: 0,
            num_slow: 0,
            print_time: 0.0,
            printer_model: String::new(),
            used_material: 0.0,
            material_name: String::new(),
            job_dir: String::new(),
        };

        for line in ini_text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                match key {
                    "exp_time" => config.exp_time = value.parse().unwrap_or(config.exp_time),
                    "exp_time_first" => {
                        config.exp_time_first = value.parse().unwrap_or(config.exp_time_first)
                    }
                    "layer_height" => {
                        config.layer_height = value.parse().unwrap_or(config.layer_height)
                    }
                    "num_fade" => config.num_fade = value.parse().unwrap_or(config.num_fade),
                    "num_fast" => config.num_fast = value.parse().unwrap_or(config.num_fast),
                    "num_slow" => config.num_slow = value.parse().unwrap_or(config.num_slow),
                    "print_time" => config.print_time = value.parse().unwrap_or(config.print_time),
                    "printer_model" => config.printer_model = value.to_string(),
                    "used_material" => {
                        config.used_material = value.parse().unwrap_or(config.used_material)
                    }
                    "material_name" => config.material_name = value.to_string(),
                    "job_dir" => config.job_dir = value.to_string(),
                    _ => {} // ignore unknown keys
                }
            }
        }

        config
    }

    /// Convert the print-facing config fields to shared [`FileMetadata`].
    ///
    /// This is intentionally partial: screen size, resolution, and mirror flags
    /// are stored in `prusaslicer.ini` and should be merged with
    /// [`Sl1PrinterConfig::apply_to`] when available.
    ///
    /// 将 print-facing config 字段转换为共享 [`FileMetadata`]。该转换有意保持 partial：
    /// 屏幕尺寸、分辨率与镜像字段来自 `prusaslicer.ini`，可在可用时通过
    /// [`Sl1PrinterConfig::apply_to`] 合并。
    pub fn to_metadata(&self) -> FileMetadata {
        let layer_count = self.num_fast + self.num_slow as u32;
        FileMetadataBuilder::new()
            .layer_height_mm(self.layer_height)
            .layer_count(layer_count)
            .bottom_layer_count(self.num_fade as u32)
            .transition_layer_count(self.num_slow as u32)
            .exposure(self.exp_time, self.exp_time_first)
            .statistics(self.print_time as u32, self.used_material, 0.0, 0.0)
            .machine_name(self.printer_model.clone())
            .build_unchecked()
    }
}

/// Parsed subset of SL1 `prusaslicer.ini` printer settings.
///
/// This companion file supplies machine/display geometry that `config.ini` does
/// not reliably contain.
///
/// SL1 `prusaslicer.ini` 中 printer settings 的已知字段子集。该 companion file 提供
/// `config.ini` 中不稳定或不存在的机器/display geometry。
pub struct Sl1PrinterConfig {
    pub display_width: f32,
    pub display_height: f32,
    pub display_pixels_x: u32,
    pub display_pixels_y: u32,
    pub max_print_height: f32,
    pub display_mirror_x: bool,
    pub display_mirror_y: bool,
}

impl Sl1PrinterConfig {
    /// Parse printer geometry from `prusaslicer.ini` text.
    ///
    /// 解析 `prusaslicer.ini` 文本中的 printer geometry。
    pub fn parse(ini_text: &str) -> Self {
        let mut config = Self {
            display_width: 120.0,
            display_height: 68.0,
            display_pixels_x: 2560,
            display_pixels_y: 1440,
            max_print_height: 150.0,
            display_mirror_x: true,
            display_mirror_y: false,
        };

        for line in ini_text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                match key {
                    "display_width" => {
                        config.display_width = value.parse().unwrap_or(config.display_width)
                    }
                    "display_height" => {
                        config.display_height = value.parse().unwrap_or(config.display_height)
                    }
                    "display_pixels_x" => {
                        config.display_pixels_x = value.parse().unwrap_or(config.display_pixels_x)
                    }
                    "display_pixels_y" => {
                        config.display_pixels_y = value.parse().unwrap_or(config.display_pixels_y)
                    }
                    "max_print_height" => {
                        config.max_print_height = value.parse().unwrap_or(config.max_print_height)
                    }
                    "display_mirror_x" => config.display_mirror_x = value == "1",
                    "display_mirror_y" => config.display_mirror_y = value == "1",
                    _ => {}
                }
            }
        }

        config
    }

    /// Merge printer config into existing metadata.
    ///
    /// This mutates only geometry fields so print timing parsed from
    /// `config.ini` remains intact.
    ///
    /// 将 printer config 合并进已有 metadata。该方法只修改 geometry 字段，保留从
    /// `config.ini` 解析得到的打印时序。
    pub fn apply_to(&self, metadata: &mut FileMetadata) {
        metadata.printer.resolution_x = self.display_pixels_x;
        metadata.printer.resolution_y = self.display_pixels_y;
        metadata.printer.display_width_mm = self.display_width;
        metadata.printer.display_height_mm = self.display_height;
        metadata.printer.machine_z_mm = self.max_print_height;
    }
}

/// Generate SL1 `config.ini` text from shared [`FileMetadata`].
///
/// The generated file contains the keys needed by this crate's importer and by
/// common SL1 readers for print timing and material statistics.
///
/// 根据共享 [`FileMetadata`] 生成 SL1 `config.ini` 文本，包含本 crate importer 与常见
/// SL1 reader 所需的打印时序和材料统计字段。
pub fn build_config_ini(metadata: &FileMetadata) -> String {
    let transition_layer_count =
        metadata.slicing.transition_layer_count.min(u16::MAX as u32) as u16;
    let normal_layer_count = metadata
        .slicing
        .layer_count
        .saturating_sub(transition_layer_count as u32);

    format!(
        "action = print\n\
         job_dir = output\n\
         exp_time = {}\n\
         exp_time_first = {}\n\
         layer_height = {}\n\
         num_fade = {}\n\
         num_fast = {}\n\
         num_slow = {}\n\
         print_time = {}\n\
         printer_model = {}\n\
         used_material = {}\n",
        metadata.exposure.exposure_time,
        metadata.exposure.bottom_exposure_time,
        metadata.slicing.layer_height_mm,
        metadata.slicing.bottom_layer_count,
        normal_layer_count,
        transition_layer_count,
        metadata.stats.print_time_sec,
        if metadata.printer.machine_name.is_empty() {
            "SL1"
        } else {
            &metadata.printer.machine_name
        },
        metadata.stats.volume_ml,
    )
}

/// Generate SL1 `prusaslicer.ini` printer metadata from shared [`FileMetadata`].
///
/// `config.ini` carries print settings; this companion file carries display
/// geometry and mirror/orientation fields.
///
/// 根据共享 [`FileMetadata`] 生成 SL1 `prusaslicer.ini` printer metadata。`config.ini`
/// 保存 print settings；这个 companion file 保存 display geometry 与 mirror/orientation 字段。
pub fn build_prusaslicer_ini(metadata: &FileMetadata) -> String {
    format!(
        "printer_technology = SLA\n\
         printer_model = {}\n\
         display_width = {}\n\
         display_height = {}\n\
         display_pixels_x = {}\n\
         display_pixels_y = {}\n\
         max_print_height = {}\n\
         display_mirror_x = 1\n\
         display_mirror_y = 0\n\
         display_orientation = portrait\n\
         exposure_time = {}\n\
         initial_exposure_time = {}\n\
         layer_height = {}\n",
        if metadata.printer.machine_name.is_empty() {
            "SL1"
        } else {
            &metadata.printer.machine_name
        },
        metadata.printer.display_width_mm,
        metadata.printer.display_height_mm,
        metadata.printer.resolution_x,
        metadata.printer.resolution_y,
        metadata.printer.machine_z_mm,
        metadata.exposure.exposure_time,
        metadata.exposure.bottom_exposure_time,
        metadata.slicing.layer_height_mm,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config_ini() {
        let ini = "action = print\n\
                    job_dir = test_model\n\
                    exp_time = 3.5\n\
                    exp_time_first = 45.0\n\
                    layer_height = 0.05\n\
                    num_fade = 8\n\
                    num_fast = 292\n\
                    num_slow = 0\n\
                    print_time = 7200\n\
                    printer_model = SL1\n\
                    used_material = 12.5\n";

        let config = Sl1Config::parse(ini);
        assert_eq!(config.exp_time, 3.5);
        assert_eq!(config.exp_time_first, 45.0);
        assert_eq!(config.num_fade, 8);
        assert_eq!(config.num_fast, 292);
        assert_eq!(config.job_dir, "test_model");
    }

    #[test]
    fn roundtrip_config_ini() {
        let metadata = FileMetadataBuilder::new()
            .exposure(2.5, 30.0)
            .layer_height_mm(0.05)
            .bottom_layer_count(6)
            .layer_count(300)
            .transition_layer_count(4)
            .statistics(3600, 10.0, 0.0, 0.0)
            .machine_name("SL1S")
            .build()
            .expect("valid metadata");

        let ini_text = build_config_ini(&metadata);
        let parsed = Sl1Config::parse(&ini_text);

        assert_eq!(parsed.exp_time, 2.5);
        assert_eq!(parsed.exp_time_first, 30.0);
        assert_eq!(parsed.num_fade, 6);
        assert_eq!(parsed.num_fast, 296);
        assert_eq!(parsed.num_slow, 4);
        assert_eq!(parsed.to_metadata().slicing.layer_count, 300);
    }
}
