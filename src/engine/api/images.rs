use futures::StreamExt;
use futures::stream::BoxStream;
use http_body_util::Either;
use http_body_util::Full;

use crate::engine;

pub(crate) enum Images {
    Docker(bollard::Docker),
    Podman(podman_api::api::Images),
}

impl Images {
    pub(crate) fn get(&self, id: impl Into<String>) -> engine::api::Image {
        match self {
            Self::Docker(docker) => engine::api::Image::Docker {
                docker: docker.clone(),
                id: id.into(),
            },
            Self::Podman(images) => engine::api::Image::Podman(images.get(id.into())),
        }
    }
}

impl Images {
    pub(crate) fn build(
        &self,
        opts: engine::opts::ImageBuildOpts,
        context_dir: String,
    ) -> anyhow::Result<BoxStream<'_, anyhow::Result<engine::dto::ImageBuildReport>>> {
        match self {
            Self::Docker(docker) => {
                // TODO: as podman_api builds the body itself with blocking methods we have to do the same here
                let body = {
                    let mut builder = tar::Builder::new(Vec::new());

                    builder.append_dir_all(".", context_dir)?;
                    builder.finish()?;
                    builder.into_inner()?
                };

                Ok(async_stream::stream! {
                    let mut image_id = None;
                    // after Docker sends the "aux" elem there are 2 elems remaining before we can close the stream
                    let mut elem_countdown = 2;

                    let mut stream =
                        docker.build_image(opts, None, Some(Either::Left(Full::new(body.into()))));

                    while let Some(item) = stream.next().await {
                        match item {
                            Ok(item) => {
                                if let Some(aux) = item.aux {
                                    image_id = aux.id;
                                    continue;
                                }
                                if let Some(error_detail) = item.error_detail {
                                    yield Ok(engine::dto::ImageBuildReport::Error {
                                        message: error_detail.message.unwrap_or_default(),
                                    })
                                } else if let Some(line) = item.stream {
                                    yield Ok(engine::dto::ImageBuildReport::Streaming { line })
                                }

                                if image_id.is_some() {
                                    elem_countdown -= 1;
                                    if elem_countdown == 0 {
                                        break;
                                    }
                                }
                            }
                            Err(e) => yield Err(anyhow::Error::from(e)),
                        }
                    }

                    if let Some(image_id) = image_id {
                        yield Ok(engine::dto::ImageBuildReport::Finished { image_id })
                    }
                }
                .boxed())
            }
            Self::Podman(images) => images
                .build(&opts.into_podman(context_dir))
                .map_err(anyhow::Error::from)
                .map(|mut stream| {
                    async_stream::stream! {
                        let mut last_line = None;

                        while let Some(item) = stream.next().await {
                            match item {
                                Ok(item) => {
                                    last_line = Some(item.stream.clone());
                                    yield Ok(engine::dto::ImageBuildReport::Streaming {
                                        line: item.stream,
                                    })
                                }
                                Err(e) => yield Err(anyhow::Error::from(e)),
                            }
                        }

                        if let Some(mut last_line) = last_line {
                            last_line.truncate(last_line.trim_end().len());

                            yield Ok(engine::dto::ImageBuildReport::Finished {
                                image_id: last_line,
                            })
                        }
                    }
                    .boxed()
                }),
        }
    }

    pub(crate) async fn list(&self) -> anyhow::Result<Vec<engine::dto::ImageSummary>> {
        match self {
            Self::Docker(docker) => docker
                .list_images(Some(bollard::query_parameters::ListImagesOptions {
                    all: true,
                    ..Default::default()
                }))
                .await
                .map_err(anyhow::Error::from)
                .map(|summaries| summaries.into_iter().map(Into::into).collect()),
            Self::Podman(images) => images
                .list(&podman_api::opts::ImageListOpts::builder().all(true).build())
                .await
                .map_err(anyhow::Error::from)
                .map(|summaries| summaries.into_iter().map(Into::into).collect()),
        }
    }

    pub(crate) async fn prune(
        &self,
        opts: engine::opts::ImagesPruneOpts,
    ) -> anyhow::Result<String> {
        match self {
            Self::Docker(docker) => docker
                .prune_images(Some(opts))
                .await
                .map_err(anyhow::Error::from)
                .and_then(|response| {
                    serde_json::to_string_pretty(&response).map_err(anyhow::Error::from)
                }),
            Self::Podman(images) => images
                .prune(&opts.into())
                .await
                .map_err(anyhow::Error::from)
                .and_then(|response| {
                    serde_json::to_string_pretty(&response).map_err(anyhow::Error::from)
                }),
        }
    }

    pub(crate) fn pull(
        &self,
        opts: engine::opts::ImagePullOpts,
    ) -> BoxStream<'_, anyhow::Result<engine::dto::ImagePullReport>> {
        match self {
            Self::Docker(docker) => async_stream::stream! {
                let mut image_id = None;

                let mut stream = docker.create_image(Some(opts), None, None);

                while let Some(item) = stream.next().await {
                    match item {
                        Ok(item) => {
                            if let Some(error_detail) = item.error_detail {
                                yield Ok(engine::dto::ImagePullReport::Error {
                                    message: error_detail.message.unwrap_or_default(),
                                });
                            } else if let Some(line) = item.status {
                                yield Ok(engine::dto::ImagePullReport::Streaming {
                                    line: format!("{line}\n"),
                                });

                                if image_id.is_some() {
                                    break;
                                } else if line.starts_with("Digest: ") {
                                    image_id = Some(line.split_at(8).1.to_owned());
                                }
                            }
                        }
                        Err(e) => yield Err(anyhow::Error::from(e)),
                    }
                }

                if let Some(image_id) = image_id {
                    yield Ok(engine::dto::ImagePullReport::Finished { image_id })
                }
            }
            .boxed(),

            Self::Podman(images) => async_stream::stream! {
                let mut stream = images.pull(&opts.into());

                while let Some(item) = stream.next().await {
                    match item {
                        Ok(item) => {
                            if let Some(message) = item.error {
                                yield Ok(engine::dto::ImagePullReport::Error { message });
                            } else if let Some(line) = item.stream {
                                yield Ok(engine::dto::ImagePullReport::Streaming { line });
                            } else if let Some(image_id) = item.id {
                                yield Ok(engine::dto::ImagePullReport::Finished { image_id });
                                break;
                            }
                        }
                        Err(e) => yield Err(anyhow::Error::from(e)),
                    }
                }
            }
            .boxed(),
        }
    }

    pub(crate) async fn search(
        &self,
        term: String,
    ) -> anyhow::Result<Vec<engine::dto::ImageSearchResponseItem>> {
        match self {
            Self::Docker(docker) => docker
                .search_images(bollard::query_parameters::SearchImagesOptions {
                    term,
                    ..Default::default()
                })
                .await
                .map_err(anyhow::Error::from)
                .map(|items| items.into_iter().map(Into::into).collect()),
            Self::Podman(images) => images
                .search(
                    &podman_api::opts::ImageSearchOpts::builder()
                        .term(term)
                        .build(),
                )
                .await
                .map_err(anyhow::Error::from)
                .map(|items| items.into_iter().map(Into::into).collect()),
        }
    }
}
