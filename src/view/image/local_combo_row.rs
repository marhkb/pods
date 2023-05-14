use std::borrow::Borrow;

use adw::prelude::ComboRowExt;
use adw::subclass::prelude::*;
use glib::clone;
use glib::closure;
use glib::closure_local;
use glib::Properties;
use gtk::glib;
use gtk::pango;
use gtk::prelude::*;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::LocalComboRow)]
    #[template(string = r#"
    <interface>
      <template class="PdsImageLocalComboRow" parent="AdwComboRow">
        <property name="title" translatable="yes">Local Image</property>
        <property name="use-subtitle">True</property>
      </template>
    </interface>
    "#)]
    pub(crate) struct LocalComboRow {
        #[property(get, set = Self::set_client, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LocalComboRow {
        const NAME: &'static str = "PdsImageLocalComboRow";
        type Type = super::LocalComboRow;
        type ParentType = adw::ComboRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LocalComboRow {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            let image_tag_expr = model::Image::this_expression("repo-tags")
                .chain_closure::<String>(closure!(
                    |image: model::Image, repo_tags: model::RepoTagList| {
                        repo_tags
                            .get(0)
                            .as_ref()
                            .map(model::RepoTag::full)
                            .unwrap_or_else(|| utils::format_id(&image.id()))
                    }
                ));
            obj.set_expression(Some(&image_tag_expr));

            let list_factory = gtk::SignalListItemFactory::new();
            list_factory.connect_bind(
                clone!(@weak obj, @to-owned image_tag_expr => move |_, list_item| {
                    let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();

                    if let Some(item) = list_item.item() {
                        let label = gtk::Label::builder()
                            .xalign(0.0)
                            .max_width_chars(48)
                            .wrap(true)
                            .wrap_mode(pango::WrapMode::WordChar).build();
                        image_tag_expr.bind(&label, "label", Some(&item));

                        let selected_icon = gtk::Image::builder()
                            .icon_name("object-select-symbolic")
                            .build();

                        adw::ComboRow::this_expression("selected-item")
                            .chain_closure::<bool>(closure_local!(
                                |_: adw::ComboRow, selected: Option<&glib::Object>| {
                                    selected == Some(&item)
                                }
                            ))
                            .bind(&selected_icon, "visible", Some(&obj));

                        let box_ = gtk::Box::builder().spacing(3).build();
                        box_.append(&label);
                        box_.append(&selected_icon);

                        list_item.set_child(Some(&box_));
                    }
                }),
            );
            list_factory.connect_unbind(|_, list_item| {
                list_item
                    .downcast_ref::<gtk::ListItem>()
                    .unwrap()
                    .set_child(gtk::Widget::NONE);
            });
            obj.set_list_factory(Some(&list_factory));
        }
    }

    impl WidgetImpl for LocalComboRow {}
    impl ListBoxRowImpl for LocalComboRow {}
    impl PreferencesRowImpl for LocalComboRow {}
    impl ActionRowImpl for LocalComboRow {}
    impl ComboRowImpl for LocalComboRow {}

    impl LocalComboRow {
        pub(super) fn set_client(&self, value: Option<&model::Client>) {
            let obj = &*self.obj();
            if obj.client().as_ref() == value {
                return;
            }

            if let Some(client) = value {
                let model = gtk::SortListModel::new(
                    Some(gtk::FilterListModel::new(
                        Some(client.image_list()),
                        Some(gtk::CustomFilter::new(|obj| {
                            obj.downcast_ref::<model::Image>()
                                .unwrap()
                                .repo_tags()
                                .n_items()
                                > 0
                        })),
                    )),
                    Some(gtk::StringSorter::new(obj.expression())),
                );

                obj.set_model(Some(&model));
                obj.set_sensitive(true);
            }

            self.client.set(value);
        }
    }
}

glib::wrapper! {
    pub(crate) struct LocalComboRow(ObjectSubclass<imp::LocalComboRow>)
        @extends adw::ActionRow, adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

impl Borrow<adw::ComboRow> for LocalComboRow {
    fn borrow(&self) -> &adw::ComboRow {
        self.upcast_ref()
    }
}

impl AsRef<adw::ComboRow> for LocalComboRow {
    fn as_ref(&self) -> &adw::ComboRow {
        self.upcast_ref()
    }
}

impl From<LocalComboRow> for adw::ComboRow {
    fn from(local_combo_row: LocalComboRow) -> Self {
        local_combo_row.upcast()
    }
}

unsafe impl glib::IsA<adw::ComboRow> for LocalComboRow {}
