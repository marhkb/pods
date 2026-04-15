pub(crate) enum ImageBuildReport {
    Streaming { line: String },
    Error { message: String },
    Finished { image_id: String },
}
