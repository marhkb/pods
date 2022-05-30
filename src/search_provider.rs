use futures::future;
use gettextrs::gettext;
use gtk::traits::GtkWindowExt;
use search_provider::SearchProviderImpl;
use serde::Deserialize;
use serde::Serialize;

use crate::api;
use crate::application::Application;
use crate::utils;
use crate::PODMAN;
use crate::RUNTIME;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ResultMeta {
    id: String,
    kind: ResultKind,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ResultInit {
    id: String,
    name: String,
    description: String,
    kind: ResultKind,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum ResultKind {
    Image,
    Container,
}

impl SearchProviderImpl for Application {
    fn activate_result(
        &self,
        identifier: search_provider::ResultID,
        _terms: &[String],
        timestamp: u32,
    ) {
        let window = self.main_window();
        window.present_with_time(timestamp);

        let search_result = serde_json::from_str::<ResultMeta>(&identifier).unwrap();
        match search_result.kind {
            ResultKind::Image => window.show_image_details(search_result.id),
            ResultKind::Container => window.show_container_details(search_result.id),
        }
    }

    fn initial_result_set(&self, terms: &[String]) -> Vec<search_provider::ResultID> {
        let (images, containers) = RUNTIME.block_on(future::join(
            PODMAN
                .images()
                .list(&api::ImageListOpts::builder().all(false).build()),
            PODMAN
                .containers()
                .list(&api::ContainerListOpts::builder().all(true).build()),
        ));

        let terms_lower = terms.iter().map(|s| s.to_lowercase()).collect::<Vec<_>>();

        images
            .map(|images| {
                images.into_iter().filter_map(|image| {
                    let id = image.id.unwrap();
                    let tag =
                        utils::format_option(image.repo_tags.and_then(|r| r.first().cloned()));

                    let id_lower = id.to_lowercase();
                    let tag_lower = tag.to_lowercase();

                    if terms_lower
                        .iter()
                        .any(|term| id_lower.contains(term) || tag_lower.contains(term))
                    {
                        Some(ResultInit {
                            id,
                            name: tag,
                            description: gettext!(
                                // Translators: "{}" is the placeholder for the amount of containers.
                                "{} containers",
                                image.containers.unwrap_or_default()
                            ),
                            kind: ResultKind::Image,
                        })
                    } else {
                        None
                    }
                })
            })
            .into_iter()
            .flatten()
            .chain(
                containers
                    .map(|containers| {
                        containers.into_iter().filter_map(|container| {
                            let id = container.id.unwrap();
                            let name = container.names.unwrap().pop().unwrap();
                            let image = container.image.unwrap();

                            let id_lower = id.to_lowercase();
                            let name_lower = name.to_lowercase();
                            let image_lower = image.to_lowercase();

                            if terms_lower.iter().any(|term| {
                                id_lower.contains(term)
                                    || name_lower.contains(term)
                                    || image_lower.contains(term)
                            }) {
                                Some(ResultInit {
                                    id,
                                    name,
                                    description: image,
                                    kind: ResultKind::Container,
                                })
                            } else {
                                None
                            }
                        })
                    })
                    .into_iter()
                    .flatten(),
            )
            .map(|init| serde_json::to_string(&init).unwrap())
            .collect()
    }

    fn subsearch_result_set(
        &self,
        previous_results: &[search_provider::ResultID],
        terms: &[String],
    ) -> Vec<search_provider::ResultID> {
        let terms_lower = terms.iter().map(|s| s.to_lowercase()).collect::<Vec<_>>();

        previous_results
            .iter()
            .map(|s| serde_json::from_str::<ResultInit>(s).unwrap())
            .filter(|init| {
                terms_lower.iter().any(|term| {
                    init.id.to_lowercase().contains(term)
                        || init.name.to_lowercase().contains(term)
                        || match init.kind {
                            ResultKind::Image => false,
                            ResultKind::Container => init.description.to_lowercase().contains(term),
                        }
                })
            })
            .map(|init| serde_json::to_string(&init).unwrap())
            .collect()
    }

    fn result_metas(
        &self,
        identifiers: &[search_provider::ResultID],
    ) -> Vec<search_provider::ResultMeta> {
        identifiers
            .iter()
            .map(|s| serde_json::from_str::<ResultInit>(s).unwrap())
            .map(|init| {
                search_provider::ResultMeta::builder(
                    serde_json::to_string(&ResultMeta {
                        id: init.id,
                        kind: init.kind,
                    })
                    .unwrap(),
                    &init.name,
                )
                .description(&init.description)
                .gicon(match init.kind {
                    ResultKind::Image => "image-x-generic-symbolic",
                    ResultKind::Container => "package-x-generic-symbolic",
                })
                .build()
            })
            .collect()
    }
}
