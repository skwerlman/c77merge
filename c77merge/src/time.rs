use std::time::Duration;

pub fn format_elapsed(elapsed: Duration) -> String {
    let mut time = elapsed.as_nanos();
    let mut unit = "ns";
    if time >= 1_000 {
        time /= 1_000;
        unit = "μs";

        if time >= 1_000 {
            time /= 1_000;
            unit = "ms";

            if time >= 1_000 {
                time /= 1_000;
                unit = "s";

                if time >= 60 {
                    time /= 60;
                    unit = "m";

                    if time >= 60 {
                        time /= 60;
                        unit = "h";
                    }
                }
            }
        }
    }
    format!("{} {}", time, unit)
}
