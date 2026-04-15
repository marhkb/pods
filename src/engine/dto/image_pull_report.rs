pub(crate) enum ImagePullReport {
    Streaming { line: String },
    Error { message: String },
    Finished { image_id: String },
}
