use std::convert::identity;

pub(crate) struct Top(Vec<TopProcess>);

impl Top {
    pub(crate) fn processes(&self) -> &[TopProcess] {
        &self.0
    }

    pub(crate) fn into_processes(self) -> Vec<TopProcess> {
        self.0
    }
}

pub(crate) struct TopProcess {
    pub(crate) user: String,
    pub(crate) pid: i32,
    pub(crate) ppid: i32,
    pub(crate) cpu: f64,
    pub(crate) elapsed: i64,
    pub(crate) tty: String,
    pub(crate) time: i64,
    pub(crate) command: String,
}

impl From<bollard::plugin::ContainerTopResponse> for Top {
    fn from(value: bollard::plugin::ContainerTopResponse) -> Self {
        Self(
            value
                .processes
                .unwrap_or_default()
                .into_iter()
                .map(|mut fields| {
                    let command = fields.pop().unwrap_or_default();
                    let time = fields.pop().unwrap_or_default();
                    let tty = fields.pop().unwrap_or_default();
                    let elapsed = fields.pop().unwrap_or_default();
                    let cpu = fields.pop().unwrap_or_default();
                    let ppid = fields.pop().unwrap_or_default();
                    let pid = fields.pop().unwrap_or_default();
                    let user = fields.pop().unwrap_or_default();

                    TopProcess {
                        user,
                        pid: pid.parse().unwrap_or_default(),
                        ppid: ppid.parse().unwrap_or_default(),
                        cpu: cpu.parse().unwrap_or_default(),
                        elapsed: parse_posix_time(&elapsed).unwrap_or_default(),
                        tty,
                        time: parse_posix_time(&time).unwrap_or_default(),
                        command,
                    }
                })
                .collect(),
        )
    }
}

impl From<podman_api::models::ContainerTopOkBody> for Top {
    fn from(value: podman_api::models::ContainerTopOkBody) -> Self {
        Self(
            value
                .processes
                .into_iter()
                .map(convert_podman_process)
                .collect(),
        )
    }
}

impl From<podman_api::models::PodTopOkBody> for Top {
    fn from(value: podman_api::models::PodTopOkBody) -> Self {
        Self(
            value
                .processes
                .into_iter()
                .map(convert_podman_process)
                .collect(),
        )
    }
}

fn convert_podman_process(mut fields: Vec<String>) -> TopProcess {
    let command = fields.pop().unwrap_or_default();
    let time = fields.pop().unwrap_or_default();
    let tty = fields.pop().unwrap_or_default();
    let elapsed = fields.pop().unwrap_or_default();
    let cpu = fields.pop().unwrap_or_default();
    let ppid = fields.pop().unwrap_or_default();
    let pid = fields.pop().unwrap_or_default();
    let user = fields.pop().unwrap_or_default();

    TopProcess {
        user,
        pid: pid.parse().unwrap_or_default(),
        ppid: ppid.parse().unwrap_or_default(),
        cpu: cpu.parse().unwrap_or_default(),
        elapsed: parse_podman_time(&elapsed).unwrap_or_default(),
        tty,
        time: parse_podman_time(&time).unwrap_or_default(),
        command,
    }
}

fn parse_podman_time(input: &str) -> anyhow::Result<i64> {
    match input.split_once("ms") {
        Some((millis, _)) => millis
            .parse::<f64>()
            .map_err(anyhow::Error::from)
            .map(f64::round)
            .map(|millis| millis as i64),
        None => {
            let secs = input
                .split_once('s')
                .ok_or_else(|| anyhow::anyhow!("seconds part missing"))?
                .0;

            match secs.split_once('m') {
                Some((mins, secs)) => {
                    let millis =
                        (secs.parse::<f64>().map_err(anyhow::Error::from)? * 1_000.0) as i64;

                    match mins.split_once('h') {
                        Some((hours, mins)) => {
                            let hours = hours.parse::<i64>().map_err(anyhow::Error::from)?;
                            let mins = mins.parse::<i64>().map_err(anyhow::Error::from)?;

                            Ok(hours * 3_600_000 + mins * 60_000 + millis)
                        }
                        None => {
                            let mins = mins.parse::<i64>().map_err(anyhow::Error::from)?;

                            Ok(mins * 60_000 + millis)
                        }
                    }
                }

                None => {
                    let secs = secs.parse::<f64>().map_err(anyhow::Error::from)?;
                    Ok((secs * 1_000.0) as i64)
                }
            }
        }
    }
}

// TODO: Rather pass Enum
fn parse_posix_time(input: &str) -> anyhow::Result<i64> {
    let (days, input) = input
        .split_once('-')
        .map(|(days, rest)| days.parse::<i64>().map(|days| (days, rest)))
        .transpose()
        .map_err(anyhow::Error::from)?
        .unwrap_or((0, input));

    let mut parts = input.rsplit(':');

    let secs = parse_next_posix_time_part(&mut parts)
        .ok_or_else(|| anyhow::anyhow!("missing secs"))
        .flatten()?;
    let mins = parse_next_posix_time_part(&mut parts)
        .ok_or_else(|| anyhow::anyhow!("missing mins"))
        .flatten()?;
    let hours = parse_next_posix_time_part(&mut parts).map_or(Ok(0), identity)?;

    Ok((days * 86_400 + hours * 3_600 + mins * 60 + secs) * 1000)
}

fn parse_next_posix_time_part<'a>(
    parts: &mut impl Iterator<Item = &'a str>,
) -> Option<anyhow::Result<i64>> {
    parts
        .next()
        .map(|part| part.parse().map_err(anyhow::Error::from))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_time() {
        assert_eq!(parse_podman_time("0s").unwrap(), 0);
        assert_eq!(parse_podman_time("4m29.407166273s").unwrap(), 269_407);
        assert_eq!(
            parse_podman_time("27h24m22.259082091s").unwrap(),
            98_662_259
        );
        assert_eq!(parse_posix_time("00:11").unwrap(), 11_000);
        assert_eq!(parse_posix_time("29:09:34").unwrap(), 104_974_000);
    }
}
