pub mod size {
    const MIN_VALUE_TO_ROUND: f64 = 10.0;

    pub fn formatted_fragments(bytes: u64) -> (f64, &'static str) {
        const BASE_UNIT: u64 = 1024;

        macro_rules! match_units {
            ($($pow:expr => $unit_name:expr),+) => {{
                $(
                let threshold = BASE_UNIT.pow($pow);

                if bytes >= threshold {
                    return (bytes as f64 / threshold as f64, $unit_name);
                }
                )+

                (bytes as f64, "B")
            }};
        }

        match_units!(
            // Terabytes
            4 => "T",
            // Gigabytes
            3 => "G",
            // Megabytes
            2 => "M",
            // Kilobytes
            1 => "K",
            // Bytes
            0 => "B"
        )
    }

    macro_rules! gen_format {
        ($bytes:expr, $rounded_format:expr => $non_rounded_format:expr, $unit_format:expr) => {{
            let (value, unit) = formatted_fragments($bytes);

            if value >= MIN_VALUE_TO_ROUND || value < 0.01 {
                format!(
                    concat!($rounded_format, $unit_format),
                    value.round() as u64,
                    unit
                )
            } else {
                format!(concat!($non_rounded_format, $unit_format), value, unit)
            }
        }};
    }

    pub fn formatted(bytes: u64) -> String {
        gen_format!(bytes, "{}" => "{:.02}", " {}")
    }

    pub fn formatted_extra_compact(bytes: u64) -> String {
        gen_format!(bytes, "{}" => "{:.01}", "{}")
    }

    pub fn formatted_compact(bytes: u64) -> String {
        gen_format!(bytes, "{}" => "{:.02}", "{}")
    }
}
