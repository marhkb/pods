use serde_json::Value;

#[derive(Debug, Default)]
pub(crate) struct Info {
    pub(crate) version: String,
    pub(crate) api_version: Option<String>,
    pub(crate) go_version: Option<String>,
    pub(crate) git_commit: Option<String>,
    pub(crate) built: Option<i64>,
    pub(crate) os_arch: Option<String>,
    pub(crate) cpus: i64,
}

impl Info {
    pub fn docker(value: Value) -> Self {
        todo!();
    }

    pub fn podman(value: Value) -> Self {
        Info {
            version: value
                .pointer("/version/Version")
                .and_then(Value::as_str)
                .unwrap()
                .to_owned(),
            cpus: value.pointer("/host/cpus").and_then(Value::as_i64).unwrap(),
            ..Default::default()
        }
    }
}
