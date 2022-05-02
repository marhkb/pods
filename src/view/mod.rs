mod check_service_page;
mod circular_progress_bar;
mod connection_lost_page;
mod container;
mod image;
mod info_dialog;
mod leaflet_overlay;
mod property_row;
mod property_widget_row;
mod start_service_page;
mod text_search_entry;
mod theme_selector;

use gtk::prelude::Cast;
use gtk::traits::WidgetExt;

pub(crate) use self::check_service_page::CheckServicePage;
pub(crate) use self::circular_progress_bar::CircularProgressBar;
pub(crate) use self::connection_lost_page::ConnectionLostPage;
pub(crate) use self::container::menu as containers_menu;
pub(crate) use self::container::ContainerCreationPage;
pub(crate) use self::container::ContainerDetailsPanel;
pub(crate) use self::container::ContainerLogsPanel;
pub(crate) use self::container::ContainerPage;
pub(crate) use self::container::ContainerRenameDialog;
pub(crate) use self::container::ContainerRow;
pub(crate) use self::container::ContainersGroup;
pub(crate) use self::container::ContainersPanel;
pub(crate) use self::container::EnvVarRow;
pub(crate) use self::container::PortMappingRow;
pub(crate) use self::container::VolumeRow;
pub(crate) use self::image::menu as images_menu;
pub(crate) use self::image::ImageDetailsPage;
pub(crate) use self::image::ImagePullPage;
pub(crate) use self::image::ImageRow;
pub(crate) use self::image::ImageRowSimple;
pub(crate) use self::image::ImageSearchResponseRow;
pub(crate) use self::image::ImagesPanel;
pub(crate) use self::image::ImagesPrunePage;
pub(crate) use self::info_dialog::InfoDialog;
pub(crate) use self::leaflet_overlay::LeafletOverlay;
pub(crate) use self::property_row::PropertyRow;
pub(crate) use self::property_widget_row::PropertyWidgetRow;
pub(crate) use self::start_service_page::StartServicePage;
pub(crate) use self::text_search_entry::TextSearchEntry;
pub(crate) use self::theme_selector::ThemeSelector;
use crate::window::Window;

fn show_toast(widget: &gtk::Widget, title: &str) {
    widget
        .root()
        .unwrap()
        .downcast::<Window>()
        .unwrap()
        .show_toast(
            &adw::Toast::builder()
                .title(title)
                .timeout(3)
                .priority(adw::ToastPriority::High)
                .build(),
        );
}
